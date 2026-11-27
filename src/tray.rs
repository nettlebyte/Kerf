use crate::menu;
use crate::overlay;
use crate::ui;
use crate::win::*;

pub const TRAY_MSG: u32 = WM_APP + 10;

pub unsafe fn add(hwnd: HWND) {
    let icon = crate::icons::app_icon(ui::si(32.0));
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = TRAY_MSG;
    nid.hIcon = icon;
    let tip = w("Kerf");
    let n = tip.len().min(127);
    nid.szTip[..n].copy_from_slice(&tip[..n]);
    Shell_NotifyIconW(NIM_ADD, &nid);
}

pub unsafe fn remove() {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = crate::CORE.load(std::sync::atomic::Ordering::Relaxed) as HWND;
    nid.uID = 1;
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

pub unsafe fn handle(hwnd: HWND, lp: LPARAM) {
    let m = lp as u32;
    if m == WM_RBUTTONUP || m == WM_CONTEXTMENU {
        SetForegroundWindow(hwnd);
        menu::show(hwnd);
        return;
    }
    if m == WM_LBUTTONUP || m == WM_LBUTTONDBLCLK {
        overlay::start();
    }
}
