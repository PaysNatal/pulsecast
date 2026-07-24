/*
 * pulsecast-glow-filter.c — 「心跳发光」滤镜
 *
 * 作用：对应用滤镜的源叠加一层随心率节拍“怦然”明灭的辉光（默认品红 #FF4D6D），
 *       让主播头像/摄像头在每次心跳时微微泛光，强化情绪张力。
 * 数据源：订阅 ws://<host>:<port> 取得实时 bpm，发光强度随节拍脉冲。
 */
#include "pulsecast-glow-filter.h"
#include "pc-ws.h"
#include "pc-ws-manager.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h> /* expf */

#define DEFAULT_HOST "localhost"
#define DEFAULT_PORT 4567
#define FLASH_MS 800 /* 高光闪烁窗口时长（ms），与 HR 源一致 */

/* OBS 着色器：5x5 邻域求亮，叠加 tint*强度。OBS 自动绑定 image / ViewProj。 */
static const char *glow_effect_src =
    "uniform float4x4 ViewProj;\n"
    "uniform texture2d image;\n"
    "uniform float2 uv_size;\n"
    "uniform float glow_strength;\n"
    "uniform float4 glow_color;\n"
    "sampler_state def { AddressU = Clamp; AddressV = Clamp; Filter = Linear; };\n"
    "struct vs_out { float4 pos : POSITION; float2 uv : TEXCOORD0; };\n"
    "vs_out vs(float4 pos : POSITION, float2 uv : TEXCOORD0) {\n"
    "    vs_out o; o.pos = mul(ViewProj, pos); o.uv = uv; return o;\n"
    "}\n"
    "float4 ps(vs_out i) : TARGET {\n"
    "    float4 base = image.Sample(def, i.uv);\n"
    "    float3 glow = float3(0.0, 0.0, 0.0);\n"
    "    float total = 0.0;\n"
    "    for (int x = -2; x <= 2; x++) {\n"
    "        for (int y = -2; y <= 2; y++) {\n"
    "            float2 off = float2(x, y) * uv_size * 3.0;\n"
    "            float4 s = image.Sample(def, i.uv + off);\n"
    "            glow += s.rgb * s.a;\n"
    "            total += 1.0;\n"
    "        }\n"
    "    }\n"
    "    glow /= total;\n"
    "    float lum = (glow.r + glow.g + glow.b) / 3.0;\n"
    "    float a = base.a + glow_strength * lum;\n"
    "    float3 col = base.rgb + glow_color.rgb * glow_strength * glow;\n"
    "    return float4(col, clamp(a, 0.0, 1.0));\n"
    "}\n"
    "technique Draw { pass { vertex_shader = vs(); pixel_shader = ps(); } }\n";

struct glow_data {
    obs_source_t *source;
    char *host;
    int port;
    pc_ws_mgr_t *ws;
    pthread_mutex_t lock;
    int bpm;
    float intensity;       /* 心率梯度 0~1，驱动发光强度 */
    gs_effect_t *effect;
    float base_strength;   /* 心跳节拍基线发光强度 */
    float flash_strength;  /* 高光闪烁额外峰值强度 */
    uint64_t flash_until;  /* 高光窗口结束时间戳(ms)，0=无 */
};

static void on_frame(void *user, const char *json, int bpm)
{
    struct glow_data *d = (struct glow_data *)user;

    /* ── 远程控制：手机控制中心发 flash -> 触发高光发光脉冲 ──
       发光滤镜与 HR 源各自订阅同一 ws 流，直接消费 flash 控制事件，
       无需跨源 IPC，实现“手机高光闪烁 -> 摄像头/头像辉光联动”。 */
    char kind[32];
    if (pc_ws_parse_control_kind(json, kind, sizeof(kind))) {
        if (strcmp(kind, "flash") == 0) {
            pthread_mutex_lock(&d->lock);
            d->flash_until = os_gettime_ns() / 1000000 + FLASH_MS;
            pthread_mutex_unlock(&d->lock);
            blog(LOG_INFO, "[pulsecast-glow] 远程高光闪烁");
        }
        return; /* 控制事件不走心率逻辑 */
    }

    pthread_mutex_lock(&d->lock);
    if (bpm > 0)
        d->bpm = bpm;
    float inten = pc_ws_parse_intensity(json);
    if (inten >= 0.0f)
        d->intensity = inten;
    pthread_mutex_unlock(&d->lock);
}

