//! 名场面系统 —— 心率峰值自动标记 + OBS Replay Buffer 联动
//!
//! 维护最近 30 分钟的心率帧 ring buffer（1Hz = 1800 帧），
//! 检测心率跨越阈值的瞬间并记录为名场面事件。
//! 可触发 OBS Replay Buffer 自动保存前后 N 秒视频。

use serde::Serialize;
use std::collections::VecDeque;

/// 带时间戳的心率帧（ring buffer 元素）
#[derive(Clone)]
pub struct TimestampedFrame {
    /// Unix 毫秒时间戳
    pub at: u64,
    /// 心率 BPM
    pub bpm: u32,
    /// 归一化强度 0.0~1.0
    pub intensity: f32,
}

/// 名场面事件
#[derive(Clone, Serialize)]
pub struct HighlightEvent {
    /// 触发时刻 Unix 毫秒
    pub at: u64,
    /// 峰值 BPM
    pub peak_bpm: u32,
    /// 从正常到峰值的上升时间（毫秒）
    pub rise_ms: u64,
    /// 高于阈值的持续时间（毫秒）
    pub duration_ms: u64,
    /// 是否已触发 OBS Replay Buffer
    pub replay_saved: bool,
}

/// 名场面引擎
pub struct HighlightsEngine {
    /// 最近 30 分钟的心率帧（1Hz ≈ 1800 帧）
    ring: VecDeque<TimestampedFrame>,
    /// 本场所有名场面事件
    highlights: Vec<HighlightEvent>,
    /// 上一次检测到的"高能状态"起始时间（用于计算持续时间）
    spike_start: Option<u64>,
    /// 上一次检测到的"高能状态"起始 BPM（用于计算上升时间）
    spike_start_bpm: u32,
    /// 高能阈值（intensity > 此值视为高能）
    spike_threshold: f32,
}

impl Default for HighlightsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl HighlightsEngine {
    pub fn new() -> Self {
        Self {
            ring: VecDeque::with_capacity(1800),
            highlights: Vec::new(),
            spike_start: None,
            spike_start_bpm: 0,
            spike_threshold: 0.7, // intensity > 0.7 ≈ BPM > 158
        }
    }

    /// 推入一帧心率数据，自动检测名场面
    pub fn push(&mut self, frame: TimestampedFrame) {
        // 维护 ring buffer 大小（最多 30 分钟）
        if self.ring.len() >= 1800 {
            self.ring.pop_front();
        }
        self.ring.push_back(frame.clone());

        // 检测高能状态跨越
        let is_spike = frame.intensity > self.spike_threshold;
        match (self.spike_start, is_spike) {
            (None, true) => {
                // 进入高能状态
                self.spike_start = Some(frame.at);
                self.spike_start_bpm = frame.bpm;
            }
            (Some(start), false) => {
                // 离开高能状态 → 记录名场面
                let duration = frame.at.saturating_sub(start);
                // 只记录持续超过 2 秒的名场面（过滤噪声）
                if duration > 2000 {
                    // 找峰值
                    let peak = self
                        .ring
                        .iter()
                        .filter(|f| f.at >= start && f.at <= frame.at)
                        .map(|f| f.bpm)
                        .max()
                        .unwrap_or(frame.bpm);

                    self.highlights.push(HighlightEvent {
                        at: start,
                        peak_bpm: peak,
                        rise_ms: self.estimate_rise_ms(start),
                        duration_ms: duration,
                        replay_saved: false, // OBS Replay Buffer 触发后由外部设置
                    });
                }
                self.spike_start = None;
                self.spike_start_bpm = 0;
            }
            _ => {}
        }
    }

    /// 估算上升时间：从 ring buffer 中找高能开始前 10 秒内的最低 BPM 到峰值的时间差
    fn estimate_rise_ms(&self, spike_at: u64) -> u64 {
        let window_start = spike_at.saturating_sub(10_000);
        let pre_frames: Vec<&TimestampedFrame> = self
            .ring
            .iter()
            .filter(|f| f.at >= window_start && f.at <= spike_at)
            .collect();
        if pre_frames.len() < 2 {
            return 0;
        }
        // 找最低点到最高点的时间跨度
        let min_frame = pre_frames.iter().min_by_key(|f| f.bpm);
        let max_frame = pre_frames.iter().max_by_key(|f| f.bpm);
        match (min_frame, max_frame) {
            (Some(lo), Some(hi)) => hi.at.saturating_sub(lo.at),
            _ => 0,
        }
    }

    /// 获取所有名场面事件
    pub fn events(&self) -> &[HighlightEvent] {
        &self.highlights
    }

    /// 标记最近一个名场面已触发 Replay Buffer
    pub fn mark_replay_saved(&mut self) {
        if let Some(last) = self.highlights.last_mut() {
            last.replay_saved = true;
        }
    }

    /// 导出名场面列表为 JSON 字符串
    pub fn export_json(&self) -> String {
        serde_json::to_string_pretty(&self.highlights).unwrap_or_else(|_| "[]".to_string())
    }
}
