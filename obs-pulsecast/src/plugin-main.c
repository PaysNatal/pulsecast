/*
 * plugin-main.c — obs-pulsecast 模块入口
 * 注册：心率文字源 + 心跳镜头抖动滤镜 + 心跳发光滤镜。
 */
#include <obs/obs-module.h>

#include "pulsecast-hr-source.h"
#include "pulsecast-shake-filter.h"
#include "pulsecast-glow-filter.h"
#include "pulsecast-atmosphere-filter.h"

OBS_DECLARE_MODULE()
OBS_MODULE_USE_DEFAULT_LOCALE("obs-pulsecast", "zh-CN")

bool obs_module_load(void)
{
    obs_register_source(&pulsecast_hr_source_info);
    obs_register_source(&pulsecast_shake_filter_info);
    obs_register_source(&pulsecast_glow_filter_info);
    obs_register_source(&pulsecast_atmosphere_filter_info);
    blog(LOG_INFO, "[obs-pulsecast] 已加载：心率源 + 抖动/发光/氛围滤镜");
    return true;
}

void obs_module_unload(void)
{
    /* 资源由各 source/filter 的 destroy 回收 */
}

/* 阈值动作（如 bpm>阈值触发热键/可见性）预留接口，后续在桌面端配置下发。 */
