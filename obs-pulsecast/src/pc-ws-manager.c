/*
 * pc-ws-manager.c — 进程级 WebSocket 连接池实现
 *
 * 单例模式：同一 (host, port) 只维护一条 pc_ws_t 连接。
 * 所有注册的回调在收到帧时被逐一调用（fan-out）。
 * 线程安全：所有操作通过 pthread_mutex 保护。
 */
#include "pc-ws-manager.h"

#include <pthread.h>
#include <stdlib.h>
#include <string.h>

#define MAX_CALLBACKS 16

typedef struct {
    pc_ws_on_frame cb;
    void *user;
} mgr_cb_entry;

struct pc_ws_mgr {
    pc_ws_t *ws;
    char *host;
    int port;
    int refcount;
    pthread_mutex_t lock;

    /* 注册的回调列表 */
    mgr_cb_entry cbs[MAX_CALLBACKS];
    int cb_count;
};

/* 全局单例（当前仅支持一个 host:port，足够本地 4567 场景） */
static pc_ws_mgr_t *g_mgr = NULL;
static pthread_mutex_t g_mgr_lock = PTHREAD_MUTEX_INITIALIZER;

/* 内部 fan-out 回调：pc_ws 收到帧时调用。
   在锁外调用回调，防止回调中调用 release 导致死锁。 */
static void mgr_on_frame(void *user, const char *json, int bpm)
{
    pc_ws_mgr_t *m = (pc_ws_mgr_t *)user;

    /* 在锁内复制回调列表 */
    mgr_cb_entry snapshot[MAX_CALLBACKS];
    int count;
    pthread_mutex_lock(&m->lock);
    count = m->cb_count;
    for (int i = 0; i < count; i++)
        snapshot[i] = m->cbs[i];
    pthread_mutex_unlock(&m->lock);

    /* 在锁外调用回调 */
    for (int i = 0; i < count; i++) {
        if (snapshot[i].cb)
            snapshot[i].cb(snapshot[i].user, json, bpm);
    }
}

pc_ws_mgr_t *pc_ws_mgr_acquire(const char *host, int port,
                                pc_ws_on_frame cb, void *user)
{
    pthread_mutex_lock(&g_mgr_lock);

    if (!g_mgr) {
        g_mgr = calloc(1, sizeof(*g_mgr));
        if (!g_mgr) {
            pthread_mutex_unlock(&g_mgr_lock);
            return NULL;
        }
        g_mgr->host = strdup(host ? host : "localhost");
        g_mgr->port = port > 0 ? port : 4567;
        g_mgr->refcount = 0;
        g_mgr->cb_count = 0;
        pthread_mutex_init(&g_mgr->lock, NULL);
        g_mgr->ws = pc_ws_connect(g_mgr->host, g_mgr->port, "/ws",
                                   mgr_on_frame, g_mgr);
        if (!g_mgr->ws) {
            /* 连接失败：清理并返回 NULL，下次 acquire 会重试 */
            pthread_mutex_destroy(&g_mgr->lock);
            free(g_mgr->host);
            free(g_mgr);
            g_mgr = NULL;
            pthread_mutex_unlock(&g_mgr_lock);
            return NULL;
        }
    }

    /* 注册回调 */
    if (g_mgr->cb_count < MAX_CALLBACKS) {
        pthread_mutex_lock(&g_mgr->lock);
        g_mgr->cbs[g_mgr->cb_count].cb = cb;
        g_mgr->cbs[g_mgr->cb_count].user = user;
        g_mgr->cb_count++;
        pthread_mutex_unlock(&g_mgr->lock);
    }

    g_mgr->refcount++;
    pc_ws_mgr_t *ret = g_mgr;
    pthread_mutex_unlock(&g_mgr_lock);
    return ret;
}

void pc_ws_mgr_release(pc_ws_mgr_t *mgr, pc_ws_on_frame cb, void *user)
{
    if (!mgr)
        return;

    pthread_mutex_lock(&g_mgr_lock);

    /* 注销回调 */
    pthread_mutex_lock(&mgr->lock);
    for (int i = 0; i < mgr->cb_count; i++) {
        if (mgr->cbs[i].cb == cb && mgr->cbs[i].user == user) {
            /* 用最后一个填充空位 */
            mgr->cbs[i] = mgr->cbs[mgr->cb_count - 1];
            mgr->cb_count--;
            break;
        }
    }
    pthread_mutex_unlock(&mgr->lock);

    mgr->refcount--;
    if (mgr->refcount <= 0) {
        if (mgr->ws)
            pc_ws_destroy(mgr->ws);
        pthread_mutex_destroy(&mgr->lock);
        free(mgr->host);
        free(mgr);
        g_mgr = NULL;
    }

    pthread_mutex_unlock(&g_mgr_lock);
}
