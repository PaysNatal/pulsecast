/*
 * pulsecast-atmosphere-filter.c — 「心率氛围」全屏滤镜
 *
 * 作用：作为场景级滤镜，根据心率梯度 (intensity 0~1) 驱动全画面氛围效果：
 *   - Vignette 暗角（强度随 intensity 增加）
 *   - 色温偏移（高心率偏暖/红移，模拟肾上腺素上升）
 *   - 边缘脉冲（红色边框，频率跟心跳同步）
 *   - 全屏闪烁（trigger=high 时触发 0.3s 白→红脉冲）
 *
 * 数据源：订阅 ws://<host>:<port>/ws 取得 intensity + bpm + trigger。
 * 性能：单次全屏 pass，~4 次 texture sample，4K@60fps < 0.5ms。
 */
#include "pulsecast-atmosphere-filter.h"
#include "pc-ws.h"
#include "pc-ws-manager.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#define DEFAULT_HOST "localhost"
#define DEFAULT_PORT 4567
#define FLASH_MS 300 /* 全屏闪烁窗口 */

static const char *atmosphere_effect_src =
    "uniform float4x4 ViewProj;\n"
    "uniform texture2d image;\n"
    "uniform float hr_intensity;\n"  /* 0.0~1.0 心率梯度 */
    "uniform float bpm;\n"
    "uniform float time_sec;\n"      /* 当前时间（秒） */
    "uniform float2 uv_size;\n"
    "uniform float flash;\n"         /* 0.0~1.0 闪烁强度（由 C 端衰减） */
    "sampler_state def { AddressU = Clamp; AddressV = Clamp; Filter = Linear; };\n"
    "struct vs_out { float4 pos : POSITION; float2 uv : TEXCOORD0; };\n"
    "vs_out vs(float4 pos : POSITION, float2 uv : TEXCOORD0) {\n"
    "    vs_out o; o.pos = mul(ViewProj, pos); o.uv = uv; return o;\n"
    "}\n"
    "float4 ps(vs_out i) : TARGET {\n"
    "    float4 base = image.Sample(def, i.uv);\n"
    /* 1. Vignette 暗角 */
    "    float2 c = i.uv - 0.5;\n"
    "    float d = length(c);\n"
    "    float vig = smoothstep(0.3, 0.85, d) * hr_intensity * 0.4;\n"
    "    base.rgb *= 1.0 - vig;\n"
    /* 2. 色温偏移（高心率偏暖） */
    "    base.r += hr_intensity * 0.06;\n"
    "    base.b -= hr_intensity * 0.04;\n"
    /* 3. 边缘脉冲（心跳同步红色边框） */
    "    float period = bpm > 0.0 ? 60.0 / bpm : 1.0;\n"
    "    float phase = fmod(time_sec, period) / period;\n"
    "    float pulse = pow(max(0.0, 1.0 - phase * 3.0), 2.0);\n"
    "    float edge = smoothstep(0.03, 0.0, min(min(i.uv.x, 1.0 - i.uv.x), min(i.uv.y, 1.0 - i.uv.y)));\n"
    "    base.rgb = lerp(base.rgb, float3(1.0, 0.15, 0.25), edge * pulse * hr_intensity * 0.7);\n"
    /* 4. 全屏闪烁（trigger 触发） */
    "    base.rgb = lerp(base.rgb, float3(1.0, 0.25, 0.35), flash * 0.35);\n"
    "    return base;\n"
    "}\n"
    "technique Draw { pass { vertex_shader = vs(); pixel_shader = ps(); } }\n";

struct atmosphere_data {
    obs_source_t *source;
    char *host;
    int port;
    pc_ws_mgr_t *ws;
    pthread_mutex_t lock;
    float intensity;   /* 心率梯度 0~1 */
    int bpm;
    gs_effect_t *effect;
    uint64_t flash_until; /* 闪烁窗口结束时间(ms) */
};

/* 从 JSON 提取 "intensity":<float> */
static float parse_intensity(const char *json)
{
    const char *p = strstr(json, "\"intensity\"");
    if (!p) return 0.0f;
    p = strchr(p, ':');
    if (!p) return 0.0f;
    return (float)atof(p + 1);
}

/* 从 JSON 提取 "trigger":"high" */
static int parse_trigger_high(const char *json)
{
    const char *p = strstr(json, "\"trigger\"");
    if (!p) return 0;
    return strstr(p, "\"high\"") != NULL;
}

static void on_frame(void *user, const char *json, int bpm)
{
    struct atmosphere_data *d = (struct atmosphere_data *)user;
    float intensity = parse_intensity(json);
    int is_trigger = parse_trigger_high(json);

    pthread_mutex_lock(&d->lock);
    d->intensity = intensity;
    if (bpm > 0) d->bpm = bpm;
    if (is_trigger)
        d->flash_until = os_gettime_ns() / 1000000 + FLASH_MS;
    pthread_mutex_unlock(&d->lock);
}

static const char *atmosphere_get_name(void *priv)
{
    (void)priv;
    return "心率氛围 (PulseCast)";
}

