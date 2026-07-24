/*
 * pulsecast-hr-source.c — 「怦然心率」文字源
 *
 * 功能：作为独立 OBS 来源，实时显示当前心率（BPM）。
 * 数据源：订阅本地 PulseCast 服务 ws://<host>:<port>（默认 localhost:4567），
 *         该服务由桌面主程序 / Rust 内嵌 server 提供，契约同 obs-overlay.html。
 * 渲染：内部创建一个 obs 内置 "text_ft2_source" 子源来绘制文字，
 *       心跳时字号随脉冲轻微放大（视觉“怦然”感）。
 */
#include "pulsecast-hr-source.h"
#include "pc-ws.h"

#include <obs-frontend-api.h> /* 阈值动作：进程内切换场景 */

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* 心率分区：0=正常 1=高 2=低；high/low 为 0 表示该方向禁用 */
static int zone_of(int bpm, int high, int low)
{
    if (bpm <= 0)
        return 0;
    if (high > 0 && bpm > high)
        return 1;
    if (low > 0 && bpm < low)
        return 2;
    return 0;
}

/* 切换到指定名称的场景（OBS 28+ 用 preview 接口，不影响录制）。
   旧版 OBS 可改用 obs_frontend_set_current_scene()。
   frontend API 必须在主线程调用，故用 obs_queue_task(OBS_TASK_UI) 派发，
   避免在 WS 后台线程直接调用导致未定义行为。 */
static void do_switch_scene(void *param)
{
    obs_source_t *scene = (obs_source_t *)param;
    if (scene)
        obs_frontend_set_current_preview_scene2(scene);
}
static void switch_scene(const char *name)
{
    if (!name || !*name)
        return;
    obs_source_t *scene = obs_get_source_by_name(name);
    if (scene) {
        obs_queue_task(OBS_TASK_UI, do_switch_scene, scene, true);
        obs_source_release(scene);
    }
}

#define DEFAULT_HOST "localhost"
#define DEFAULT_PORT 4567
#define EMPHASIS_COLOR 0xFFFF4D6D /* #FF4D6D, OBS 0xAARRGGBB */

struct hr_data {
    obs_source_t *source;
    obs_source_t *text; /* 内置文字子源 */
    char *host;
    int port;

    pc_ws_t *ws;
    pthread_mutex_t lock;
    int bpm;
    int bpm_dirty; /* 主线程据此刷新文字 */
    int last_frame_ms;

    /* 阈值动作配置 */
    int hr_high;
    int hr_low;
    int last_zone; /* 上一次分区，用于检测跨越边界 */
    char scene_high[256];
    char scene_low[256];

    /* 远程控制：高光闪烁窗口（ms 时间戳，0=无） */
    uint64_t flash_until;
};

/* WS 回调（在 WS 后台线程触发） */
static void on_frame(void *user, const char *json, int bpm)
{
    struct hr_data *d = (struct hr_data *)user;

    /* ── 远程控制事件（手机控制中心发出，type:"control"） ──
       浏览器叠层仅能显示横幅，真正切场景必须由本原生源在 OBS 进程内完成。 */
    char kind[32];
    if (pc_ws_parse_control_kind(json, kind, sizeof(kind))) {
        if (strcmp(kind, "switch_scene") == 0) {
            char scene[256];
            if (pc_ws_parse_str(json, "scene", scene, sizeof(scene)) && scene[0]) {
                switch_scene(scene);
                blog(LOG_INFO, "[pulsecast] 远程切场景 -> %s", scene);
            }
        } else if (strcmp(kind, "flash") == 0) {
            d->flash_until = os_gettime_ns() / 1000000 + 800; /* 800ms 高光 */
        } else if (strcmp(kind, "set_threshold") == 0) {
            int hi = pc_ws_parse_int_field(json, "high");
            int lo = pc_ws_parse_int_field(json, "low");
            if (hi > 0 && lo >= 0 && lo < hi) {
                pthread_mutex_lock(&d->lock);
                d->hr_high = hi;
                d->hr_low = lo;
                pthread_mutex_unlock(&d->lock);
                blog(LOG_INFO, "[pulsecast] 远程阈值 high=%d low=%d", hi, lo);
            }
        }
        return; /* 控制事件不再走心率渲染 */
    }

    /* ── 普通心率帧 ── */
    int switch_to = 0; /* 0=无 1=高 2=低 */
    pthread_mutex_lock(&d->lock);
    if (bpm > 0) {
        d->bpm = bpm;
        d->bpm_dirty = 1;
        int z = zone_of(bpm, d->hr_high, d->hr_low);
        if (z != d->last_zone) {
            d->last_zone = z;
            if (z == 1)
                switch_to = 1;
            else if (z == 2)
                switch_to = 2;
        }
    }
    pthread_mutex_unlock(&d->lock);

    /* 越界触发场景切换（OBS 进程内，零外部依赖，主线程安全派发）。 */
    if (switch_to == 1 && d->scene_high[0])
        switch_scene(d->scene_high);
    else if (switch_to == 2 && d->scene_low[0])
        switch_scene(d->scene_low);
}

