use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;

use crate::config;
use crate::gfx as gp;
use crate::icons;
use crate::ui;
use crate::widgets as wg;
use crate::win::*;

pub const WM_CMD: u32 = WM_APP + 4;
pub const CMD_REGION: usize = 1;
pub const CMD_FULL: usize = 2;
pub const CMD_REPEAT: usize = 3;
pub const CMD_SETTINGS: usize = 4;
pub const CMD_EXIT: usize = 5;

static MENU_HWND: AtomicUsize = AtomicUsize::new(0);
static INIT: Once = Once::new();

struct Entry {
    label: &'static str,
    key: String,
    cmd: usize,
    icon: usize,
    danger: bool,
}

struct Item {
    y: f32,
    h: f32,
    entry: Option<usize>,
}

struct MenuState {
    owner: HWND,
    c: gp::Canvas,
    back: Vec<u8>,
    bw: usize,
    bh: usize,
    activated: bool,
    entries: Vec<Entry>,
    items: Vec<Item>,
    hover: i32,
    alpha: u8,
    x: i32,
    y: i32,
    w: f32,
}

fn entries() -> Vec<Entry> {
    let cfg = config::get();
    vec![
        Entry { label: "Capture region", key: cfg.region.label(), cmd: CMD_REGION, icon: 0, danger: false },
        Entry { label: "Capture fullscreen", key: cfg.fullscreen.label(), cmd: CMD_FULL, icon: 1, danger: false },
        Entry { label: "Repeat last region", key: cfg.repeat.label(), cmd: CMD_REPEAT, icon: 2, danger: false },
        Entry { label: "Settings", key: String::new(), cmd: CMD_SETTINGS, icon: 3, danger: false },
        Entry { label: "Exit", key: String::new(), cmd: CMD_EXIT, icon: 4, danger: true },
    ]
}

fn layout() -> (f32, Vec<Item>) {
    let k = ui::scale();
    let pad = ui::s(8.0);
    let ih = ui::s(38.0);
    let header = ui::s(46.0);
    let w = ui::s(268.0);
    let sep = ui::s(7.0);
    let mut y = pad + header;
    let mut items = Vec::new();
    for i in 0..5 {
        items.push(Item { y, h: ih, entry: Some(i) });
        y += ih;
        if i == 2 || i == 3 {
            items.push(Item { y: y + ui::s(2.0), h: sep, entry: None });
            y += ui::s(8.0);
        }
    }
    let _ = k;
    (w, items)
}

fn total_h(items: &[Item]) -> f32 {
    let pad = ui::s(8.0);
    let last = items.last().map(|i| i.y + i.h).unwrap_or(0.0);
    last + pad
}

pub unsafe fn show(owner: HWND) {
    let old = MENU_HWND.load(Ordering::Relaxed);
    if old != 0 {
        DestroyWindow(old as HWND);
        MENU_HWND.store(0, Ordering::Relaxed);
    }
    INIT.call_once(|| {
        let cls_name = w("KerfMenu");
        let cls = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null_mut()),
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_HAND),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: cls_name.as_ptr(),
        };
        RegisterClassW(&cls);
    });
    let (pw, items) = layout();
    let h = total_h(&items);
    let m = ui::s(16.0);
    let cw = (pw + m * 2.0) as i32;
    let ch = (h + m * 2.0) as i32;
    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    let wa = ui::work_area(pt.x, pt.y);
    let x = (pt.x - ui::si(10.0)).min(wa.right - cw - ui::si(8.0)).max(wa.left + ui::si(8.0));
    let y = (pt.y - ui::si(12.0)).min(wa.bottom - ch - ui::si(8.0)).max(wa.top + ui::si(8.0));
    let shot = crate::capture::Shot::rect(x, y, cw, ch);
    let (back, bw, bh) = gp::acrylic(shot.c.bits, cw, ch, (m as i32, m as i32, pw as i32, h as i32), ui::s(14.0), ui::BG, 0.78, ui::si(18.0) as usize);
    let state = Box::new(MenuState {
        owner,
        c: gp::Canvas::new(cw, ch),
        back,
        bw,
        bh,
        activated: false,
        entries: entries(),
        items,
        hover: -1,
        alpha: 0,
        x,
        y,
        w: pw,
    });
    let lp = Box::into_raw(state);
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        w("KerfMenu").as_ptr(),
        w("").as_ptr(),
        WS_POPUP,
        x,
        y,
        cw,
        ch,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        GetModuleHandleW(std::ptr::null_mut()),
        lp as *const std::ffi::c_void,
    );
    if hwnd.is_null() {
        drop(Box::from_raw(lp));
        return;
    }
    MENU_HWND.store(hwnd as usize, Ordering::Relaxed);
    compose(hwnd);
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    SetForegroundWindow(hwnd);
    SetFocus(hwnd);
    SetTimer(hwnd, 1, 16, None);
}

