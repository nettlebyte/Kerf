use std::sync::atomic::{AtomicU32, Ordering};

use crate::win::*;

pub const BG: u32 = 0x1C1C21;
pub const LINE_2: u32 = 0x4A4A56;
pub const TEXT: u32 = 0xEBEBF5;
pub const TEXT_2: u32 = 0xA3A3B0;
pub const TEXT_3: u32 = 0x68687A;
pub const ACCENT: u32 = 0x3E8BFF;
pub const ACCENT_2: u32 = 0x74B4FF;
pub const DANGER: u32 = 0xFF6B6B;
pub const OK: u32 = 0x5CD695;
pub const WARN: u32 = 0xFFDB4D;

pub fn argb(a: u32, c: u32) -> u32 {
    ((a & 0xFF) << 24) | (c & 0xFFFFFF)
}

pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |sh: u32| {
        let x = ((a >> sh) & 0xFF) as f32;
        let y = ((b >> sh) & 0xFF) as f32;
        (((x + (y - x) * t + 0.5) as u32) & 0xFF) << sh
    };
    0xFF000000 | ch(16) | ch(8) | ch(0)
}

pub fn luma(c: u32) -> f32 {
    let r = ((c >> 16) & 0xFF) as f32;
    let g = ((c >> 8) & 0xFF) as f32;
    let b = (c & 0xFF) as f32;
    (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0
}

static SCALE: AtomicU32 = AtomicU32::new(100);

pub fn set_scale(v: f32) {
    SCALE.store(((v * 100.0).round() as u32).clamp(100, 400), Ordering::Relaxed);
}

pub fn scale() -> f32 {
    SCALE.load(Ordering::Relaxed) as f32 / 100.0
}

pub fn s(v: f32) -> f32 {
    (v * scale() * 2.0).round() / 2.0
}

pub fn si(v: f32) -> i32 {
    (v * scale()).round() as i32
}

pub unsafe fn dpi_scale_at(x: i32, y: i32) -> f32 {
    let m = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
    let (mut dx, mut dy) = (96u32, 96u32);
    if !m.is_null() && GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) != 0 {
        dx = 96;
    }
    if dx == 0 {
        dx = 96;
    }
    dx as f32 / 96.0
}

pub unsafe fn work_area(x: i32, y: i32) -> RECT {
    let m = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        rcMonitor: RECT { left: 0, top: 0, right: 0, bottom: 0 },
        rcWork: RECT { left: 0, top: 0, right: 0, bottom: 0 },
        dwFlags: 0,
    };
    if m.is_null() || GetMonitorInfoW(m, &mut mi) == 0 {
        mi.rcWork = RECT {
            left: GetSystemMetrics(SM_XVIRTUALSCREEN),
            top: GetSystemMetrics(SM_YVIRTUALSCREEN),
            right: GetSystemMetrics(SM_XVIRTUALSCREEN) + GetSystemMetrics(SM_CXVIRTUALSCREEN),
            bottom: GetSystemMetrics(SM_YVIRTUALSCREEN) + GetSystemMetrics(SM_CYVIRTUALSCREEN),
        };
    }
    mi.rcWork
}
