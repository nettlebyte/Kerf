use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;

use crate::gfx as gp;
use crate::ui;
use crate::win::*;

static TOAST_HWND: AtomicUsize = AtomicUsize::new(0);
static INIT: Once = Once::new();

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Ok,
    Err,
    Info,
    Warn,
}

impl Kind {
    fn color(self) -> u32 {
        match self {
            Kind::Ok => ui::OK,
            Kind::Err => ui::DANGER,
            Kind::Info => ui::ACCENT,
            Kind::Warn => ui::WARN,
        }
    }
}

struct ToastState {
    c: gp::Canvas,
    text: String,
    kind: Kind,
    x: i32,
    y: i32,
    t: u32,
}

pub unsafe fn show(text: &str, kind: Kind, cx: i32, y: i32) {
    let old = TOAST_HWND.load(Ordering::Relaxed);
    if old != 0 {
        DestroyWindow(old as HWND);
        TOAST_HWND.store(0, Ordering::Relaxed);
    }
    INIT.call_once(|| {
        let cls_name = w("KerfToast");
        let cls = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wndproc),
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
    });
    let (tw, _) = gp::measure(text, ui::s(13.0), false);
    let m = ui::si(14.0);
    let ww = tw + ui::si(50.0);
    let hh = ui::si(38.0);
    let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
    let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
    let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
    let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);
    let x = (cx - ww / 2 - m).max(vx).min(vx + vw - ww - 2 * m);
    let y = (y - m).max(vy).min(vy + vh - hh - 2 * m);
    let state = Box::new(ToastState {
        c: gp::Canvas::new(ww + 2 * m, hh + 2 * m),
        text: text.to_string(),
        kind,
        x,
        y,
        t: 0,
    });
    let lp = Box::into_raw(state);
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
        w("KerfToast").as_ptr(),
        w("").as_ptr(),
        WS_POPUP,
        x,
        y,
        (*lp).c.w,
        (*lp).c.h,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        GetModuleHandleW(std::ptr::null_mut()),
        lp as *const std::ffi::c_void,
    );
    if hwnd.is_null() {
        drop(Box::from_raw(lp));
        return;
    }
    TOAST_HWND.store(hwnd as usize, Ordering::Relaxed);
    compose(hwnd);
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    SetTimer(hwnd, 1, 16, None);
}

unsafe fn state_ptr(hwnd: HWND) -> *mut ToastState {
    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ToastState
}

pub unsafe fn snapshot() -> Option<(Vec<u8>, i32, i32)> {
    let h = TOAST_HWND.load(Ordering::Relaxed) as HWND;
    let st = state_ptr(h);
    if st.is_null() {
        return None;
    }
    (*st).t = 600;
    compose(h);
    Some(((*st).c.px().to_vec(), (*st).c.w, (*st).c.h))
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = *(lp as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let st = state_ptr(hwnd);
    if st.is_null() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    match msg {
        WM_TIMER => {
            (*st).t += 16;
            if (*st).t >= 1760 {
                KillTimer(hwnd, 1);
                DestroyWindow(hwnd);
            } else {
                compose(hwnd);
            }
            0
        }
        WM_DESTROY => {
            TOAST_HWND.store(0, Ordering::Relaxed);
            0
        }
        WM_NCDESTROY => {
            let _ = Box::from_raw(st);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn compose(hwnd: HWND) {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return;
    }
    let s = &mut *st;
    let t = s.t;
    let a: i32 = if t < 160 {
        t as i32 * 255 / 160
    } else if t < 1540 {
        255
    } else {
        ((1760 - t) as i32 * 255 / 220).max(0)
    };
    let w = s.c.w as f32;
    let m = ui::s(14.0);
    let ph = ui::s(38.0);
    let pw = w - m * 2.0;
    s.c.clear();
    if let Some(g) = gp::Gfx::from_dc(s.c.dc) {
        gp::setup(g.raw);
        gp::shadow(g.raw, m, m, pw, ph, ph / 2.0, 0.5);
        gp::fill_round(g.raw, m, m, pw, ph, ph / 2.0, ui::argb(0xF4, ui::BG));
        gp::stroke_round(g.raw, m + 0.5, m + 0.5, pw - 1.0, ph - 1.0, ph / 2.0, ui::argb(255, ui::LINE_2), 1.0);
        gp::circle(g.raw, m + ui::s(19.0), m + ph / 2.0, ui::s(4.0), ui::argb(255, s.kind.color()), true, 1.0);
    }
    gp::text(&s.c, &s.text, (m + ui::s(31.0)) as i32, (m + ph / 2.0) as i32, ui::s(13.0), false, ui::TEXT, DT_LEFT | DT_VCENTER);
    let lift = ((255 - a) as f32 / 255.0 * ui::s(8.0)) as i32;
    s.c.present(hwnd, s.x, s.y + lift, a as u8);
}
