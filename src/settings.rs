use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;

use crate::config::{self, Bind};
use crate::gfx as gp;
use crate::icons;
use crate::ui;
use crate::widgets as wg;
use crate::win::*;

static SET_HWND: AtomicUsize = AtomicUsize::new(0);
static INIT: Once = Once::new();

pub const WM_REREGISTER: u32 = WM_APP + 3;

const W: f32 = 900.0;
const H: f32 = 660.0;
const ROW: f32 = 46.0;
const HEADER: f32 = 40.0;

#[derive(Clone, Copy, PartialEq)]
enum Row {
    Bind(usize),
    Toggle(usize),
    Seg,
    Dim,
    Format,
    Folder,
    Reset,
    Close,
}

struct Card {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    title: &'static str,
    rows: usize,
}

struct Layout {
    cards: Vec<Card>,
    preview: (f32, f32, f32, f32),
    caption_y: f32,
    status_y: f32,
}

struct Hit {
    r: (f32, f32, f32, f32),
    row: Row,
    t: f32,
}

struct St {
    c: gp::Canvas,
    back: Vec<u8>,
    hits: Vec<Hit>,
    hover: i32,
    pressed: i32,
    rebind: Option<usize>,
    err: Option<String>,
    dragging: bool,
    drag_off: (i32, i32),
    alpha: u8,
    tog: [f32; 4],
    close_t: f32,
    running: bool,
    x: i32,
    y: i32,
}

pub fn core_hwnd() -> HWND {
    crate::CORE.load(Ordering::Relaxed) as HWND
}

