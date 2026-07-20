/*
 * pc-ws.h — 极简 WebSocket 客户端（仅依赖 POSIX socket + libobs 线程）
 * 用途：订阅 PulseCast 本地服务 ws://localhost:4567 的心率帧。
 * 仅支持读（服务端 -> 客户端）文本帧，不做掩码（客户端接收不需掩码）。
 * 不做 TLS（本地 4567 明文）。够用且零外部依赖。
 */
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct pc_ws pc_ws_t;

/* 每收到一帧 JSON 文本回调。bpm 已从 JSON 中提取（找不到为 -1）。 */
typedef void (*pc_ws_on_frame)(void *user, const char *json, int bpm);

/* 连接并启动后台读取线程。host 形如 "localhost"，port 如 4567，path 如 "/"。 */
pc_ws_t *pc_ws_connect(const char *host, int port, const char *path,
                       pc_ws_on_frame cb, void *user);

/* 停止并释放。 */
void pc_ws_destroy(pc_ws_t *ws);

/* 从一段 JSON 文本里提取 "bpm":<int>（找不到返回 -1）。 */
int pc_ws_parse_bpm(const char *json);

/* 从 JSON 提取字符串字段 "key":"value" 写入 out（不含引号，最多 outsz-1 字节）。
   成功返回 1，否则返回 0。值内的转义/嵌套结构不处理（够用即可）。 */
int pc_ws_parse_str(const char *json, const char *key, char *out, size_t outsz);

/* 从 JSON 提取整数字段 "key":<int>。找不到返回 -1。 */
int pc_ws_parse_int_field(const char *json, const char *key);

/* 判定 JSON 是否为远程控制事件（type=="control"）并提取 kind 到 out。
   成功返回 1 并把 kind 写入 out，否则返回 0。 */
int pc_ws_parse_control_kind(const char *json, char *out, size_t outsz);

#ifdef __cplusplus
}
#endif