static void update_text(struct hr_data *d)
{
    if (!d->text)
        return;
    char buf[32];
    snprintf(buf, sizeof(buf), "%d BPM", d->bpm);

    obs_data_t *settings = obs_source_get_settings(d->text);
    obs_data_set_string(settings, "text", buf);

    /* 字体：中文用系统默认，数字清晰 */
    obs_data_t *font = obs_data_get_obj(settings, "font");
    if (!font)
        font = obs_data_create();
    obs_data_set_string(font, "face", "Inter");
    obs_data_set_int(font, "size", 120);
    obs_data_set_int(font, "flags", 0);
    obs_data_set_obj(settings, "font", font);
    obs_data_release(font);

    obs_data_set_int(settings, "color", EMPHASIS_COLOR);
    obs_data_set_bool(settings, "drop_shadow", true);
    obs_data_set_bool(settings, "outline", true);
    obs_data_set_int(settings, "outline_size", 3);
    obs_data_set_int(settings, "outline_color", 0xFF000000);
    obs_data_set_int(settings, "align", 1);  /* center */
    obs_data_set_int(settings, "valign", 1); /* center */

    obs_source_update(d->text, settings);
    obs_data_release(settings);
}

static const char *hr_get_name(void *priv)
{
    (void)priv;
    return "怦然心率 (PulseCast)";
}

static void *hr_create(obs_data_t *settings, obs_source_t *source)
{
    struct hr_data *d = calloc(1, sizeof(*d));
    if (!d)
        return NULL;
    d->source = source;
    d->bpm = 0;
    d->bpm_dirty = 1;
    d->last_frame_ms = 0;
    d->flash_until = 0;
    d->hr_high = (int)obs_data_get_int(settings, "hr_high");
    d->hr_low = (int)obs_data_get_int(settings, "hr_low");
    d->last_zone = 0;
    {
        const char *sh = obs_data_get_string(settings, "scene_high");
        const char *sl = obs_data_get_string(settings, "scene_low");
        strncpy(d->scene_high, sh ? sh : "", sizeof(d->scene_high) - 1);
        d->scene_high[sizeof(d->scene_high) - 1] = '\0';
        strncpy(d->scene_low, sl ? sl : "", sizeof(d->scene_low) - 1);
        d->scene_low[sizeof(d->scene_low) - 1] = '\0';
    }
    pthread_mutex_init(&d->lock, NULL);

    d->host = bstrdup(obs_data_get_string(settings, "host") ?: DEFAULT_HOST);
    d->port = (int)obs_data_get_int(settings, "port");
    if (d->port <= 0)
        d->port = DEFAULT_PORT;

    /* 创建内置文字子源 */
    obs_data_t *ts = obs_data_create();
    obs_data_set_string(ts, "text", "-- BPM");
    obs_source_t *text =
        obs_source_create_private("text_ft2_source", "pulsecast_hr_text", ts);
    obs_data_release(ts);
    d->text = text;

    /* 连接本地服务 */
    d->ws = pc_ws_connect(d->host, d->port, "/ws", on_frame, d);

    return d;
}

static void hr_destroy(void *priv)
{
    struct hr_data *d = (struct hr_data *)priv;
    if (!d)
        return;
    if (d->ws)
        pc_ws_destroy(d->ws);
    if (d->text)
        obs_source_release(d->text);
    pthread_mutex_destroy(&d->lock);
    bfree(d->host);
    free(d);
}