static const char *glow_get_name(void *priv)
{
    (void)priv;
    return "心跳发光 (PulseCast)";
}

static void *glow_create(obs_data_t *settings, obs_source_t *source)
{
    struct glow_data *d = calloc(1, sizeof(*d));
    if (!d)
        return NULL;
    d->source = source;
    d->bpm = 72;
    d->intensity = 0.0f;
    pthread_mutex_init(&d->lock, NULL);
    d->host = bstrdup(obs_data_get_string(settings, "host") ?: DEFAULT_HOST);
    d->port = (int)obs_data_get_int(settings, "port");
    if (d->port <= 0)
        d->port = DEFAULT_PORT;
    d->base_strength = (float)obs_data_get_double(settings, "strength");
    if (d->base_strength <= 0.0f)
        d->base_strength = 0.5f;
    d->flash_strength = (float)obs_data_get_double(settings, "flash_strength");
    if (d->flash_strength <= 0.0f)
        d->flash_strength = 1.6f;
    d->flash_until = 0;

    d->effect = gs_effect_create(glow_effect_src, NULL, NULL);
    if (!d->effect)
        blog(LOG_ERROR, "[pulsecast-glow] 着色器编译失败，发光滤镜不可用");
    d->ws = pc_ws_mgr_acquire(d->host, d->port, on_frame, d);
    return d;
}

static void glow_destroy(void *priv)
{
    struct glow_data *d = (struct glow_data *)priv;
    if (!d)
        return;
    if (d->ws)
        pc_ws_mgr_release(d->ws, on_frame, d);
    if (d->effect)
        gs_effect_destroy(d->effect);
    pthread_mutex_destroy(&d->lock);
    bfree(d->host);
    free(d);
}

static void glow_update(void *priv, obs_data_t *settings)
{
    struct glow_data *d = (struct glow_data *)priv;
    d->base_strength = (float)obs_data_get_double(settings, "strength");
    d->flash_strength = (float)obs_data_get_double(settings, "flash_strength");
    const char *nh = obs_data_get_string(settings, "host");
    int np = (int)obs_data_get_int(settings, "port");
    if (np <= 0)
        np = DEFAULT_PORT;
    pthread_mutex_lock(&d->lock);
    int changed = (strcmp(nh ? nh : DEFAULT_HOST, d->host) != 0) || (np != d->port);
    if (changed) {
        bfree(d->host);
        d->host = bstrdup(nh ? nh : DEFAULT_HOST);
        d->port = np;
    }
    pthread_mutex_unlock(&d->lock);
    if (changed && d->ws) {
        pc_ws_mgr_release(d->ws, on_frame, d);
        d->ws = pc_ws_mgr_acquire(d->host, d->port, on_frame, d);
    }
}

/* 当前发光强度与高光比例。
 * strength = 心跳节拍脉冲 + 高光闪烁额外峰值(0..~flash_strength)。
 * flash_ratio ∈ [0,1] 用于把辉光色向白色靠拢（高光白色脉冲，呼应浏览器叠层）。 */