unsafe fn state_ptr(hwnd: HWND) -> *mut MenuState {
    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut MenuState
}

pub fn hwnd() -> HWND {
    MENU_HWND.load(Ordering::Relaxed) as HWND
}

pub unsafe fn snapshot(hwnd: HWND) -> Option<(Vec<u8>, i32, i32)> {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return None;
    }
    (*st).alpha = 255;
    compose(hwnd);
    Some(((*st).c.px().to_vec(), (*st).c.w, (*st).c.h))
}

fn hit_at(s: &MenuState, y: f32) -> i32 {
    for it in &s.items {
        if let Some(e) = it.entry {
            if y >= it.y && y <= it.y + it.h {
                return e as i32;
            }
        }
    }
    -1
}

fn next_entry(s: &MenuState, from: i32, dir: i32) -> i32 {
    let n = s.entries.len() as i32;
    let mut i = from;
    for _ in 0..n {
        i += dir;
        if i < 0 {
            i = n - 1;
        }
        if i >= n {
            i = 0;
        }
        if i != from {
            return i;
        }
    }
    from
}

unsafe fn run(hwnd: HWND, s: &mut MenuState, idx: i32) {
    if idx < 0 || idx as usize >= s.entries.len() {
        return;
    }
    let cmd = s.entries[idx as usize].cmd;
    PostMessageW(s.owner, WM_CMD, cmd as WPARAM, 0);
    DestroyWindow(hwnd);
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
        WM_MOUSEMOVE => {
            let h = hit_at(&*st, lp_y(lp) as f32 - ui::s(16.0));
            if h != (*st).hover {
                (*st).hover = h;
                compose(hwnd);
            }
            0
        }
        WM_LBUTTONUP => {
            let h = hit_at(&*st, lp_y(lp) as f32 - ui::s(16.0));
            run(hwnd, &mut *st, h);
            0
        }
        WM_RBUTTONUP => {
            DestroyWindow(hwnd);
            0
        }
        WM_KILLFOCUS | WM_ACTIVATE => {
            if msg == WM_ACTIVATE {
                let st_code = lp as u32 & 0xFFFF;
                if st_code != WA_INACTIVE {
                    (*st).activated = true;
                } else if (*st).activated {
                    DestroyWindow(hwnd);
                }
            } else if (*st).activated {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_KEYDOWN => {
            const ESC: u32 = VK_ESCAPE as u32;
            const UP: u32 = VK_UP as u32;
            const DOWN: u32 = VK_DOWN as u32;
            const RET: u32 = VK_RETURN as u32;
            let h = (*st).hover;
            match wp as u32 {
                ESC => {
                    DestroyWindow(hwnd);
                }
                UP => {
                    (*st).hover = next_entry(&*st, if h < 0 { 0 } else { h }, -1);
                    compose(hwnd);
                }
                DOWN => {
                    (*st).hover = next_entry(&*st, if h < 0 { -1 } else { h }, 1);
                    compose(hwnd);
                }
                RET => run(hwnd, &mut *st, h),
                _ => {}
            }
            0
        }
        WM_TIMER => {
            let a = (((*st).alpha as u32) + 24).min(255) as u8;
            (*st).alpha = a;
            compose(hwnd);
            if a >= 255 {
                KillTimer(hwnd, 1);
            }
            0
        }
        WM_DESTROY => {
            MENU_HWND.store(0, Ordering::Relaxed);
            0
        }
        WM_NCDESTROY => {
            KillTimer(hwnd, 1);
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
    let m = ui::s(16.0);
    s.c.clear();
    if let Some(g) = gp::Gfx::from_dc(s.c.dc) {
        gp::setup(g.raw);
        gp::shadow(g.raw, m, m, s.w, (s.c.h as f32) - m * 2.0, ui::s(14.0), 0.6);
    }
    let (cw, ch) = (s.c.w as usize, s.c.h as usize);
    let (bw, bh) = (s.bw, s.bh);
    gp::over(s.c.px(), cw, ch, &s.back, bw, bh, m as usize, m as usize);
    let g = match gp::Gfx::from_dc(s.c.dc) {
        Some(g) => g,
        None => return,
    };
    gp::setup(g.raw);
    gp::stroke_round(g.raw, m + 0.5, m + 0.5, s.w - 1.0, (s.c.h as f32) - m * 2.0 - 1.0, ui::s(14.0), 0x2AFFFFFF, 1.0);
    let ox = m + ui::s(14.0);
    let header_cy = m + ui::s(22.0);
    icons::logo(g.raw, ox, header_cy - ui::s(11.0), ui::s(22.0), true);
    gp::text(&s.c, "Kerf", (ox + ui::s(30.0)) as i32, header_cy as i32, ui::s(14.0), true, ui::TEXT, DT_LEFT | DT_VCENTER);
    gp::text(&s.c, &format!("v{}", env!("CARGO_PKG_VERSION")), (m + s.w - ui::s(14.0)) as i32, header_cy as i32, ui::s(11.0), false, ui::TEXT_3, DT_RIGHT | DT_VCENTER);
    gp::fill_rect(g.raw, ox, m + ui::s(40.0), s.w - ui::s(28.0), 1.0, 0x1FFFFFFF);
    for it in &s.items {
        if let Some(e) = it.entry {
            let en = &s.entries[e];
            let hovered = s.hover == e as i32;
            let (x, y, w, h) = (m + ui::s(6.0), m + it.y, s.w - ui::s(12.0), it.h);
            if hovered {
                gp::fill_round(g.raw, x, y, w, h, ui::s(8.0), if en.danger { ui::argb(0x33, ui::DANGER) } else { 0x1FFFFFFF });
            }
            let cy = y + h / 2.0;
            let col = if en.danger { ui::mix(ui::TEXT_3, ui::DANGER, if hovered { 1.0 } else { 0.62 }) } else { ui::mix(ui::TEXT_2, ui::TEXT, if hovered { 1.0 } else { 0.0 }) };
            icons::menu_icon(g.raw, en.icon, x + ui::s(18.0), cy, col);
            gp::text(&s.c, en.label, (x + ui::s(38.0)) as i32, cy as i32, ui::s(13.0), false, if hovered || en.danger { col } else { ui::TEXT }, DT_LEFT | DT_VCENTER);
            if !en.key.is_empty() {
                let (tw, _) = gp::measure(&en.key, ui::s(11.0), true);
                let kw = tw as f32 + ui::s(14.0);
                wg::keycap(&s.c, g.raw, x + w - ui::s(10.0) - kw, cy, &en.key, false);
            }
        } else {
            gp::fill_rect(g.raw, ox, m + it.y, s.w - ui::s(28.0), 1.0, 0x1FFFFFFF);
        }
    }
    drop(g);
    s.c.present(hwnd, s.x, s.y, s.alpha);
}