static void hr_update(void *priv, obs_data_t *settings)
{
    struct hr_data *d = (struct hr_data *)priv;
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
    d->hr_high = (int)obs_data_get_int(settings, "hr_high");
    d->hr_low = (int)obs_data_get_int(settings, "hr_low");
    {
        const char *sh = obs_data_get_string(settings, "scene_high");
        const char *sl = obs_data_get_string(settings, "scene_low");
        strncpy(d->scene_high, sh ? sh : "", sizeof(d->scene_high) - 1);
        d->scene_high[sizeof(d->scene_high) - 1] = '\0';
        strncpy(d->scene_low, sl ? sl : "", sizeof(d->scene_low) - 1);
        d->scene_low[sizeof(d->scene_low) - 1] = '\0';
    }
    pthread_mutex_unlock(&d->lock);

    if (changed && d->ws) {
        /* 重连 */
        pc_ws_destroy(d->ws);
        d->ws = pc_ws_connect(d->host, d->port, "/ws", on_frame, d);
    }
    d->bpm_dirty = 1;
}

static void hr_video_render(void *priv, gs_effect_t *effect)
{
    struct hr_data *d = (struct hr_data *)priv;
    (void)effect;

    pthread_mutex_lock(&d->lock);
    if (d->bpm_dirty) {
        d->bpm_dirty = 0;
        int bpm = d->bpm;
        pthread_mutex_unlock(&d->lock);
        if (bpm > 0)
            update_text(d);
    } else {
        pthread_mutex_unlock(&d->lock);
    }

    if (d->text) {
        uint64_t now = os_gettime_ns() / 1000000;
        /* 心跳脉冲：字号随 60/bpm 节拍轻微缩放 */
        float scale = 1.0f;
        if (d->bpm > 0) {
            float period_ms = 60000.0f / (float)d->bpm;
            float phase = (float)(now % (uint64_t)period_ms) / period_ms;
            /* 前 12% 时间快速放大再回落 */
            if (phase < 0.12f)
                scale = 1.0f + 0.06f * (1.0f - phase / 0.12f);
        }
        /* 远程控制「高光闪烁」：800ms 窗口内整体放大 + 外发光脉冲 */
        if (d->flash_until && now < d->flash_until) {
            float fphase = (float)((d->flash_until - now) % 400) / 400.0f;
            scale *= 1.10f + 0.12f * fphase;
        }
        if (scale != 1.0f) {
            gs_matrix_push();
            gs_matrix_scale3f(scale, scale, 1.0f);
            obs_source_video_render(d->text);
            gs_matrix_pop();
        } else {
            obs_source_video_render(d->text);
        }
    }
}

static obs_properties_t *hr_get_properties(void *priv)
{
    (void)priv;
    obs_properties_t *p = obs_properties_create();
    obs_properties_add_text(p, "host", "服务地址", OBS_TEXT_DEFAULT);
    obs_properties_add_int(p, "port", "端口", 1, 65535, 1);
    obs_properties_add_text(p, "sep_t", "— 阈值动作 —", OBS_TEXT_INFO);
    obs_properties_add_int(p, "hr_high", "心率高阈值 (BPM, 0=禁用)", 0, 255, 1);
    obs_properties_add_int(p, "hr_low", "心率低阈值 (BPM, 0=禁用)", 0, 255, 1);
    obs_properties_add_text(p, "scene_high", "越阈切换到的场景名", OBS_TEXT_DEFAULT);
    obs_properties_add_text(p, "scene_low", "低心率切换到的场景名", OBS_TEXT_DEFAULT);
    return p;
}

static void hr_get_defaults(obs_data_t *s)
{
    obs_data_set_string(s, "host", DEFAULT_HOST);
    obs_data_set_int(s, "port", DEFAULT_PORT);
    obs_data_set_int(s, "hr_high", 150);
    obs_data_set_int(s, "hr_low", 55);
    obs_data_set_string(s, "scene_high", "");
    obs_data_set_string(s, "scene_low", "");
}

struct obs_source_info pulsecast_hr_source_info = {
    .id = "pulsecast_hr_source",
    .type = OBS_SOURCE_TYPE_INPUT,
    .output_flags = OBS_SOURCE_VIDEO | OBS_SOURCE_CUSTOM_DRAW,
    .get_name = hr_get_name,
    .create = hr_create,
    .destroy = hr_destroy,
    .update = hr_update,
    .video_render = hr_video_render,
    .get_properties = hr_get_properties,
    .get_defaults = hr_get_defaults,
};