static float glow_compute(struct glow_data *d, float *flash_ratio_out)
{
    pthread_mutex_lock(&d->lock);
    int bpm = d->bpm;
    float base = d->base_strength;
    float flash_peak = d->flash_strength;
    uint64_t flash_until = d->flash_until;
    float intensity = d->intensity;
    pthread_mutex_unlock(&d->lock);

    /* 梯度缩放：低心率时发光休眠，高心率时全开 */
    float scale = 0.15f + 0.85f * intensity; /* 0.15~1.0 */

    /* 心跳节拍脉冲（尖峰在每次心跳起始） */
    float beat;
    if (bpm <= 0) {
        beat = base * 0.5f * scale;
    } else {
        uint64_t now_ms = os_gettime_ns() / 1000000;
        float period_ms = 60000.0f / (float)bpm;
        float phase = (float)(now_ms % (uint64_t)period_ms) / period_ms;
        float pulse;
        if (phase < 0.06f)
            pulse = phase / 0.06f;
        else
            pulse = expf(-(phase - 0.06f) * 4.0f);
        beat = base * (0.4f + 0.6f * pulse) * scale;
    }

    /* 高光闪烁：FLASH_MS 内快速攻击(前15%) + 指数衰减 */
    float flash_ratio = 0.0f;
    if (flash_until) {
        uint64_t now_ms = os_gettime_ns() / 1000000;
        if (now_ms < flash_until) {
            float rem = (float)(flash_until - now_ms);
            float elapsed = (FLASH_MS - rem) / FLASH_MS; /* 0 -> 1 */
            float attack = elapsed < 0.15f ? (elapsed / 0.15f) : 1.0f;
            float decay = expf(-(1.0f - rem / FLASH_MS) * 3.0f);
            flash_ratio = attack * decay;
        }
    }
    if (flash_ratio_out)
        *flash_ratio_out = flash_ratio;

    return beat + flash_ratio * flash_peak;
}

static void glow_video_render(void *priv, gs_effect_t *effect)
{
    struct glow_data *d = (struct glow_data *)priv;
    (void)effect;
    if (!d->effect)
        return;
    if (!obs_source_process_filter_begin(d->source, GS_RGBA,
                                         OBS_NO_DIRECT_RENDERING))
        return;

    gs_epass_t *pass = gs_effect_get_epass_by_name(d->effect, "Draw");
    if (!pass) {
        obs_source_process_filter_end(d->source, d->effect, 0, 0);
        return;
    }

    float flash_ratio = 0.0f;
    float strength = glow_compute(d, &flash_ratio);
    gs_effect_set_float(gs_effect_get_param_by_name(d->effect, "glow_strength"),
                        strength);
    uint32_t cx = obs_source_get_base_width(d->source);
    uint32_t cy = obs_source_get_base_height(d->source);
    if (cx == 0) cx = 1920;
    if (cy == 0) cy = 1080;
    gs_effect_set_vec2(gs_effect_get_param_by_name(d->effect, "uv_size"),
                       (const struct vec2 *)&(struct vec2){1.0f / (float)cx,
                                                          1.0f / (float)cy});
    /* 基础色 #FF4D6D；高光时向白色靠拢，模拟“高光闪烁白色脉冲” */
    struct vec4 tint = {
        1.0f,
        0.302f + (1.0f - 0.302f) * flash_ratio,
        0.427f + (1.0f - 0.427f) * flash_ratio,
        1.0f,
    };
    gs_effect_set_vec4(gs_effect_get_param_by_name(d->effect, "glow_color"),
                       &tint);

    obs_source_process_filter_end(d->source, d->effect, 0, 0);
}

static obs_properties_t *glow_get_properties(void *priv)
{
    (void)priv;
    obs_properties_t *p = obs_properties_create();
    obs_properties_add_text(p, "host", "服务地址", OBS_TEXT_DEFAULT);
    obs_properties_add_int(p, "port", "端口", 1, 65535, 1);
    obs_properties_add_float(p, "strength", "心跳发光强度", 0.0, 2.0, 0.05);
    obs_properties_add_float(p, "flash_strength", "高光闪烁峰值", 0.0, 3.0, 0.05);
    return p;
}

static void glow_get_defaults(obs_data_t *s)
{
    obs_data_set_string(s, "host", DEFAULT_HOST);
    obs_data_set_int(s, "port", DEFAULT_PORT);
    obs_data_set_double(s, "strength", 0.5);
    obs_data_set_double(s, "flash_strength", 1.6);
}

struct obs_source_info pulsecast_glow_filter_info = {
    .id = "pulsecast_glow_filter",
    .type = OBS_SOURCE_TYPE_FILTER,
    .output_flags = OBS_SOURCE_VIDEO,
    .get_name = glow_get_name,
    .create = glow_create,
    .destroy = glow_destroy,
    .update = glow_update,
    .video_render = glow_video_render,
    .get_properties = glow_get_properties,
    .get_defaults = glow_get_defaults,
};
