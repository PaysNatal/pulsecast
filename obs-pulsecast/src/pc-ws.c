/*
 * pc-ws.c — 极简 WebSocket 客户端实现（见 pc-ws.h）。
 * 跨平台：POSIX 用 socket + pthread；Windows 用 Winsock2 + _beginthreadex。
 * 不引入任何第三方网络/线程库。
 */
#include "pc-ws.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <arpa/inet.h>
#include <netdb.h>
#include <sys/socket.h>
#include <unistd.h>
#endif

/* 跨平台 socket 句柄类型：Windows 为 64 位 SOCKET，POSIX 为 int（不可混用 int 存储） */
#ifdef _WIN32
typedef SOCKET pc_sock_t;
#define PC_INVALID_SOCKET INVALID_SOCKET
#else
typedef int pc_sock_t;
#define PC_INVALID_SOCKET (-1)
#endif

/* 跨平台线程句柄类型（在 struct 之前声明，供其引用） */
#ifdef _WIN32
typedef HANDLE pc_thread_t;
#else
#include <pthread.h>
typedef pthread_t pc_thread_t;
#endif

struct pc_ws {
    pc_sock_t sock;
    volatile int stop;
    pc_thread_t thread;
    pc_ws_on_frame cb;
    void *user;
    char *host;
    int port;
    char *path;
};

/* 跨平台 socket 关闭 */
static void pc_close(pc_sock_t s)
{
#ifdef _WIN32
    closesocket(s);
#else
    close(s);
#endif
}

/* 跨平台线程封装实现
 * Windows MSVC 不提供 pthread，需用 _beginthreadex（而非 CreateThread）
 * 以正确初始化每线程 CRT 状态（worker 内用到 malloc/strtol 等 CRT 函数）。 */
#ifdef _WIN32
#include <process.h>
static unsigned __stdcall pc_worker_thunk(void *arg);
#endif

/* 前向声明 */
static void *pc_worker(void *arg);

/* 启动工作线程（跨平台） */
static int pc_thread_spawn(pc_thread_t *t, void *arg)
{
#ifdef _WIN32
    HANDLE h = (HANDLE)_beginthreadex(NULL, 0, pc_worker_thunk, arg, 0, NULL);
    if (h == NULL)
        return -1;
    *t = h;
    return 0;
#else
    return pthread_create(t, NULL, pc_worker, arg);
#endif
}

/* 等待工作线程结束并释放句柄 */
static void pc_thread_join(pc_thread_t t)
{
#ifdef _WIN32
    WaitForSingleObject(t, INFINITE);
    CloseHandle(t);
#else
    pthread_join(t, NULL);
#endif
}

/* 从 JSON 文本提取 "bpm":<int>。找不到返回 -1。 */
int pc_ws_parse_bpm(const char *json)
{
    if (!json)
        return -1;
    const char *p = json;
    while ((p = strstr(p, "bpm")) != NULL) {
        /* JSON 键为带引号形式："bpm" — 确认 bpm 后是闭合引号再冒号 */
        const char *q = p + 3;
        if (*q != '"') { /* 不是键的结尾（如出现在某值内） */
            p += 3;
            continue;
        }
        q++; /* 跳过闭合引号 */
        while (*q == ' ' || *q == '\t')
            q++;
        if (*q != ':') {
            p += 3;
            continue;
        }
        q++;
        while (*q == ' ' || *q == '\t')
            q++;
        /* 允许负号（理论上不出现），但解析为整数 */
        char *end = NULL;
        long v = strtol(q, &end, 10);
        if (end != q && v >= 0 && v <= 300) {
            return (int)v;
        }
        p += 3;
    }
    return -1;
}

/* 从 JSON 提取字符串字段 "key":"value" 到 out（不含引号）。
   匹配规则：找到 key 后须紧跟闭合引号 '"' 与冒号，再读取到下一个 '"' 为止。 */
int pc_ws_parse_str(const char *json, const char *key, char *out, size_t outsz)
{
    if (!json || !key || !out || outsz == 0)
        return 0;
    size_t klen = strlen(key);
    const char *p = json;
    while ((p = strstr(p, key)) != NULL) {
        const char *q = p + klen;
        if (*q != '"') { /* 不是键的结尾（如 key 是某长名的子串，或出现在值内） */
            p += klen;
            continue;
        }
        q++; /* 跳过键的闭合引号 */
        while (*q == ' ' || *q == '\t')
            q++;
        if (*q != ':') { /* 键后不是冒号 */
            p += klen;
            continue;
        }
        q++;
        while (*q == ' ' || *q == '\t')
            q++;
        if (*q != '"') { /* 非字符串值（如数字字段），跳过 */
            p += klen;
            continue;
        }
        q++; /* 跳过开头引号 */
        size_t i = 0;
        while (*q && *q != '"' && i + 1 < outsz) {
            out[i++] = *q++;
        }
        out[i] = '\0';
        return 1;
    }
    return 0;
}

