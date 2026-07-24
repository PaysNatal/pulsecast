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
    /// 峰值时刻 Unix 毫秒（用于 SVG 卡片定位）
    pub peak_at: u64,
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
                    // 找峰值及其时刻
                    let peak_frame = self
                        .ring
                        .iter()
                        .filter(|f| f.at >= start && f.at <= frame.at)
                        .max_by_key(|f| f.bpm);
                    let (peak, peak_at) = match peak_frame {
                        Some(pf) => (pf.bpm, pf.at),
                        None => (frame.bpm, frame.at),
                    };

                    self.highlights.push(HighlightEvent {
                        at: start,
                        peak_bpm: peak,
                        peak_at,
                        rise_ms: self.estimate_rise_ms(start),
                        duration_ms: duration,
                        replay_saved: false,
                    });
                }
                self.spike_start = None;
                self.spike_start_bpm = 0;
            }
            _ => {}
        }
    }

    /// 估算上升时间：从 ring buffer 中找高能开始前 10 秒内的最低 BPM 到峰值的时间差
    /// 仅考虑 spike_at 之前的帧，确保 min 在 max 之前
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
        let min_frame = pre_frames.iter().min_by_key(|f| f.bpm);
        let max_frame = pre_frames.iter().max_by_key(|f| f.bpm);
        match (min_frame, max_frame) {
            (Some(lo), Some(hi)) if lo.at <= hi.at => hi.at.saturating_sub(lo.at),
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

    /// 检查最近一次 push 是否产生了新的名场面（用于触发 Replay Buffer）
    pub fn last_event_is_new(&self, prev_count: usize) -> bool {
        self.highlights.len() > prev_count
    }

    /// 生成名场面 SVG 卡片（心率曲线 + 峰值 + 品牌）
    pub fn export_card_svg(&self, index: usize) -> Option<String> {
        let event = self.highlights.get(index)?;

        // 提取 spike 前后 30 秒的心率数据
        let window_start = event.at.saturating_sub(15_000);
        let window_end = event.at.saturating_add(event.duration_ms).saturating_add(15_000);
        let frames: Vec<&TimestampedFrame> = self
            .ring
            .iter()
            .filter(|f| f.at >= window_start && f.at <= window_end)
            .collect();

        if frames.len() < 2 {
            return None;
        }

        // SVG 尺寸
        let w = 600.0_f32;
        let h = 300.0_f32;
        let pad = 40.0_f32;
        let plot_w = w - pad * 2.0;
        let plot_h = h - pad * 2.0 - 30.0; // 底部留空给标签

        // 归一化坐标
        let t_min = frames[0].at as f32;
        let t_max = frames[frames.len() - 1].at as f32;
        let t_range = (t_max - t_min).max(1.0);
        let bpm_min = 40.0_f32;
        let bpm_max = (event.peak_bpm as f32 + 20.0).max(120.0);
        let bpm_range = bpm_max - bpm_min;

        let points: Vec<String> = frames
            .iter()
            .map(|f| {
                let x = pad + (f.at as f32 - t_min) / t_range * plot_w;
                let y = pad + plot_h - ((f.bpm as f32 - bpm_min) / bpm_range * plot_h).clamp(0.0, plot_h);
                format!("{:.1},{:.1}", x, y)
            })
            .collect();

        // 峰值标记位置（使用实际峰值时刻，而非 spike 起始时刻）
        let peak_x = pad + (event.peak_at as f32 - t_min) / t_range * plot_w;
        let peak_y = pad + plot_h - ((event.peak_bpm as f32 - bpm_min) / bpm_range * plot_h).clamp(0.0, plot_h);

        let ts = {
            let secs = event.at / 1000;
            let h = (secs / 3600) % 24;
            let m = (secs / 60) % 60;
            let s = secs % 60;
            format!("{:02}:{:02}:{:02}", h, m, s)
        };
        let dur = event.duration_ms as f32 / 1000.0;

        Some(format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <rect width="{w}" height="{h}" rx="16" fill="#0E1116"/>
  <text x="{pad}" y="28" font-family="system-ui,sans-serif" font-size="14" fill="#8B929A">怦然 PulseCast · 名场面</text>
  <polyline points="{points}" fill="none" stroke="#FF4D6D" stroke-width="2.5" stroke-linejoin="round" stroke-linecap="round"/>
  <circle cx="{peak_x:.1}" cy="{peak_y:.1}" r="5" fill="#FF4D6D"/>
  <text x="{peak_x:.1}" y="{peak_y:.1}" dy="-12" text-anchor="middle" font-family="system-ui,sans-serif" font-size="18" font-weight="700" fill="#FF4D6D">{peak} BPM</text>
  <text x="{pad}" y="{h:.0}" dy="-10" font-family="system-ui,sans-serif" font-size="12" fill="#8B929A">{ts} · 持续 {dur:.1}s</text>
  <text x="{w:.0}" y="{h:.0}" dy="-10" dx="-{pad}" text-anchor="end" font-family="system-ui,sans-serif" font-size="11" fill="#555">♥ PulseCast</text>
</svg>"##,
            w = w, h = h, pad = pad,
            points = points.join(" "),
            peak_x = peak_x, peak_y = peak_y,
            peak = event.peak_bpm,
            ts = ts, dur = dur,
        ))
    }
}
