cmake_minimum_required(VERSION 3.16)
project(obs-pulsecast VERSION 0.1.0)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

# —— 构建前置（需 OBS Studio 开发头文件）——
# macOS:   brew install obs
#          或 git clone https://github.com/obsproject/obs-studio 后用 -Dlibobs_DIR=...
# Windows: vcpkg install obs / 或官方 OBS 开发包
# 然后：cmake -B build -Dlibobs_DIR=<obs cmake 目录> && cmake --build build
find_package(libobs QUIET)
if(NOT libobs_FOUND)
    # 允许通过 OBS_SOURCE_DIR 指定 obs-studio 源码树
    if(DEFINED OBS_SOURCE_DIR)
        set(libobs_DIR "${OBS_SOURCE_DIR}/libobs")
        find_package(libobs REQUIRED)
    else()
        message(FATAL_ERROR "找不到 libobs。请安装 OBS 开发包，或传入 -DOBS_SOURCE_DIR=<obs-studio 源码根>")
    endif()
endif()

add_library(obs-pulsecast MODULE
    src/plugin-main.c
    src/pc-ws.c
    src/pulsecast-hr-source.c
    src/pulsecast-shake-filter.c
    src/pulsecast-glow-filter.c
    src/pulsecast-atmosphere-filter.c
)

target_include_directories(obs-pulsecast PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/src)
target_link_libraries(obs-pulsecast PRIVATE OBS::libobs OBS::obs-frontend-api)

# 二进制名必须带 obs- 前缀，OBS 才能识别为插件
set_target_properties(obs-pulsecast PROPERTIES
    PREFIX ""
    OUTPUT_NAME "obs-pulsecast"
)

if(APPLE)
    set_target_properties(obs-pulsecast PROPERTIES
        LINK_FLAGS "-Wl,-undefined,dynamic_lookup")
endif()

# 安装目标（可选）：macOS 放到 ~/Library/Application Support/obs-studio/plugins
install(TARGETS obs-pulsecast DESTINATION .)