pub fn hwnd() -> HWND {
    SET_HWND.load(Ordering::Relaxed) as HWND
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

fn layout() -> Layout {
    let (lw, rw) = (ui::s(420.0), ui::s(428.0));
    let (left, right) = (ui::s(20.0), ui::s(452.0));
    let rh = ui::s(ROW);
    let hd = ui::s(HEADER);
    let pb = ui::s(10.0);
    Layout {
        cards: vec![
            Card { x: left, y: ui::s(60.0), w: lw, h: hd + 3.0 * rh + pb, title: "CAPTURE", rows: 3 },
            Card { x: left, y: ui::s(258.0), w: lw, h: hd + 2.0 * rh + pb, title: "SYSTEM", rows: 2 },
            Card { x: left, y: ui::s(410.0), w: lw, h: hd + 3.0 * rh + pb, title: "INTERFACE", rows: 3 },
            Card { x: right, y: ui::s(330.0), w: rw, h: hd + 3.0 * rh + pb, title: "OUTPUT", rows: 3 },
        ],
        preview: (right + ui::s(12.0), ui::s(72.0), rw - ui::s(24.0), ui::s(212.0)),
        caption_y: ui::s(304.0),
        status_y: ui::s(604.0),
    }
}

fn build_hits(l: &Layout) -> Vec<Hit> {
    let mut v: Vec<Hit> = Vec::new();
    let close_s = ui::s(28.0);
    v.push(Hit { r: (ui::s(W) - ui::s(44.0), ui::s(10.0), close_s, close_s), row: Row::Close, t: 0.0 });
    for (ci, c) in l.cards.iter().enumerate() {
        for i in 0..c.rows {
            let row = match (ci, i) {
                (0, i) => Row::Bind(i),
                (1, 0) => Row::Toggle(0),
                (1, 1) => Row::Toggle(1),
                (2, 0) => Row::Seg,
                (2, 1) => Row::Toggle(2),
                (2, 2) => Row::Dim,
                (3, 0) => Row::Format,
                (3, 1) => Row::Folder,
                _ => Row::Toggle(3),
            };
            v.push(Hit { r: (c.x + ui::s(8.0), c.y + ui::s(HEADER) + i as f32 * ui::s(ROW), c.w - ui::s(16.0), ui::s(ROW)), row, t: 0.0 });
        }
    }
    let c = &l.cards[0];
    v.push(Hit { r: (c.x + c.w - ui::s(96.0), c.y + ui::s(8.0), ui::s(88.0), ui::s(24.0)), row: Row::Reset, t: 0.0 });
    v
}

fn row_text(row: Row) -> (&'static str, &'static str) {
    match row {
        Row::Bind(0) => ("Region capture", "Drag an area or click a window"),
        Row::Bind(1) => ("Fullscreen", "Copy the whole screen to clipboard"),
        Row::Bind(2) => ("Repeat region", "Re-capture the last selected area"),
        Row::Toggle(0) => ("Start with Windows", "Launch quietly into the tray"),
        Row::Toggle(1) => ("Sound", "Play a short sound after each capture"),
        Row::Toggle(2) => ("Loupe", "Magnifier and color sample near the cursor"),
        Row::Toggle(3) => ("Ask for file name", "Show the save dialog instead of saving directly"),
        Row::Seg => ("Toolbar", "Where the capture toolbar appears"),
        Row::Dim => ("Dimming", "Darkening outside the selection"),
        Row::Format => ("Format", "Image format for saved screenshots"),
        Row::Folder => ("Save folder", "Where screenshots are saved"),
        _ => ("", ""),
    }
}

fn current_bind(idx: usize) -> Bind {
    config::get().binds()[idx]
}

unsafe fn activate(hwnd: HWND) {
    ShowWindow(hwnd, SW_SHOW);
    if SetForegroundWindow(hwnd) == 0 {
        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
        SetForegroundWindow(hwnd);
        SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
    }
    SetFocus(hwnd);
}

pub unsafe fn open() {
    let old = SET_HWND.load(Ordering::Relaxed);
    if old != 0 {
        activate(old as HWND);
        return;
    }
    INIT.call_once(|| {
        let cls_name = w("KerfSettings");
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
    let (sw, sh) = (ui::si(W), ui::si(H));
    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    let wa = ui::work_area(pt.x, pt.y);
    let x = wa.left + ((wa.right - wa.left) - sw) / 2;
    let y = wa.top + ((wa.bottom - wa.top) - sh) / 2;
    let shot = crate::capture::Shot::rect(x, y, sw, sh);
    let back = gp::acrylic(shot.c.bits, sw, sh, (0, 0, sw, sh), ui::s(14.0), ui::BG, 0.82, ui::si(20.0) as usize).0;
    let cfg = config::get();
    if cfg.autostart {
        config::apply_autostart(true);
    }
    let lay = layout();
    let hits = build_hits(&lay);
    let state = Box::new(St {
        c: gp::Canvas::new(sw, sh),
        back,
        hits,
        hover: -1,
        pressed: -1,
        rebind: None,
        err: None,
        dragging: false,
        drag_off: (0, 0),
        alpha: 0,
        tog: [if cfg.autostart { 1.0 } else { 0.0 }, if cfg.sound { 1.0 } else { 0.0 }, if cfg.loupe { 1.0 } else { 0.0 }, if cfg.ask_name { 1.0 } else { 0.0 }],
        close_t: 0.0,
        running: false,
        x,
        y,
    });
    let lp = Box::into_raw(state);
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOOLWINDOW,
        w("KerfSettings").as_ptr(),
        w("").as_ptr(),
        WS_POPUP,
        x,
        y,
        sw,
        sh,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        GetModuleHandleW(std::ptr::null_mut()),
        lp as *const std::ffi::c_void,
    );
    if hwnd.is_null() {
        drop(Box::from_raw(lp));
        return;
    }
    SET_HWND.store(hwnd as usize, Ordering::Relaxed);
    compose(hwnd);
    activate(hwnd);
    wake(hwnd);
}

unsafe fn state_ptr(hwnd: HWND) -> *mut St {
    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut St
}

unsafe fn wake(hwnd: HWND) {
    let st = state_ptr(hwnd);
    if st.is_null() || (*st).running {
        return;
    }
    (*st).running = true;
    SetTimer(hwnd, 3, 16, None);
}

fn hit_index(hits: &[Hit], x: f32, y: f32) -> i32 {
    for (i, h) in hits.iter().enumerate() {
        let (rx, ry, rw, rh) = h.r;
        if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
            return i as i32;
        }
    }
    -1
}

unsafe fn pick_folder(hwnd: HWND) {
    let title = w("Choose the folder for screenshots");
    let mut bi: BROWSEINFOW = std::mem::zeroed();
    bi.hwndOwner = hwnd;
    bi.lpszTitle = title.as_ptr();
    bi.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;
    let pidl = SHBrowseForFolderW(&bi);
    if !pidl.is_null() {
        let mut buf = [0u16; 1024];
        if SHGetPathFromIDListW(pidl, buf.as_mut_ptr()) != 0 {
            let n = buf.iter().position(|&c| c == 0).unwrap_or(0);
            let p = String::from_utf16_lossy(&buf[..n]);
            if !p.is_empty() {
                config::update(|c| c.save_dir = Some(p));
            }
        }
        CoTaskMemFree(pidl as *const std::ffi::c_void);
    }
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
        WM_SETCURSOR => {
            let (x, y) = client_mouse(hwnd);
            let h = hit_index(&(*st).hits, x as f32, y as f32);
            let c = if y < ui::si(48.0) || h >= 0 { IDC_HAND } else { IDC_ARROW };
            SetCursor(LoadCursorW(std::ptr::null_mut(), c));
            TRUE as LRESULT
        }
        WM_MOUSEMOVE => {
            let (x, y) = (lp_x(lp), lp_y(lp));
            if (*st).dragging {
                let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetWindowRect(hwnd, &mut r);
                let nx = r.left + x - (*st).drag_off.0;
                let ny = r.top + y - (*st).drag_off.1;
                (*st).x = nx;
                (*st).y = ny;
                SetWindowPos(hwnd, HWND_TOP, nx, ny, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
                present(hwnd);
                return 0;
            }
            let h = hit_index(&(*st).hits, x as f32, y as f32);
            if h != (*st).hover {
                (*st).hover = h;
                wake(hwnd);
            }
            0
        }
        WM_MOUSELEAVE => {
            (*st).hover = -1;
            wake(hwnd);
            0
        }
        WM_LBUTTONDOWN => {
            let (x, y) = (lp_x(lp), lp_y(lp));
            let h = hit_index(&(*st).hits, x as f32, y as f32);
            if h < 0 && y < ui::si(48.0) {
                (*st).dragging = true;
                (*st).drag_off = (x, y);
                SetCapture(hwnd);
                return 0;
            }
            (*st).pressed = h;
            0
        }
        WM_LBUTTONUP => {
            if (*st).dragging {
                (*st).dragging = false;
                ReleaseCapture();
                return 0;
            }
            let h = (*st).pressed;
            (*st).pressed = -1;
            if h >= 0 && h == (*st).hover {
                on_click(hwnd, &mut *st, h as usize, lp_x(lp));
            }
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            if (*st).rebind.is_some() {
                on_rebind_key(hwnd, &mut *st, wp as u32);
            } else if wp as u32 == VK_ESCAPE as u32 {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_CHAR => 0,
        WM_KILLFOCUS => {
            if (*st).rebind.is_some() {
                (*st).rebind = None;
                compose(hwnd);
            }
            0
        }
        WM_TIMER => {
            tick(hwnd);
            0
        }
        WM_DESTROY => {
            SET_HWND.store(0, Ordering::Relaxed);
            0
        }
        WM_NCDESTROY => {
            KillTimer(hwnd, 3);
            let _ = Box::from_raw(st);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn client_mouse(hwnd: HWND) -> (i32, i32) {
    let mut p = POINT { x: 0, y: 0 };
    GetCursorPos(&mut p);
    ScreenToClient(hwnd, &mut p);
    (p.x, p.y)
}

unsafe fn on_click(hwnd: HWND, st: &mut St, idx: usize, mx: i32) {
    let row = st.hits[idx].row;
    match row {
        Row::Close => {
            DestroyWindow(hwnd);
            return;
        }
        Row::Bind(i) => {
            st.rebind = if st.rebind == Some(i) { None } else { Some(i) };
            st.err = None;
            SetForegroundWindow(hwnd);
            SetFocus(hwnd);
        }
        Row::Toggle(i) => match i {
            0 => {
                let on = !config::get().autostart;
                config::update(|c| c.autostart = on);
                config::apply_autostart(on);
            }
            1 => config::update(|c| c.sound = !c.sound),
            2 => config::update(|c| c.loupe = !c.loupe),
            _ => config::update(|c| c.ask_name = !c.ask_name),
        },
        Row::Seg => {
            let right = st.hits[idx].r.0 + st.hits[idx].r.2 - ui::s(20.0);
            let w = unsafe { wg::segmented_w(&["Bottom", "Top"]) };
            let sel = seg_pick(right - w, mx, &["Bottom", "Top"]);
            if sel == 0 {
                config::update(|c| c.toolbar_top = false);
            } else if sel == 1 {
                config::update(|c| c.toolbar_top = true);
            }
        }
        Row::Dim => {
            let right = st.hits[idx].r.0 + st.hits[idx].r.2 - ui::s(20.0);
            let opts = ["Light", "Normal", "Dark"];
            let w = unsafe { wg::segmented_w(&opts) };
            let sel = seg_pick(right - w, mx, &opts);
            if sel >= 0 {
                config::update(|c| c.dim = sel as u8);
            }
        }
        Row::Format => {
            let right = st.hits[idx].r.0 + st.hits[idx].r.2 - ui::s(20.0);
            let opts = ["PNG", "JPG"];
            let w = unsafe { wg::segmented_w(&opts) };
            let sel = seg_pick(right - w, mx, &opts);
            if sel >= 0 {
                config::update(|c| c.jpg = sel == 1);
            }
        }
        Row::Folder => pick_folder(hwnd),
        Row::Reset => {
            config::update(|c| {
                let d = config::Config::default();
                c.region = d.region;
                c.fullscreen = d.fullscreen;
                c.repeat = d.repeat;
            });
            st.rebind = None;
            st.err = None;
            PostMessageW(core_hwnd(), WM_REREGISTER, 0, 0);
        }
    }
    compose(hwnd);
}

unsafe fn seg_pick(x: f32, mx: i32, opts: &[&str]) -> i32 {
    let font = ui::s(13.0);
    let mut cx = x + ui::s(3.0);
    for (i, o) in opts.iter().enumerate() {
        let ow = gp::measure(o, font, false).0 as f32 + ui::s(22.0);
        if (mx as f32) >= cx && (mx as f32) < cx + ow {
            return i as i32;
        }
        cx += ow;
    }
    -1
}

unsafe fn on_rebind_key(hwnd: HWND, st: &mut St, vk: u32) {
    let idx = match st.rebind {
        Some(i) => i,
        None => return,
    };
    if vk == VK_ESCAPE as u32 {
        st.rebind = None;
        st.err = None;
        compose(hwnd);
        return;
    }
    if matches!(vk, 16 | 17 | 18 | 91 | 92) {
        compose(hwnd);
        return;
    }
    let bind = Bind {
        ctrl: vk_down(VK_CONTROL),
        shift: vk_down(VK_SHIFT),
        alt: vk_down(VK_MENU),
        win: vk_down(VK_LWIN) || vk_down(VK_RWIN),
        vk,
    };
    let binds = config::get().binds();
    if let Some((j, _)) = binds.iter().enumerate().find(|(j, b)| *j != idx && b.mods() == bind.mods() && b.vk == bind.vk) {
        let (t, _) = row_text(Row::Bind(j));
        st.err = Some(format!("Already used for \"{}\"", t));
        compose(hwnd);
        return;
    }
    if RegisterHotKey(std::ptr::null_mut(), 7777, bind.mods(), bind.vk) == 0 {
        st.err = Some("This combination is taken by another app".to_string());
        compose(hwnd);
        return;
    }
    UnregisterHotKey(std::ptr::null_mut(), 7777);
    let old = current_bind(idx);
    if old != bind {
        config::update(|c| match idx {
            0 => c.region = bind,
            1 => c.fullscreen = bind,
            _ => c.repeat = bind,
        });
        PostMessageW(core_hwnd(), WM_REREGISTER, 0, 0);
    }
    st.rebind = None;
    st.err = None;
    compose(hwnd);
}

fn present(hwnd: HWND) {
    unsafe {
        let st = state_ptr(hwnd);
        if st.is_null() {
            return;
        }
        let (x, y, a) = ((*st).x, (*st).y, (*st).alpha);
        (*st).c.present(hwnd, x, y, a);
    }
}

unsafe fn tick(hwnd: HWND) {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return;
    }
    let s = &mut *st;
    let mut busy = false;
    if s.alpha < 255 {
        s.alpha = (s.alpha as u32 + 22).min(255) as u8;
        busy = true;
    }
    let cfg = config::get();
    let targets = [
        if cfg.autostart { 1.0 } else { 0.0 },
        if cfg.sound { 1.0 } else { 0.0 },
        if cfg.loupe { 1.0 } else { 0.0 },
        if cfg.ask_name { 1.0 } else { 0.0 },
    ];
    for i in 0..4 {
        if (s.tog[i] - targets[i]).abs() > 0.01 {
            s.tog[i] += (targets[i] - s.tog[i]) * 0.3;
            if (s.tog[i] - targets[i]).abs() < 0.03 {
                s.tog[i] = targets[i];
            }
            busy = true;
        } else {
            s.tog[i] = targets[i];
        }
    }
    let hover = s.hover;
    for (i, h) in s.hits.iter_mut().enumerate() {
        let target = if hover == i as i32 { 1.0 } else { 0.0 };
        if (h.t - target).abs() > 0.015 {
            h.t += (target - h.t) * 0.3;
            if (h.t - target).abs() < 0.03 {
                h.t = target;
            }
            busy = true;
        } else {
            h.t = target;
        }
    }
    let close_target = if hover == 0 { 1.0 } else { 0.0 };
    if (s.close_t - close_target).abs() > 0.015 {
        s.close_t += (close_target - s.close_t) * 0.3;
        busy = true;
    } else {
        s.close_t = close_target;
    }
    if !busy {
        KillTimer(hwnd, 3);
        s.running = false;
    }
    compose(hwnd);
}

unsafe fn compose(hwnd: HWND) {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return;
    }
    let s = &mut *st;
    let lay = layout();
    let cfg = config::get();
    s.c.px().copy_from_slice(&s.back);
    let g = match gp::Gfx::from_dc(s.c.dc) {
        Some(g) => g,
        None => return,
    };
    gp::setup(g.raw);
    let (wf, hf) = (ui::s(W), ui::s(H));
    gp::stroke_round(g.raw, 0.5, 0.5, wf - 1.0, hf - 1.0, ui::s(14.0), 0x33FFFFFF, 1.0);
    icons::logo(g.raw, ui::s(20.0), ui::s(12.0), ui::s(24.0), true);
    gp::text(&s.c, "Kerf", ui::si(54.0), ui::si(24.0), ui::s(15.0), true, ui::TEXT, DT_LEFT | DT_VCENTER);
    gp::text(&s.c, "Settings", ui::si(100.0), ui::si(24.0), ui::s(13.0), false, ui::TEXT_3, DT_LEFT | DT_VCENTER);
    let (cx, cy) = (ui::s(W) - ui::s(30.0), ui::s(24.0));
    if s.close_t > 0.01 {
        gp::fill_round(g.raw, cx - ui::s(14.0), cy - ui::s(14.0), ui::s(28.0), ui::s(28.0), ui::s(8.0), ui::argb((0x33 as f32 * s.close_t) as u32, ui::DANGER));
    }
    icons::close_icon(g.raw, cx, cy, ui::s(10.0), ui::mix(ui::TEXT_2, ui::DANGER, s.close_t));
    for (ci, c) in lay.cards.iter().enumerate() {
        gp::fill_round(g.raw, c.x, c.y, c.w, c.h, ui::s(12.0), 0x0FFFFFFF);
        gp::stroke_round(g.raw, c.x + 0.5, c.y + 0.5, c.w - 1.0, c.h - 1.0, ui::s(12.0), 0x1AFFFFFF, 1.0);
        gp::text(&s.c, c.title, (c.x + ui::s(20.0)) as i32, (c.y + ui::s(21.0)) as i32, ui::s(11.0), true, ui::TEXT_3, DT_LEFT | DT_VCENTER);
        if ci == 0 {
            let reset_t = s.hits.iter().find(|h| h.row == Row::Reset).map(|h| h.t).unwrap_or(0.0);
            gp::text(&s.c, "Reset", (c.x + c.w - ui::s(44.0)) as i32, (c.y + ui::s(21.0)) as i32, ui::s(12.0), false, ui::mix(ui::TEXT_3, ui::TEXT, reset_t), DT_CENTER | DT_VCENTER);
        }
    }
    for h in s.hits.iter() {
        let (rx, ry, rw, rh) = h.r;
        let row = h.row;
        match row {
            Row::Close | Row::Reset => {}
            _ => {
                if h.t > 0.01 {
                    gp::fill_round(g.raw, rx, ry + ui::s(3.0), rw, rh - ui::s(6.0), ui::s(8.0), ui::argb((0x0A as f32 * h.t) as u32, 0xFFFFFF));
                }
                let (title, sub) = row_text(row);
                let tx = rx + ui::s(12.0);
                gp::text(&s.c, title, tx as i32, (ry + ui::s(15.0)) as i32, ui::s(14.0), true, ui::TEXT, DT_LEFT | DT_VCENTER);
                let err = match (row, &s.err, s.rebind) {
                    (Row::Bind(b), Some(e), Some(rb)) if b == rb => Some(e.clone()),
                    _ => None,
                };
                let folder = if row == Row::Folder { Some(cfg.save_dir().to_string_lossy().to_string()) } else { None };
                let (scol, stxt) = match (&folder, &err) {
                    (Some(p), _) => (ui::TEXT_2, p.as_str()),
                    (None, Some(e)) => (ui::DANGER, e.as_str()),
                    _ => (ui::TEXT_3, sub),
                };
                let smax = if folder.is_some() { (rw - ui::s(12.0) - ui::s(126.0)).max(ui::s(40.0)) as i32 } else { (rw - ui::s(24.0)).max(ui::s(40.0)) as i32 };
                gp::text(&s.c, &gp::fit(stxt, smax, ui::s(12.0), false), tx as i32, (ry + ui::s(33.0)) as i32, ui::s(12.0), false, scol, DT_LEFT | DT_VCENTER);
                let right = rx + rw - ui::s(20.0);
                let cy = ry + rh / 2.0;
                match row {
                    Row::Bind(b) => draw_bind(&s.c, g.raw, right, cy, b, s.rebind == Some(b)),
                    Row::Toggle(t) => {
                        let on = [cfg.autostart, cfg.sound, cfg.loupe, cfg.ask_name][t];
                        wg::toggle(&s.c, g.raw, right - ui::s(40.0), cy - ui::s(11.0), on, s.tog[t], h.t);
                    }
                    Row::Seg => {
                        wg::segmented(&s.c, g.raw, right - wg::segmented_w(&["Bottom", "Top"]), cy - ui::s(15.0), &["Bottom", "Top"], if cfg.toolbar_top { 1 } else { 0 }, -1);
                    }
                    Row::Dim => {
                        let opts = ["Light", "Normal", "Dark"];
                        wg::segmented(&s.c, g.raw, right - wg::segmented_w(&opts), cy - ui::s(15.0), &opts, (cfg.dim as usize).min(2), -1);
                    }
                    Row::Format => {
                        let opts = ["PNG", "JPG"];
                        wg::segmented(&s.c, g.raw, right - wg::segmented_w(&opts), cy - ui::s(15.0), &opts, if cfg.jpg { 1 } else { 0 }, -1);
                    }
                    Row::Folder => {
                        let btn_w = ui::s(86.0);
                        let bx = right - btn_w;
                        gp::fill_round(g.raw, bx, cy - ui::s(13.0), btn_w, ui::s(26.0), ui::s(7.0), ui::argb(0x1C as u32 + (0x14 as f32 * h.t) as u32, 0xFFFFFF));
                        gp::stroke_round(g.raw, bx + 0.5, cy - ui::s(12.5), btn_w - 1.0, ui::s(25.0), ui::s(7.0), 0x2EFFFFFF, 1.0);
                        gp::text(&s.c, "Change\u{2026}", (bx + btn_w / 2.0) as i32, cy as i32, ui::s(12.0), false, ui::TEXT, DT_CENTER | DT_VCENTER);
                    }
                    _ => {}
                }
            }
        }
    }
    let (px, py, pw, ph) = lay.preview;
    gp::fill_round(g.raw, px - ui::s(12.0), py - ui::s(12.0), pw + ui::s(24.0), ph + ui::s(24.0), ui::s(12.0), 0x0FFFFFFF);
    gp::stroke_round(g.raw, px - ui::s(11.5), py - ui::s(11.5), pw + ui::s(23.0), ph + ui::s(23.0), ui::s(12.0), 0x1AFFFFFF, 1.0);
    draw_preview(&s.c, g.raw, px, py, pw, ph, &cfg);
    gp::text(&s.c, "Live preview", (px) as i32, (py - ui::s(24.0)) as i32, ui::s(11.0), true, ui::TEXT_3, DT_LEFT | DT_VCENTER);
    gp::text(&s.c, "Region capture  \u{00B7}  frozen overlay  \u{00B7}  toolbar", (px) as i32, lay.caption_y as i32, ui::s(12.0), false, ui::TEXT_3, DT_LEFT | DT_VCENTER);
    gp::fill_rect(g.raw, 0.0, lay.status_y, wf, hf - lay.status_y, 0x59000000);
    gp::fill_rect(g.raw, 0.0, lay.status_y, wf, 1.0, 0x1FFFFFFF);
    let scy = lay.status_y + (hf - lay.status_y) / 2.0;
    let mut sx = ui::s(20.0);
    let label = cfg.region.label();
    sx += wg::keycap(&s.c, g.raw, sx, scy, &label, false) + ui::s(10.0);
    gp::text(&s.c, "Start capture", sx as i32, scy as i32, ui::s(12.0), false, ui::TEXT_3, DT_LEFT | DT_VCENTER);
    gp::text(&s.c, &format!("v{}  \u{00B7}  MIT License", env!("CARGO_PKG_VERSION")), (wf - ui::s(20.0)) as i32, scy as i32, ui::s(12.0), false, ui::TEXT_3, DT_RIGHT | DT_VCENTER);
    drop(g);
    let (x, y, a) = (s.x, s.y, s.alpha);
    s.c.present(hwnd, x, y, a);
}

unsafe fn draw_bind(c: &gp::Canvas, g: *mut GpGraphics, right: f32, cy: f32, idx: usize, active: bool) {
    let label = if active {
        let mut mods: Vec<&str> = Vec::new();
        if vk_down(VK_CONTROL) {
            mods.push("Ctrl");
        }
        if vk_down(VK_SHIFT) {
            mods.push("Shift");
        }
        if vk_down(VK_MENU) {
            mods.push("Alt");
        }
        if vk_down(VK_LWIN) || vk_down(VK_RWIN) {
            mods.push("Win");
        }
        if mods.is_empty() {
            "Press keys\u{2026}".to_string()
        } else {
            format!("{} + \u{2026}", mods.join(" + "))
        }
    } else {
        current_bind(idx).label()
    };
    let (tw, _) = gp::measure(&label, ui::s(13.0), true);
    let (w, h) = (tw as f32 + ui::s(24.0), ui::s(32.0));
    let x = right - w;
    gp::fill_round(g, x, cy - h / 2.0, w, h, ui::s(8.0), if active { ui::argb(0x26, ui::ACCENT) } else { 0x1FFFFFFF });
    gp::stroke_round(g, x + 0.5, cy - h / 2.0 + 0.5, w - 1.0, h - 1.0, ui::s(8.0), if active { ui::argb(0xE6, ui::ACCENT) } else { 0x2EFFFFFF }, if active { 1.5 } else { 1.0 });
    gp::text(c, &label, (x + w / 2.0) as i32, cy as i32, ui::s(13.0), true, if active { ui::TEXT } else { ui::TEXT_2 }, DT_CENTER | DT_VCENTER);
}

unsafe fn draw_preview(c: &gp::Canvas, g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, cfg: &config::Config) {
    let k = ui::scale();
    let r = ui::s(10.0);
    let path = gp::round_path(x, y, w, h, r);
    let rect = RectF { X: x, Y: y, Width: w, Height: h };
    let mut br: *mut GpLineGradient = std::ptr::null_mut();
    GdipCreateLineBrushFromRect(&rect, 0xFF2E3444, 0xFF161920, LinearGradientModeVertical, WrapModeTile, &mut br);
    if !br.is_null() {
        GdipFillPath(g, br as *mut GpBrush, path);
        GdipDeleteBrush(br as *mut GpBrush);
    }
    GdipDeletePath(path);
    let clip = gp::round_path(x, y, w, h, r);
    GdipSetClipPath(g, clip, CombineModeReplace);
    GdipDeletePath(clip);
    let (wx, wy, ww, wh) = (x + w * 0.16, y + h * 0.13, w * 0.56, h * 0.70);
    gp::shadow(g, wx, wy, ww, wh, 8.0 * k, 0.6);
    gp::fill_round(g, wx, wy, ww, wh, 8.0 * k, 0xFFF5F6F8);
    gp::fill_round(g, wx, wy, ww, 18.0 * k, 8.0 * k, 0xFFE6E9EF);
    gp::fill_rect(g, wx, wy + 9.0 * k, ww, 9.0 * k, 0xFFE6E9EF);
    gp::fill_round(g, wx + 12.0 * k, wy + 6.0 * k, 8.0 * k, 6.0 * k, 1.5 * k, 0xFF3E8BFF);
    gp::fill_round(g, wx + 26.0 * k, wy + 6.5 * k, 40.0 * k, 5.0 * k, 2.5 * k, 0xFFB9BFC9);
    let (ix, iy) = (wx + 16.0 * k, wy + 30.0 * k);
    gp::fill_round(g, ix, iy, ww * 0.42, 10.0 * k, 3.0 * k, 0xFF2A2F3A);
    gp::fill_round(g, ix, iy + 18.0 * k, ww * 0.46, 4.0 * k, 2.0 * k, 0xFFC5CAD3);
    gp::fill_round(g, ix, iy + 27.0 * k, ww * 0.40, 4.0 * k, 2.0 * k, 0xFFC5CAD3);
    gp::fill_round(g, ix, iy + 36.0 * k, ww * 0.44, 4.0 * k, 2.0 * k, 0xFFC5CAD3);
    gp::fill_round(g, ix, iy + 50.0 * k, 52.0 * k, 13.0 * k, 3.5 * k, 0xFF2F7BFF);
    gp::fill_round(g, ix + 60.0 * k, iy + 50.0 * k, 44.0 * k, 13.0 * k, 3.5 * k, 0xFFE3E8F0);
    let (bx, by) = (wx + ww * 0.56, iy);
    gp::fill_round(g, bx, by, ww * 0.38, wh * 0.36, 5.0 * k, 0xFFFFFFFF);
    gp::stroke_round(g, bx + 0.5, by + 0.5, ww * 0.38 - 1.0, wh * 0.36 - 1.0, 5.0 * k, 0xFFDADFE7, 1.0);
    for (i, col) in [0xFFE8EEF9, 0xFFEAF4EE, 0xFFFFF3E0].iter().enumerate() {
        gp::fill_round(g, bx + 8.0 * k, by + wh * 0.36 + 8.0 * k + i as f32 * 16.0 * k, ww * 0.38 - 16.0 * k, 12.0 * k, 3.0 * k, *col);
    }
    let (sx, sy, sw, sh) = (wx, wy, ww, wh);
    let mut frame: *mut GpPath = std::ptr::null_mut();
    GdipCreatePath(FillModeAlternate, &mut frame);
    GdipAddPathRectangle(frame, x, y, w, h);
    let hole = gp::round_path(sx, sy, sw, sh, 8.0 * k);
    GdipAddPathPath(frame, hole, 0);
    GdipDeletePath(hole);
    let scrim = [0x3C, 0x78, 0xB0][(cfg.dim as usize).min(2)];
    let b = gp::brush(scrim << 24);
    GdipFillPath(g, b as *mut GpBrush, frame);
    GdipDeleteBrush(b as *mut GpBrush);
    GdipDeletePath(frame);
    gp::stroke_round(g, sx + 0.5, sy + 0.5, sw - 1.0, sh - 1.0, 8.0 * k, 0xE6FFFFFF, 1.2 * k);
    for (hx, hy) in [(sx, sy), (sx + sw / 2.0, sy), (sx + sw, sy), (sx, sy + sh / 2.0), (sx + sw, sy + sh / 2.0), (sx, sy + sh), (sx + sw / 2.0, sy + sh), (sx + sw, sy + sh)] {
        gp::circle(g, hx, hy, 3.2 * k, 0xFFFFFFFF, true, 1.0);
    }
    if cfg.loupe {
        let lr = 18.5 * k;
        let (lx, ly) = (sx + sw - 31.0 * k, sy + sh - 33.0 * k);
        let src = (13.0 * k) as i32;
        let (ox, oy) = ((lx as i32 - src / 2).clamp(0, c.w - src), (ly as i32 - src / 2).clamp(0, c.h - src));
        let mut buf = vec![0u8; (src * src * 4) as usize];
        for row in 0..src {
            let so = ((oy + row) * c.w + ox) as usize * 4;
            let dof = (row * src) as usize * 4;
            std::ptr::copy_nonoverlapping(c.bits.add(so), buf.as_mut_ptr().add(dof), (src * 4) as usize);
        }
        let lp = gp::round_path(lx - lr, ly - lr, lr * 2.0, lr * 2.0, lr);
        GdipSetClipPath(g, lp, CombineModeReplace);
        GdipDeletePath(lp);
        let mut bmp: *mut GpBitmap = std::ptr::null_mut();
        GdipCreateBitmapFromScan0(src, src, src * 4, PXF_32ARGB, buf.as_ptr(), &mut bmp);
        if !bmp.is_null() {
            GdipSetInterpolationMode(g, InterpolationModeNearestNeighbor);
            GdipDrawImageRectRectI(g, bmp as *mut GpImage, (lx - lr) as i32, (ly - lr) as i32, (lr * 2.0) as i32, (lr * 2.0) as i32, 0, 0, src, src, UnitPixel, std::ptr::null(), 0, std::ptr::null_mut());
            GdipSetInterpolationMode(g, InterpolationModeDefault);
            GdipDisposeImage(bmp as *mut GpImage);
        }
        GdipResetClip(g);
        let clip2 = gp::round_path(x, y, w, h, r);
        GdipSetClipPath(g, clip2, CombineModeReplace);
        GdipDeletePath(clip2);
        gp::circle(g, lx, ly, lr - 0.5 * k, 0xCC000000, false, 1.0 * k);
        gp::circle(g, lx, ly, lr + 0.5 * k, 0x99FFFFFF, false, 1.0 * k);
        let (a, b) = (lr * 0.34, 3.0 * k);
        for (col, wd) in [(0x66000000u32, 3.0 * k), (0xF2FFFFFFu32, 1.2 * k)] {
            gp::line(g, lx - a, ly, lx - b, ly, col, wd, true);
            gp::line(g, lx + b, ly, lx + a, ly, col, wd, true);
            gp::line(g, lx, ly - a, lx, ly - b, col, wd, true);
            gp::line(g, lx, ly + b, lx, ly + a, col, wd, true);
        }
        gp::circle(g, lx, ly, 2.0 * k, 0x66000000, true, 1.0);
        gp::circle(g, lx, ly, 1.4 * k, ui::argb(255, ui::ACCENT), true, 1.0);
    }
    let (tw_w, tw_h) = (196.0 * k, 22.0 * k);
    let tx = sx + sw / 2.0 - tw_w / 2.0;
    let ty = if cfg.toolbar_top { sy - tw_h - 6.0 * k - 26.0 * k } else { sy + sh + 6.0 * k };
    let ty = ty.clamp(y + 4.0 * k, y + h - tw_h - 30.0 * k);
    wg::glass(g, tx, ty + 26.0 * k, tw_w, tw_h, 7.0 * k, false);
    for i in 0..7 {
        let bx = tx + 9.0 * k + i as f32 * 16.0 * k;
        if i == 0 {
            gp::fill_round(g, bx - 3.0 * k, ty + 26.0 * k + 3.0 * k, 16.0 * k, 16.0 * k, 4.0 * k, ui::argb(0x40, ui::ACCENT));
        }
        gp::circle(g, bx + 5.0 * k, ty + 26.0 * k + 11.0 * k, 3.0 * k, 0xCCFFFFFF, true, 1.0);
    }
    gp::fill_rect(g, tx + 120.0 * k, ty + 26.0 * k + 6.0 * k, 1.0 * k, 10.0 * k, 0x33FFFFFF);
    for (i, col) in crate::overlay::palette().iter().enumerate() {
        gp::circle(g, tx + 130.0 * k + i as f32 * 9.5 * k, ty + 26.0 * k + 11.0 * k, 3.0 * k, ui::argb(255, *col), true, 1.0);
    }
    let ax = sx + sw / 2.0 - 90.0 * k;
    let ay = if cfg.toolbar_top { ty } else { ty + 26.0 * k + tw_h + 5.0 * k };
    gp::fill_round(g, ax, ay, 180.0 * k, 22.0 * k, 7.0 * k, ui::argb(0xE8, ui::BG));
    gp::fill_round(g, ax + 4.0 * k, ay + 3.0 * k, 60.0 * k, 16.0 * k, 5.0 * k, ui::argb(255, ui::ACCENT));
    gp::fill_round(g, ax + 68.0 * k, ay + 3.0 * k, 50.0 * k, 16.0 * k, 5.0 * k, 0x2AFFFFFF);
    gp::fill_round(g, ax + 122.0 * k, ay + 3.0 * k, 50.0 * k, 16.0 * k, 5.0 * k, 0x2AFFFFFF);
    GdipResetClip(g);
}