/* 从 JSON 提取整数字段 "key":<int>（允许负号）。找不到返回 -1。 */
int pc_ws_parse_int_field(const char *json, const char *key)
{    if (!json || !key)
        return -1;
    size_t klen = strlen(key);
    const char *p = json;
    while ((p = strstr(p, key)) != NULL) {
        const char *q = p + klen;
        if (*q != '"') { /* 不是键的结尾 */
            p += klen;
            continue;
        }
        q++;
        while (*q == ' ' || *q == '\t')
            q++;
        if (*q != ':') {
            p += klen;
            continue;
        }
        q++;
        while (*q == ' ' || *q == '\t')
            q++;
        char *end = NULL;
        long v = strtol(q, &end, 10);
        if (end != q)
            return (int)v;
        p += klen;
    }
    return -1;
}

/* 判定一段 JSON 是否为“远程控制事件”并提取其动作类型（kind）。
 *
 * 契约（与服务端 ControlEvent 一致）：控制事件必须带 "type":"control"，
 * 动作类型在 "kind" 字段。心率帧（含 "bpm"）不含 type，返回 0。
 * 成功时把 kind 写入 out（不含引号），返回 1；否则返回 0。
 *
 * 用法示例（发光滤镜 / HR 源通用）：
 *     char kind[32];
 *     if (pc_ws_parse_control_kind(json, kind, sizeof(kind))) {
 *         if (strcmp(kind, "flash") == 0) { ... }
 *     }
 */
int pc_ws_parse_control_kind(const char *json, char *out, size_t outsz)
{
    if (!json || !out || outsz == 0)
        return 0;
    char typ[32];
    if (!pc_ws_parse_str(json, "type", typ, sizeof(typ)))
        return 0;
    if (strcmp(typ, "control") != 0)
        return 0;
    return pc_ws_parse_str(json, "kind", out, outsz);
}

/* 发送 HTTP Upgrade 握手 */
static int pc_send_handshake(pc_sock_t sock, const char *host, int port, const char *path)
{
    char req[512];
    int n = snprintf(req, sizeof(req),
                     "GET %s HTTP/1.1\r\n"
                     "Host: %s:%d\r\n"
                     "Upgrade: websocket\r\n"
                     "Connection: Upgrade\r\n"
                     "Sec-WebSocket-Key: Vm1sdGlwQ2FzdA==\r\n"
                     "Sec-WebSocket-Version: 13\r\n"
                     "\r\n",
                     path, host, port);
    if (send(sock, req, (size_t)n, 0) < 0)
        return -1;
    return 0;
}

/* 读取到 "\r\n\r\n" 表示握手响应结束 */
static int pc_read_handshake(pc_sock_t sock)
{
    char buf[4];
    int seen_cr = 0, seen_lf = 0, got_blank = 0;
    /* 简单扫描：连续两个 CRLF */
    int state = 0;
    char prev = 0;
    for (int i = 0; i < 4096; i++) {
        char c;
        if (recv(sock, &c, 1, 0) <= 0)
            return -1;
        if (prev == '\r' && c == '\n') {
            if (state == 0)
                state = 1;
            else if (state == 1) {
                got_blank = 1;
                break;
            }
        } else if (c != '\r' && c != '\n') {
            state = 0;
        }
        prev = c;
        (void)buf;
    }
    return got_blank ? 0 : -1;
}

/* 解析单个 WS 帧，返回 payload 长度（写入 out，需调用方 free），并填 opcode/fin。
 * 返回 -1 表示连接关闭/错误。 */
static long pc_read_frame(pc_sock_t sock, char **out, unsigned char *opcode, int *fin)
{
    unsigned char hdr[2];
    if (recv(sock, (char *)hdr, 2, 0) != 2)
        return -1;
    *fin = (hdr[0] & 0x80) ? 1 : 0;
    *opcode = hdr[0] & 0x0F;
    int masked = (hdr[1] & 0x80) ? 1 : 0;
    long len = hdr[1] & 0x7F;
    if (len == 126) {
        unsigned char ext[2];
        if (recv(sock, (char *)ext, 2, 0) != 2)
            return -1;
        len = (long)((ext[0] << 8) | ext[1]);
    } else if (len == 127) {
        unsigned char ext[8];
        if (recv(sock, (char *)ext, 8, 0) != 8)
            return -1;
        len = 0;
        for (int i = 0; i < 8; i++)
            len = (len << 8) | ext[i];
    }
    unsigned char mask[4] = {0, 0, 0, 0};
    if (masked) {
        if (recv(sock, (char *)mask, 4, 0) != 4)
            return -1;
    }
    if (len <= 0 || len > 1048576) /* 最大 1MB，防恶意长度 OOM */
        return -1;
    char *payload = malloc((size_t)len + 1);
    if (!payload)
        return -1;
    long got = 0;
    while (got < len) {
        int r = recv(sock, payload + got, (size_t)(len - got), 0);
        if (r <= 0) {
            free(payload);
            return -1;
        }
        got += r;
    }
    if (masked) {
        for (long i = 0; i < len; i++)
            payload[i] ^= mask[i & 3];
    }
    payload[len] = '\0';
    *out = payload;
    return len;
}

