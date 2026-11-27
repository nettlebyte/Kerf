use crate::win::*;

pub struct Target {
    pub rect: (i32, i32, i32, i32),
    pub title: String,
}

struct Probe {
    x: i32,
    y: i32,
    found: Option<Target>,
}

unsafe fn is_desktop_class(hwnd: HWND) -> bool {
    let mut buf = [0u16; 64];
    let n = GetClassNameW(hwnd, buf.as_mut_ptr(), 64) as usize;
    if n == 0 {
        return false;
    }
    let name = String::from_utf16_lossy(&buf[..n]);
    matches!(name.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "Shell_ChangeWindowMessageFilter")
}

unsafe fn frame(hwnd: HWND) -> Option<(i32, i32, i32, i32)> {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    if DwmGetWindowAttribute(
        hwnd,
        DWMWA_EXTENDED_FRAME_BOUNDS as u32,
        &mut r as *mut RECT as *mut std::ffi::c_void,
        std::mem::size_of::<RECT>() as u32,
    ) != 0
        && GetWindowRect(hwnd, &mut r) == 0
    {
        return None;
    }
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    if w < 8 || h < 8 {
        return None;
    }
    Some((r.left, r.top, w, h))
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lp: LPARAM) -> i32 {
    let probe = &mut *(lp as *mut Probe);
    if probe.found.is_some() {
        return 0;
    }
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == GetCurrentProcessId() || IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
        return 1;
    }
    let mut cloaked = 0u32;
    if DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED as u32,
        &mut cloaked as *mut u32 as *mut std::ffi::c_void,
        4,
    ) == 0
        && cloaked != 0
    {
        return 1;
    }
    let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
    if ex & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT) != 0 {
        return 1;
    }
    if is_desktop_class(hwnd) {
        return 1;
    }
    let Some(rect) = frame(hwnd) else { return 1 };
    let (x, y, w, h) = rect;
    if probe.x < x || probe.y < y || probe.x >= x + w || probe.y >= y + h {
        return 1;
    }
    let len = GetWindowTextLengthW(hwnd);
    let title = if len > 0 {
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) as usize;
        String::from_utf16_lossy(&buf[..n])
    } else {
        String::new()
    };
    probe.found = Some(Target { rect, title });
    0
}

pub unsafe fn window_at(x: i32, y: i32) -> Option<Target> {
    let mut probe = Probe { x, y, found: None };
    EnumWindows(Some(enum_proc), &mut probe as *mut Probe as LPARAM);
    probe.found
}

pub unsafe fn monitor_at(x: i32, y: i32) -> (i32, i32, i32, i32) {
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        rcMonitor: RECT { left: 0, top: 0, right: 0, bottom: 0 },
        rcWork: RECT { left: 0, top: 0, right: 0, bottom: 0 },
        dwFlags: 0,
    };
    let m = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
    if m.is_null() || GetMonitorInfoW(m, &mut mi) == 0 {
        return (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        );
    }
    (mi.rcMonitor.left, mi.rcMonitor.top, mi.rcMonitor.right - mi.rcMonitor.left, mi.rcMonitor.bottom - mi.rcMonitor.top)
}
