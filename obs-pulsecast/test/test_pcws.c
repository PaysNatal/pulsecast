/*
 * test_pcws.c — pc-ws JSON 解析 + 控制分发逻辑回归测试。
 * 不依赖 libobs，仅编译 ../src/pc-ws.c 与本文件即可运行：
 *   cc -std=c11 -I../src test_pcws.c ../src/pc-ws.c -o test_pcws -lpthread && ./test_pcws
 */
#include "../src/pc-ws.h"
#include <stdio.h>
#include <string.h>

static int failures = 0;
#define CHECK(c, m) do { if (!(c)) { printf("FAIL: %s\n", m); failures++; } \
                       else printf("OK:   %s\n", m); } while (0)

/* ── 解析函数（真实 pc-ws 实现） ── */
static void test_parsers(void)
{
    const char *hr = "{\"bpm\":128,\"status\":\"live\",\"device\":{\"name\":\"Amazfit\"}}";
    CHECK(pc_ws_parse_bpm(hr) == 128, "hr frame bpm=128");

    const char *ctl_switch = "{\"type\":\"control\",\"kind\":\"switch_scene\",\"scene\":\"亲密时刻\"}";
    char typ[32], kind[32], scene[256];
    CHECK(pc_ws_parse_str(ctl_switch, "type", typ, sizeof(typ)) && strcmp(typ, "control") == 0, "ctl type=control");
    CHECK(pc_ws_parse_str(ctl_switch, "kind", kind, sizeof(kind)) && strcmp(kind, "switch_scene") == 0, "ctl kind=switch_scene");
    CHECK(pc_ws_parse_str(ctl_switch, "scene", scene, sizeof(scene)) && strcmp(scene, "亲密时刻") == 0, "ctl scene UTF-8 完整");
    CHECK(pc_ws_parse_bpm(ctl_switch) == -1, "ctl frame 无 bpm");

    const char *ctl_thr = "{\"type\":\"control\",\"kind\":\"set_threshold\",\"high\":170,\"low\":50}";
    CHECK(pc_ws_parse_int_field(ctl_thr, "high") == 170, "set_threshold high=170");
    CHECK(pc_ws_parse_int_field(ctl_thr, "low") == 50, "set_threshold low=50");

    /* 新契约：pc_ws_parse_control_kind 统一判别（要求 type:"control"） */
    char ck[32];
    CHECK(pc_ws_parse_control_kind("{\"type\":\"control\",\"kind\":\"flash\"}", ck, sizeof(ck)) == 1 &&
          strcmp(ck, "flash") == 0, "control_kind: flash 识别");
    CHECK(pc_ws_parse_control_kind("{\"type\":\"control\",\"kind\":\"switch_scene\",\"scene\":\"x\"}", ck, sizeof(ck)) == 1 &&
          strcmp(ck, "switch_scene") == 0, "control_kind: switch_scene 识别");
    CHECK(pc_ws_parse_control_kind("{\"bpm\":128,\"status\":\"live\"}", ck, sizeof(ck)) == 0,
          "control_kind: 心率帧不识别");
    CHECK(pc_ws_parse_control_kind("{\"kind\":\"flash\"}", ck, sizeof(ck)) == 0,
          "control_kind: 缺 type 不识别（契约要求服务端带 type:\"control\"）");

    const char *ann = "{\"bpm\":75,\"status\":\"live\",\"threshold\":{\"high\":150,\"low\":55},\"trigger\":\"normal\"}";
    char t3[32];
    CHECK(!pc_ws_parse_str(ann, "type", t3, sizeof(t3)), "注解帧无 type");
    CHECK(pc_ws_parse_bpm(ann) == 75, "注解帧 bpm=75");
}

/* ── 控制分发（复刻 on_frame 分支，验证 JSON→动作路由） ── */
static char g_scene[256];
static int g_flash = 0;
static int g_hi = 0, g_lo = 0;
static void stub_switch(const char *n) { if (n && *n) strncpy(g_scene, n, sizeof(g_scene) - 1); }

static void dispatch(const char *json, int bpm)
{
    char kind[32];
    if (pc_ws_parse_control_kind(json, kind, sizeof(kind))) {
        if (strcmp(kind, "switch_scene") == 0) {
            char s[256];
            if (pc_ws_parse_str(json, "scene", s, sizeof(s)) && s[0]) stub_switch(s);
        } else if (strcmp(kind, "flash") == 0) {
            g_flash++;
        } else if (strcmp(kind, "set_threshold") == 0) {
            int hi = pc_ws_parse_int_field(json, "high");
            int lo = pc_ws_parse_int_field(json, "low");
            if (hi > 0 && lo >= 0 && lo < hi) { g_hi = hi; g_lo = lo; }
        }
        return;
    }
    (void)bpm;
}

static void test_dispatch(void)
{
    g_scene[0] = 0;
    dispatch("{\"type\":\"control\",\"kind\":\"switch_scene\",\"scene\":\"亲密时刻\"}", -1);
    CHECK(strcmp(g_scene, "亲密时刻") == 0, "分发: switch_scene → 亲密时刻");

    int before = g_flash;
    dispatch("{\"type\":\"control\",\"kind\":\"flash\"}", -1);
    CHECK(g_flash == before + 1, "分发: flash 计数 +1");

    dispatch("{\"type\":\"control\",\"kind\":\"set_threshold\",\"high\":170,\"low\":50}", -1);
    CHECK(g_hi == 170 && g_lo == 50, "分发: set_threshold 更新高低");

    g_scene[0] = 0; g_flash = 0;
    dispatch("{\"bpm\":128,\"status\":\"live\"}", 128);
    CHECK(g_scene[0] == 0 && g_flash == 0, "分发: 普通心率帧不触发场景/闪烁");
}

int main(void)
{
    test_parsers();
    test_dispatch();
    printf("\n%d failure(s)\n", failures);
    return failures ? 1 : 0;
}
