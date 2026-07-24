/*
 * pulsecast-shake-filter.c — 「心跳镜头抖动」滤镜
 *
 * 作用：对应用了该滤镜的视频/图片源，按当前心率节拍施加一次快速衰减的位移抖动，
 *       让画面在每次“怦然”时轻微一震，强化主播心跳的临场感。
 * 数据源：同心率源，订阅 ws://<host>:<port> 取得实时 bpm。
 */
#include "pulsecast-shake-filter.h"
#include "pc-ws.h"

#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define DEFAULT_HOST "localhost"
#define DEFAULT_PORT 4567

struct shake_data {
    obs_source_t *source;
    char *host;
    int port;
    pc_ws_t *ws;
    pthread_mutex_t lock;
    int bpm;
    float amplitude; /* 像素 */
};

static void on_frame(void *user, const char *json, int bpm)
{
    (void)json;
    struct shake_data *d = (struct shake_data *)user;
    pthread_mutex_lock(&d->lock);
    if (bpm > 0)
        d->bpm = bpm;
    pthread_mutex_unlock(&d->lock);
}

static const char *shake_get_name(void *priv)
{
    (void)priv;
    return "心跳镜头抖动 (PulseCast)";
}

static void *shake_create(obs_data_t *settings, obs_source_t *source)
{
    struct shake_data *d = calloc(1, sizeof(*d));
    if (!d)
        return NULL;
    d->source = source;
    d->bpm = 72;
    pthread_mutex_init(&d->lock, NULL);
    d->host = bstrdup(obs_data_get_string(settings, "host") ?: DEFAULT_HOST);
    d->port = (int)obs_data_get_int(settings, "port");
    if (d->port <= 0)
        d->port = DEFAULT_PORT;
    d->amplitude = (float)obs_data_get_double(settings, "amplitude");
    if (d->amplitude <= 0.0f)
        d->amplitude = 8.0f;

    d->ws = pc_ws_connect(d->host, d->port, "/ws", on_frame, d);
    return d;
}

static void shake_destroy(void *priv)
{
    struct shake_data *d = (struct shake_data *)priv;
    if (!d)
        return;
    if (d->ws)
        pc_ws_destroy(d->ws);
    pthread_mutex_destroy(&d->lock);
    bfree(d->host);
    free(d);
}

static void shake_update(void *priv, obs_data_t *settings)
{
    struct shake_data *d = (struct shake_data *)priv;
    d->amplitude = (float)obs_data_get_double(settings, "amplitude");
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
        pc_ws_destroy(d->ws);
        d->ws = pc_ws_connect(d->host, d->port, "/ws", on_frame, d);
    }
}

/* 计算当前抖动位移（每次心跳快速衰减的正弦脉冲） */
static void compute_offset(struct shake_data *d, float *dx, float *dy)
{
    pthread_mutex_lock(&d->lock);
    int bpm = d->bpm;
    float amp = d->amplitude;
    pthread_mutex_unlock(&d->lock);

    *dx = 0.0f;
    *dy = 0.0f;
    if (bpm <= 0)
        return;

    uint64_t now_ms = os_gettime_ns() / 1000000;
    float period_ms = 60000.0f / (float)bpm;
    float phase = (float)(now_ms % (uint64_t)period_ms) / period_ms; /* 0..1 */

    /* 前 18% 周期内：衰减正弦，模拟“咚”一下的震动 */
    if (phase < 0.18f) {
        float t = phase / 0.18f;             /* 0..1 */
        float decay = (1.0f - t);            /* 线性衰减 */
        float osc = sinf(t * 3.14159f * 3.0f); /* 3 个半波 */
        float mag = amp * decay * osc;
        /* x/y 用不同相位，制造小幅随机感 */
        *dx = mag;
        *dy = mag * 0.6f;
    }
}

static void shake_video_render(void *priv, gs_effect_t *effect)
{
    struct shake_data *d = (struct shake_data *)priv;
    if (!obs_source_process_filter_begin(d->source, GS_RGBA,
                                         OBS_NO_DIRECT_RENDERING))
        return;

    float dx, dy;
    compute_offset(d, &dx, &dy);

    if (dx != 0.0f || dy != 0.0f) {
        gs_matrix_push();
        gs_matrix_translate3f(dx, dy, 0.0f);
        obs_source_process_filter_end(d->source, effect, 0, 0);
        gs_matrix_pop();
    } else {
        obs_source_process_filter_end(d->source, effect, 0, 0);
    }
}

static obs_properties_t *shake_get_properties(void *priv)
{
    (void)priv;
    obs_properties_t *p = obs_properties_create();
    obs_properties_add_text(p, "host", "服务地址", OBS_TEXT_DEFAULT);
    obs_properties_add_int(p, "port", "端口", 1, 65535, 1);
    obs_properties_add_float(p, "amplitude", "抖动幅度 (px)", 0.0, 40.0, 0.5);
    return p;
}

static void shake_get_defaults(obs_data_t *s)
{
    obs_data_set_string(s, "host", DEFAULT_HOST);
    obs_data_set_int(s, "port", DEFAULT_PORT);
    obs_data_set_double(s, "amplitude", 8.0);
}

struct obs_source_info pulsecast_shake_filter_info = {
    .id = "pulsecast_shake_filter",
    .type = OBS_SOURCE_TYPE_FILTER,
    .output_flags = OBS_SOURCE_VIDEO,
    .get_name = shake_get_name,
    .create = shake_create,
    .destroy = shake_destroy,
    .update = shake_update,
    .video_render = shake_video_render,
    .get_properties = shake_get_properties,
    .get_defaults = shake_get_defaults,
};
