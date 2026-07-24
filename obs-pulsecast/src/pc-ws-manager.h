/*
 * pc-ws-manager.h — 进程级 WebSocket 连接池
 *
 * 所有 OBS 源/滤镜共享同一条 WS 连接（同一 host:port），
 * 引用计数管理生命周期，帧数据 fan-out 到所有注册的回调。
 * 替代每个源/滤镜各开一条连接的旧模式（N 源 = 1 连接而非 N 连接）。
 */
#pragma once
#include "pc-ws.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct pc_ws_mgr pc_ws_mgr_t;

/* 获取共享连接（引用计数 +1），注册 cb/user 用于帧 fan-out。
   首次调用时创建底层 WS 连接；后续调用复用。 */
pc_ws_mgr_t *pc_ws_mgr_acquire(const char *host, int port,
                                pc_ws_on_frame cb, void *user);

/* 释放引用（引用计数 -1），注销 cb/user。
   引用归零时关闭底层 WS 连接。 */
void pc_ws_mgr_release(pc_ws_mgr_t *mgr, pc_ws_on_frame cb, void *user);

#ifdef __cplusplus
}
#endif