#ifdef _WIN32
/* _beginthreadex 需要的 __stdcall 入口：转调标准签名的 pc_worker */
static unsigned __stdcall pc_worker_thunk(void *arg)
{
    pc_worker(arg);
    return 0;
}
#endif

static void *pc_worker(void *arg)
{
    pc_ws_t *ws = (pc_ws_t *)arg;

    /* 连接 TCP */
    struct addrinfo hints, *res = NULL;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    char portstr[16];
    snprintf(portstr, sizeof(portstr), "%d", ws->port);
    if (getaddrinfo(ws->host, portstr, &hints, &res) != 0 || !res) {
        return NULL;
    }
    pc_sock_t sock = PC_INVALID_SOCKET;
    for (struct addrinfo *r = res; r; r = r->ai_next) {
        sock = socket(r->ai_family, r->ai_socktype, r->ai_protocol);
        if (sock == PC_INVALID_SOCKET)
            continue;
        if (connect(sock, r->ai_addr, r->ai_addrlen) == 0)
            break;
        pc_close(sock);
        sock = PC_INVALID_SOCKET;
    }
    freeaddrinfo(res);
    if (sock == PC_INVALID_SOCKET)
        return NULL;

    if (pc_send_handshake(sock, ws->host, ws->port, ws->path) < 0 ||
        pc_read_handshake(sock) < 0) {
        pc_close(sock);
        return NULL;
    }
    ws->sock = sock;

    /* 帧读取循环（支持分片） */
    char *frag = NULL;
    size_t frag_len = 0;
    while (!ws->stop) {
        char *payload = NULL;
        unsigned char opcode = 0;
        int fin = 0;
        long len = pc_read_frame(sock, &payload, &opcode, &fin);
        if (len < 0) {
            free(payload);
            break;
        }
        if (opcode == 0x8) { /* close */
            free(payload);
            break;
        } else if (opcode == 0x9) { /* ping -> pong */
            /* 简化：忽略，不回 pong（PulseCast 服务端不发 ping） */
            free(payload);
            continue;
        } else if (opcode == 0x1 || opcode == 0x0) { /* text / continuation */
            /* 拼接分片 */
            char *tmp = realloc(frag, frag_len + (size_t)len + 1);
            if (!tmp) {
                free(payload);
                break;
            }
            frag = tmp;
            memcpy(frag + frag_len, payload, (size_t)len);
            frag_len += (size_t)len;
            frag[frag_len] = '\0';
            free(payload);
            if (fin && frag_len > 0) {
                int bpm = pc_ws_parse_bpm(frag);
                if (ws->cb)
                    ws->cb(ws->user, frag, bpm);
                free(frag);
                frag = NULL;
                frag_len = 0;
            }
        } else {
            free(payload);
        }
    }
    free(frag);
    if (ws->sock != PC_INVALID_SOCKET) {
        pc_close(ws->sock);
        ws->sock = PC_INVALID_SOCKET;
    }
    return NULL;
}

pc_ws_t *pc_ws_connect(const char *host, int port, const char *path,
                       pc_ws_on_frame cb, void *user)
{
    pc_ws_t *ws = calloc(1, sizeof(*ws));
    if (!ws)
        return NULL;
    ws->host = strdup(host ? host : "localhost");
    ws->port = port;
    ws->path = strdup(path ? path : "/");
    ws->cb = cb;
    ws->user = user;
    ws->sock = PC_INVALID_SOCKET;
    ws->stop = 0;

#ifdef _WIN32
    /* Windows 必须先初始化 Winsock，否则 socket()/connect() 全部失败（WSANOTINITIALISED） */
    WSADATA wsa;
    if (WSAStartup(MAKEWORD(2, 2), &wsa) != 0) {
        free(ws->host);
        free(ws->path);
        free(ws);
        return NULL;
    }
#endif

    if (pc_thread_spawn(&ws->thread, ws) != 0) {
#ifdef _WIN32
        WSACleanup();
#endif
        free(ws->host);
        free(ws->path);
        free(ws);
        return NULL;
    }
    return ws;
}

void pc_ws_destroy(pc_ws_t *ws)
{
    if (!ws)
        return;
    ws->stop = 1;
    if (ws->sock != PC_INVALID_SOCKET) {
        /* 触发 recv 返回：关闭 socket */
        pc_close(ws->sock);
        ws->sock = PC_INVALID_SOCKET;
    }
    pc_thread_join(ws->thread);
#ifdef _WIN32
    WSACleanup();
#endif
    free(ws->host);
    free(ws->path);
    free(ws);
}

float pc_ws_parse_intensity(const char *json)
{
    const char *p = strstr(json, "\"intensity\"");
    if (!p)
        return -1.0f;
    p = strchr(p, ':');
    if (!p)
        return -1.0f;
    return (float)atof(p + 1);
}