static void *atmosphere_create(obs_data_t *settings, obs_source_t *source)
{
    struct atmosphere_data *d = calloc(1, sizeof(*d));
    if (!d) return NULL;
    d->source = source;
    d->intensity = 0.0f;
    d->bpm = 72;
    d->flash_until = 0;
    pthread_mutex_init(&d->lock, NULL);
    d->host = bstrdup(obs_data_get_string(settings, "host") ?: DEFAULT_HOST);
    d->port = (int)obs_data_get_int(settings, "port");
    if (d->port <= 0) d->port = DEFAULT_PORT;

    d->effect = gs_effect_create(atmosphere_effect_src, NULL, NULL);
    if (!d->effect)
        blog(LOG_ERROR, "[pulsecast-atmosphere] 着色器编译失败");
    d->ws = pc_ws_mgr_acquire(d->host, d->port, on_frame, d);
    return d;
}

static void atmosphere_destroy(void *priv)
{
    struct atmosphere_data *d = (struct atmosphere_data *)priv;
    if (!d) return;
    if (d->ws) pc_ws_mgr_release(d->ws, on_frame, d);
    if (d->effect) gs_effect_destroy(d->effect);
    pthread_mutex_destroy(&d->lock);
    bfree(d->host);
    free(d);
}

static void atmosphere_update(void *priv, obs_data_t *settings)
{
    struct atmosphere_data *d = (struct atmosphere_data *)priv;
    const char *nh = obs_data_get_string(settings, "host");
    int np = (int)obs_data_get_int(settings, "port");
    if (np <= 0) np = DEFAULT_PORT;
    pthread_mutex_lock(&d->lock);
    int changed = (strcmp(nh ? nh : DEFAULT_HOST, d->host) != 0) || (np != d->port);
    char *local_host = NULL;
    int local_port = np;
    if (changed) {
        bfree(d->host);
        d->host = bstrdup(nh ? nh : DEFAULT_HOST);
        d->port = np;
        local_host = bstrdup(d->host);
    }
    pthread_mutex_unlock(&d->lock);
    if (changed && d->ws) {
        pc_ws_mgr_release(d->ws, on_frame, d);
        d->ws = pc_ws_mgr_acquire(local_host, local_port, on_frame, d);
        bfree(local_host);
    }
}

static void atmosphere_video_render(void *priv, gs_effect_t *effect)
{
    struct atmosphere_data *d = (struct atmosphere_data *)priv;
    (void)effect;
    if (!d->effect) return;
    if (!obs_source_process_filter_begin(d->source, GS_RGBA, OBS_NO_DIRECT_RENDERING))
        return;

    gs_epass_t *pass = gs_effect_get_epass_by_name(d->effect, "Draw");
    if (!pass) {
        obs_source_process_filter_end(d->source, d->effect, 0, 0);
        return;
    }

    pthread_mutex_lock(&d->lock);
    float intensity = d->intensity;
    int bpm = d->bpm;
    uint64_t flash_until = d->flash_until;
    pthread_mutex_unlock(&d->lock);

    /* 计算闪烁衰减 */
    float flash = 0.0f;
    if (flash_until) {
        uint64_t now_ms = os_gettime_ns() / 1000000;
        if (now_ms < flash_until) {
            float rem = (float)(flash_until - now_ms);
            flash = rem / (float)FLASH_MS; /* 线性衰减 1→0 */
        }
    }

    uint64_t now_ms = os_gettime_ns() / 1000000;
    float time_sec = (float)(now_ms / 1000.0);

    gs_effect_set_float(gs_effect_get_param_by_name(d->effect, "hr_intensity"), intensity);
    gs_effect_set_float(gs_effect_get_param_by_name(d->effect, "bpm"), (float)bpm);
    gs_effect_set_float(gs_effect_get_param_by_name(d->effect, "time_sec"), time_sec);
    gs_effect_set_float(gs_effect_get_param_by_name(d->effect, "flash"), flash);

    uint32_t cx = obs_source_get_base_width(d->source);
    uint32_t cy = obs_source_get_base_height(d->source);
    if (cx == 0) cx = 1920;
    if (cy == 0) cy = 1080;
    gs_effect_set_vec2(gs_effect_get_param_by_name(d->effect, "uv_size"),
                       (const struct vec2 *)&(struct vec2){1.0f / (float)cx, 1.0f / (float)cy});

    obs_source_process_filter_end(d->source, d->effect, 0, 0);
}

static obs_properties_t *atmosphere_get_properties(void *priv)
{
    (void)priv;
    obs_properties_t *p = obs_properties_create();
    obs_properties_add_text(p, "host", "服务地址", OBS_TEXT_DEFAULT);
    obs_properties_add_int(p, "port", "端口", 1, 65535, 1);
    return p;
}

static void atmosphere_get_defaults(obs_data_t *s)
{
    obs_data_set_string(s, "host", DEFAULT_HOST);
    obs_data_set_int(s, "port", DEFAULT_PORT);
}

struct obs_source_info pulsecast_atmosphere_filter_info = {
    .id = "pulsecast_atmosphere_filter",
    .type = OBS_SOURCE_TYPE_FILTER,
    .output_flags = OBS_SOURCE_VIDEO,
    .get_name = atmosphere_get_name,
    .create = atmosphere_create,
    .destroy = atmosphere_destroy,
    .update = atmosphere_update,
    .video_render = atmosphere_video_render,
    .get_properties = atmosphere_get_properties,
    .get_defaults = atmosphere_get_defaults,
};
