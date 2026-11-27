#![windows_subsystem = "windows"]

mod capture;
mod config;
mod gfx;
mod icons;
mod menu;
mod overlay;
mod selftest;
mod settings;
mod shapes;
mod target;
mod toolbar;
mod toast;
mod tray;
mod ui;
mod widgets;
mod win;

use std::sync::atomic::{AtomicUsize, Ordering};

use toast::Kind;
use win::*;

pub static CORE: AtomicUsize = AtomicUsize::new(0);

pub const WM_TRIGGER: u32 = WM_APP + 2;
pub const OPEN_SETTINGS: u32 = WM_APP + 5;

unsafe extern "system" fn core_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            match wp as usize {
                1 => overlay::start(),
                2 => fullscreen_copy(),
                3 => repeat_last(),
                _ => {}
            }
            0
        }
        WM_TRIGGER => {
            overlay::start();
            0
        }
        OPEN_SETTINGS => {
            settings::open();
            0
        }
        settings::WM_REREGISTER => {
            register_hotkeys(hwnd);
            0
        }
        menu::WM_CMD => {
            match wp as usize {
                menu::CMD_REGION => overlay::start(),
                menu::CMD_FULL => fullscreen_copy(),
                menu::CMD_REPEAT => repeat_last(),
                menu::CMD_SETTINGS => settings::open(),
                menu::CMD_EXIT => PostQuitMessage(0),
                _ => {}
            }
            0
        }
        tray::TRAY_MSG => {
            tray::handle(hwnd, lp);
            0
        }
        WM_DESTROY => {
            tray::remove();
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn register_hotkeys(hwnd: HWND) {
    for id in 1..4 {
        UnregisterHotKey(hwnd, id);
    }
    let cfg = config::get();
    let mut ok = true;
    if RegisterHotKey(hwnd, 1, cfg.region.mods(), cfg.region.vk) == 0 {
        ok = false;
    }
    if RegisterHotKey(hwnd, 2, cfg.fullscreen.mods(), cfg.fullscreen.vk) == 0 {
        ok = false;
    }
    if RegisterHotKey(hwnd, 3, cfg.repeat.mods(), cfg.repeat.vk) == 0 {
        ok = false;
    }
    if !ok {
        let cx = GetSystemMetrics(SM_XVIRTUALSCREEN) + GetSystemMetrics(SM_CXVIRTUALSCREEN) / 2;
        let cy = GetSystemMetrics(SM_YVIRTUALSCREEN) + GetSystemMetrics(SM_CYVIRTUALSCREEN) - ui::si(90.0);
        toast::show("Some hotkeys are busy - open Settings", Kind::Warn, cx, cy);
    }
}

pub unsafe fn fullscreen_copy() {
    let shot = capture::Shot::grab();
    let r = (shot.vx, shot.vy, shot.w(), shot.h());
    let crop = capture::crop(&shot, r, &[]);
    let dib = capture::dib_bytes(&crop);
    let png = capture::encode(&crop, false).unwrap_or_default();
    let ok = capture::clipboard_put(&dib, &png);
    if config::get().sound && ok {
        MessageBeep(0);
    }
    let cx = GetSystemMetrics(SM_XVIRTUALSCREEN) + GetSystemMetrics(SM_CXVIRTUALSCREEN) / 2;
    let cy = GetSystemMetrics(SM_YVIRTUALSCREEN) + GetSystemMetrics(SM_CYVIRTUALSCREEN) / 2;
    if ok {
        toast::show("Fullscreen copied to clipboard", Kind::Ok, cx, cy);
    } else {
        toast::show("Copy failed", Kind::Err, cx, cy);
    }
}

pub unsafe fn repeat_last() {
    let cfg = config::get();
    let cx = GetSystemMetrics(SM_XVIRTUALSCREEN) + GetSystemMetrics(SM_CXVIRTUALSCREEN) / 2;
    let cy = GetSystemMetrics(SM_YVIRTUALSCREEN) + GetSystemMetrics(SM_CYVIRTUALSCREEN) / 2;
    match cfg.last {
        None => toast::show("No previous capture yet", Kind::Info, cx, cy),
        Some(r) => {
            let shot = capture::Shot::grab();
            let crop = capture::crop(&shot, (r[0], r[1], r[2], r[3]), &[]);
            let dib = capture::dib_bytes(&crop);
            let png = capture::encode(&crop, false).unwrap_or_default();
            let ok = capture::clipboard_put(&dib, &png);
            if config::get().sound && ok {
                MessageBeep(0);
            }
            if ok {
                toast::show("Region copied to clipboard", Kind::Ok, r[0] + r[2] / 2, r[1] + r[3] / 2 + r[3] / 2 + ui::si(16.0));
            } else {
                toast::show("Copy failed", Kind::Err, cx, cy);
            }
        }
    }
}

unsafe fn set_dpi_aware() {
    let u = LoadLibraryA(b"user32.dll\0".as_ptr());
    if !u.is_null() {
        let p = GetProcAddress(u, b"SetProcessDpiAwarenessContext\0".as_ptr());
        if let Some(f) = p {
            let f: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32 = std::mem::transmute(f);
            if f(-4isize as *mut std::ffi::c_void) != 0 {
                return;
            }
        }
    }
    let s = LoadLibraryA(b"shcore.dll\0".as_ptr());
    if !s.is_null() {
        let p = GetProcAddress(s, b"SetProcessDpiAwareness\0".as_ptr());
        if let Some(f) = p {
            let f: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(f);
            if f(2) == 0 {
                return;
            }
        }
    }
    if !u.is_null() {
        let p = GetProcAddress(u, b"SetProcessDPIAware\0".as_ptr());
        if let Some(f) = p {
            let f: unsafe extern "system" fn() -> i32 = std::mem::transmute(f);
            f();
        }
    }
}

unsafe fn register_class() {
    let cls_name = w("KerfCore");
    let cls = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(core_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: GetModuleHandleW(std::ptr::null_mut()),
        hIcon: std::ptr::null_mut(),
        hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: cls_name.as_ptr(),
    };
    RegisterClassW(&cls);
}

unsafe fn forward(arg: Option<&str>) {
    let h = FindWindowW(w("KerfCore").as_ptr(), std::ptr::null());
    if h.is_null() {
        return;
    }
    let (msg, wp) = match arg {
        Some("--settings") => (OPEN_SETTINGS, 0usize),
        Some("--full") => (menu::WM_CMD, menu::CMD_FULL),
        Some("--repeat") => (menu::WM_CMD, menu::CMD_REPEAT),
        _ => (WM_TRIGGER, 0),
    };
    PostMessageW(h, msg, wp as WPARAM, 0);
}

fn main() {
    unsafe {
        let args: Vec<String> = std::env::args().collect();
        let arg = args.get(1).map(|s| s.as_str());
        set_dpi_aware();
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        ui::set_scale(ui::dpi_scale_at(vx + vw / 2, vy + vh / 2));
        CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
        gfx::startup();
        let mutex = CreateMutexExW(std::ptr::null(), w("Kerf.SingleInstance.Mutex").as_ptr(), 0, 0x1F0001);
        if !mutex.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
            forward(arg);
            std::process::exit(0);
        }
        config::load();
        register_class();
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w("KerfCore").as_ptr(),
            w("Kerf").as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            GetModuleHandleW(std::ptr::null_mut()),
            std::ptr::null(),
        );
        if hwnd.is_null() {
            std::process::exit(1);
        }
        CORE.store(hwnd as usize, Ordering::Relaxed);
        let hicon = icons::app_icon(ui::si(32.0));
        let hicon_s = icons::app_icon(ui::si(16.0));
        PostMessageW(hwnd, 0x0080, 1, hicon as isize);
        PostMessageW(hwnd, 0x0080, 0, hicon_s as isize);
        tray::add(hwnd);
        register_hotkeys(hwnd);
        if let Some("--selftest") = arg {
            let dir = args.get(2).cloned().unwrap_or_else(|| std::env::temp_dir().to_string_lossy().to_string());
            selftest::run(&dir);
            tray::remove();
            std::process::exit(0);
        }
        let mut m: MSG = std::mem::zeroed();
        loop {
            let r = GetMessageW(&mut m, std::ptr::null_mut(), 0, 0);
            if r <= 0 {
                break;
            }
            TranslateMessage(&m);
            DispatchMessageW(&m);
        }
        std::process::exit(0);
    }
}
