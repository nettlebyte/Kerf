use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;

use crate::capture;
use crate::config;
use crate::gfx as gp;
use crate::shapes::{self, Shape, Tool};
use crate::target::{self, Target};
use crate::toolbar;
use crate::toast::{self, Kind};
use crate::ui;
use crate::widgets as wg;
use crate::win::*;

pub const COPY_MSG: u32 = WM_APP + 1;

static OV_HWND: AtomicUsize = AtomicUsize::new(0);
static INIT: Once = Once::new();

pub const COLOR_NAMES: [&str; 6] = ["White", "Yellow", "Green", "Blue", "Red", "Black"];
const DIMS: [u32; 3] = [196, 156, 112];

pub fn palette() -> [u32; 6] {
    [0xFFFFFF, 0xFFDB4D, 0x5CD695, 0x66B2FF, 0xFF6B6B, 0x181818]
}

pub fn widths() -> [f32; 3] {
    [2.0, 3.0, 5.0]
}

pub fn text_sizes() -> [f32; 3] {
    [14.0, 17.0, 22.0]
}

pub fn mosaic_blocks() -> [f32; 3] {
    [8.0, 14.0, 22.0]
}

struct OvState {
    shot: capture::Shot,
    dim: Vec<u8>,
    comp: gp::Canvas,
    sel: (i32, i32, i32, i32),
    has_sel: bool,
    drag: Option<Drag>,
    moved: bool,
    drawing: Option<Shape>,
    anchor: (f32, f32),
    tool: Tool,
    color: u32,
    color_idx: usize,
    size_idx: usize,
    shapes: Vec<Shape>,
    redo: Vec<Shape>,
    text: Option<TextEdit>,
    tb: HWND,
    alpha: u8,
    mouse: (i32, i32),
    inside: bool,
    loupe: bool,
    caret: bool,
    hover: Option<Target>,
}

enum Drag {
    Sel { ax: i32, ay: i32 },
    Move { ox: i32, oy: i32, sx: i32, sy: i32 },
    Resize { h: usize, orig: (i32, i32, i32, i32) },
}

struct TextEdit {
    x: f32,
    y: f32,
    buf: Vec<u16>,
}

pub unsafe fn start() {
    let old = OV_HWND.load(Ordering::Relaxed);
    if old != 0 {
        DestroyWindow(old as HWND);
    }
    INIT.call_once(|| {
        let cls_name = w("KerfOverlay");
        let cls = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null_mut()),
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_CROSS),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: cls_name.as_ptr(),
        };
        RegisterClassW(&cls);
    });
    let cfg = config::get();
    let shot = capture::Shot::grab();
    let (vx, vy, cw, ch) = (shot.vx, shot.vy, shot.w(), shot.h());
    let comp = gp::Canvas::new(cw, ch);
    let mut dim = vec![0u8; comp.len()];
    dim.copy_from_slice(shot.c.px());
    darken(&mut dim, DIMS[(cfg.dim as usize).min(2)]);
    comp.px().copy_from_slice(&dim);
    let state = Box::new(OvState {
        shot,
        dim,
        comp,
        sel: (0, 0, 0, 0),
        has_sel: false,
        drag: None,
        moved: false,
        drawing: None,
        anchor: (0.0, 0.0),
        tool: Tool::None,
        color: palette()[3],
        color_idx: 3,
        size_idx: 1,
        shapes: Vec::new(),
        redo: Vec::new(),
        text: None,
        tb: std::ptr::null_mut(),
        alpha: 0,
        mouse: (0, 0),
        inside: false,
        loupe: false,
        caret: true,
        hover: None,
    });
    let lp = Box::into_raw(state);
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
        w("KerfOverlay").as_ptr(),
        w("").as_ptr(),
        WS_POPUP,
        vx,
        vy,
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
    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    SetForegroundWindow(hwnd);
    SetFocus(hwnd);
    OV_HWND.store(hwnd as usize, Ordering::Relaxed);
    let st = state_ptr(hwnd);
    (*st).tb = toolbar::create(hwnd);
    SetTimer(hwnd, 1, 16, None);
    compose(hwnd, &mut *st);
}

fn darken(buf: &mut [u8], f: u32) {
    for p in buf.chunks_exact_mut(4) {
        p[0] = ((p[0] as u32 * f) / 255) as u8;
        p[1] = ((p[1] as u32 * f) / 255) as u8;
        p[2] = ((p[2] as u32 * f) / 255) as u8;
        p[3] = 255;
    }
}

unsafe fn state_ptr(hwnd: HWND) -> *mut OvState {
    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OvState
}

unsafe fn client_mouse(hwnd: HWND) -> (i32, i32) {
    let mut p = POINT { x: 0, y: 0 };
    GetCursorPos(&mut p);
    ScreenToClient(hwnd, &mut p);
    (p.x, p.y)
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
            set_cursor_for(&*st, x, y);
            TRUE as LRESULT
        }
        WM_MOUSEACTIVATE => MA_ACTIVATE as LRESULT,
        WM_MOUSEMOVE => {
            track_leave(hwnd);
            on_move(hwnd, &mut *st, lp_x(lp), lp_y(lp));
            0
        }
        WM_MOUSELEAVE => {
            (*st).inside = false;
            (*st).hover = None;
            compose(hwnd, &mut *st);
            0
        }
        WM_LBUTTONDOWN => {
            on_down(hwnd, &mut *st, lp_x(lp), lp_y(lp));
            0
        }
        WM_LBUTTONUP => {
            on_up(hwnd, &mut *st, lp_x(lp), lp_y(lp));
            0
        }
        WM_RBUTTONDOWN => {
            on_right(hwnd, &mut *st);
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            on_key(hwnd, &mut *st, wp as u32);
            0
        }
        WM_CHAR => {
            on_char(hwnd, &mut *st, wp as u16);
            0
        }
        WM_TIMER => {
            if wp == 1 {
                (*st).alpha = (((*st).alpha as u32) + 34).min(255) as u8;
                compose(hwnd, &mut *st);
                if (*st).alpha >= 255 {
                    KillTimer(hwnd, 1);
                }
            } else if wp == 2 {
                (*st).caret = !(*st).caret;
                compose(hwnd, &mut *st);
            }
            0
        }
        WM_PAINT => {
            compose(hwnd, &mut *st);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        COPY_MSG => {
            toolbar_action(hwnd, &mut *st, wp as usize);
            0
        }
        WM_DESTROY => {
            OV_HWND.store(0, Ordering::Relaxed);
            0
        }
        WM_NCDESTROY => {
            KillTimer(hwnd, 1);
            KillTimer(hwnd, 2);
            let _ = Box::from_raw(st);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn handles(sel: (i32, i32, i32, i32)) -> [(i32, i32); 8] {
    let (x, y, w, h) = sel;
    let cx = x + w / 2;
    let cy = y + h / 2;
    [
        (x, y),
        (x + w, y),
        (x + w, y + h),
        (x, y + h),
        (cx, y),
        (x + w, cy),
        (cx, y + h),
        (x, cy),
    ]
}

fn handle_hit(sel: (i32, i32, i32, i32), x: i32, y: i32) -> Option<usize> {
    let k = ui::si(10.0);
    let (sx, sy, sw, sh) = sel;
    if sw < k * 3 || sh < k * 3 {
        return None;
    }
    for (i, (hx, hy)) in handles(sel).into_iter().enumerate() {
        if (x - hx).abs() <= k && (y - hy).abs() <= k {
            return Some(i);
        }
    }
    if (x - sx).abs() <= ui::si(4.0) && y > sy + k && y < sy + sh - k {
        return Some(7);
    }
    if (x - sx - sw).abs() <= ui::si(4.0) && y > sy + k && y < sy + sh - k {
        return Some(5);
    }
    if (y - sy).abs() <= ui::si(4.0) && x > sx + k && x < sx + sw - k {
        return Some(4);
    }
    if (y - sy - sh).abs() <= ui::si(4.0) && x > sx + k && x < sx + sw - k {
        return Some(6);
    }
    None
}

fn inside(sel: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= sel.0 && x <= sel.0 + sel.2 && y >= sel.1 && y <= sel.1 + sel.3
}

unsafe fn set_cursor_for(st: &OvState, x: i32, y: i32) {
    let aim = !st.has_sel || (st.tool != Tool::None && st.tool != Tool::Text && inside(st.sel, x, y));
    if aim {
        SetCursor(std::ptr::null_mut());
        return;
    }
    let c = if st.tool == Tool::Text && inside(st.sel, x, y) {
        IDC_IBEAM
    } else if handle_hit(st.sel, x, y).is_some() {
        match handle_hit(st.sel, x, y).unwrap() {
            0 | 2 => IDC_SIZENWSE,
            1 | 3 => IDC_SIZENESW,
            4 | 6 => IDC_SIZENS,
            _ => IDC_SIZEWE,
        }
    } else if inside(st.sel, x, y) {
        IDC_SIZEALL
    } else {
        IDC_ARROW
    };
    SetCursor(LoadCursorW(std::ptr::null_mut(), c));
}

unsafe fn draw_crosshair(g: *mut GpGraphics, x: i32, y: i32) {
    let k = ui::scale();
    let (x, y) = (x as f32, y as f32);
    let gap = 5.0 * k;
    let arm = 12.0 * k;
    for &(col, w) in &[(0x96000000u32, 3.2 * k), (0xF2FFFFFFu32, 1.4 * k)] {
        gp::line(g, x - gap - arm, y, x - gap, y, col, w, true);
        gp::line(g, x + gap, y, x + gap + arm, y, col, w, true);
        gp::line(g, x, y - gap - arm, x, y - gap, col, w, true);
        gp::line(g, x, y + gap, x, y + gap + arm, col, w, true);
    }
    gp::circle(g, x, y, 2.4 * k, 0x96000000, true, 1.0);
    gp::circle(g, x, y, 1.7 * k, ui::argb(255, ui::ACCENT), true, 1.0);
}

unsafe fn on_move(hwnd: HWND, st: &mut OvState, x: i32, y: i32) {
    st.mouse = (x, y);
    st.inside = true;
    match st.drag {
        Some(Drag::Sel { ax, ay }) => {
            if !st.moved && (x - ax).abs() + (y - ay).abs() > 3 {
                st.moved = true;
            }
            if st.moved {
                st.sel = (ax.min(x), ay.min(y), (ax - x).abs(), (ay - y).abs());
            }
        }
        Some(Drag::Move { ox, oy, sx, sy }) => {
            let nx = (sx + x - ox).max(0).min(st.shot.w() - st.sel.2.max(1));
            let ny = (sy + y - oy).max(0).min(st.shot.h() - st.sel.3.max(1));
            st.sel = (nx, ny, st.sel.2, st.sel.3);
        }
        Some(Drag::Resize { h, orig }) => {
            let (mut l, mut t, mut r, mut b) = (orig.0, orig.1, orig.0 + orig.2, orig.1 + orig.3);
            match h {
                0 => {
                    l = x;
                    t = y;
                }
                1 => {
                    r = x;
                    t = y;
                }
                2 => {
                    r = x;
                    b = y;
                }
                3 => {
                    l = x;
                    b = y;
                }
                4 => t = y,
                5 => r = x,
                6 => b = y,
                _ => l = x,
            }
            l = l.max(0).min(st.shot.w());
            r = r.max(0).min(st.shot.w());
            t = t.max(0).min(st.shot.h());
            b = b.max(0).min(st.shot.h());
            st.sel = (l.min(r), t.min(b), (l - r).abs(), (t - b).abs());
        }
        None => {
            if !st.has_sel {
                st.hover = target::window_at(x + st.shot.vx, y + st.shot.vy);
            }
        }
    }
    let anchor = st.anchor;
    if let Some(d) = st.drawing.as_mut() {
        update_shape(d, anchor, x, y);
    }
    compose(hwnd, st);
    if st.has_sel {
        place_toolbar(st);
    }
}

fn update_shape(d: &mut Shape, anchor: (f32, f32), x: i32, y: i32) {
    let (px, py) = (x as f32, y as f32);
    match d {
        Shape::Free { pts, .. } => {
            let last = pts[pts.len() - 1];
            if (last.0 - px).abs() > 0.8 || (last.1 - py).abs() > 0.8 {
                pts.push((px, py));
            }
        }
        Shape::Arrow { b, .. } => *b = (px, py),
        Shape::Rect { r, .. } | Shape::Ellipse { r, .. } | Shape::Mosaic { r, .. } => *r = shapes::norm(anchor, (px, py)),
        Shape::Text { .. } => {}
    }
}

unsafe fn on_down(hwnd: HWND, st: &mut OvState, x: i32, y: i32) {
    SetCapture(hwnd);
    if st.text.is_some() {
        commit_text(st);
    }
    if st.has_sel && inside(st.sel, x, y) {
        if let Some(h) = handle_hit(st.sel, x, y) {
            st.drag = Some(Drag::Resize { h, orig: st.sel });
            return;
        }
        if st.tool == Tool::Text {
            st.text = Some(TextEdit { x: x as f32, y: (y - ui::si(10.0)) as f32, buf: Vec::new() });
            st.caret = true;
            SetTimer(hwnd, 2, 530, None);
            compose(hwnd, st);
            return;
        }
        if st.tool.is_draw() {
            let p = (x as f32, y as f32);
            st.anchor = p;
            let c = st.color;
            let wd = widths()[st.size_idx];
            st.drawing = Some(match st.tool {
                Tool::Pen => Shape::Free { pts: vec![p], color: c, width: wd },
                Tool::Arrow => Shape::Arrow { a: p, b: p, color: c, width: wd },
                Tool::Rect => Shape::Rect { r: (p.0, p.1, 0.0, 0.0), color: c, width: wd },
                Tool::Ellipse => Shape::Ellipse { r: (p.0, p.1, 0.0, 0.0), color: c, width: wd },
                _ => Shape::Mosaic { r: (p.0, p.1, 0.0, 0.0), block: (mosaic_blocks()[st.size_idx] * ui::scale()) as i32 },
            });
            return;
        }
        st.drag = Some(Drag::Move { ox: x, oy: y, sx: st.sel.0, sy: st.sel.1 });
        return;
    }
    st.has_sel = false;
    st.shapes.clear();
    st.redo.clear();
    st.sel = (x, y, 0, 0);
    st.moved = false;
    st.drag = Some(Drag::Sel { ax: x, ay: y });
    st.hover = target::window_at(x + st.shot.vx, y + st.shot.vy);
    toolbar::hide(st.tb);
    compose(hwnd, st);
}

unsafe fn on_up(hwnd: HWND, st: &mut OvState, x: i32, y: i32) {
    ReleaseCapture();
    let (vx, vy) = (st.shot.vx, st.shot.vy);
    match st.drag.take() {
        Some(Drag::Sel { .. }) => {
            if st.moved {
                if st.sel.2 > 3 && st.sel.3 > 3 {
                    freeze(hwnd, st, st.sel);
                } else {
                    st.sel = (0, 0, 0, 0);
                    compose(hwnd, st);
                }
            } else {
                let rect = match &st.hover {
                    Some(t) => (t.rect.0 - vx, t.rect.1 - vy, t.rect.2, t.rect.3),
                    None => {
                        let m = target::monitor_at(x + vx, y + vy);
                        (m.0 - vx, m.1 - vy, m.2, m.3)
                    }
                };
                freeze(hwnd, st, rect);
            }
        }
        _ => {
            if let Some(d) = st.drawing.take() {
                let ok = match &d {
                    Shape::Free { pts, .. } => pts.len() >= 2,
                    Shape::Arrow { a, b, .. } => ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() > 4.0,
                    Shape::Rect { r, .. } | Shape::Ellipse { r, .. } | Shape::Mosaic { r, .. } => r.2 > 4.0 && r.3 > 4.0,
                    Shape::Text { .. } => false,
                };
                if ok {
                    st.shapes.push(d);
                    st.redo.clear();
                }
            }
            sync_toolbar(st);
            compose(hwnd, st);
            place_toolbar(st);
        }
    }
}

unsafe fn freeze(hwnd: HWND, st: &mut OvState, rect: (i32, i32, i32, i32)) {
    let (w, h) = (st.shot.w(), st.shot.h());
    let x = rect.0.clamp(0, w - 1);
    let y = rect.1.clamp(0, h - 1);
    let rw = rect.2.clamp(1, w - x);
    let rh = rect.3.clamp(1, h - y);
    st.sel = (x, y, rw, rh);
    st.has_sel = true;
    st.shapes.clear();
    st.redo.clear();
    st.text = None;
    st.drawing = None;
    st.hover = None;
    KillTimer(hwnd, 2);
    toolbar::set_size(st.tb, rw, rh);
    compose(hwnd, st);
    place_toolbar(st);
    toolbar::show_fade(st.tb);
    sync_toolbar(st);
}

unsafe fn on_right(hwnd: HWND, st: &mut OvState) {
    if st.has_sel {
        st.has_sel = false;
        st.sel = (0, 0, 0, 0);
        st.shapes.clear();
        st.redo.clear();
        st.text = None;
        KillTimer(hwnd, 2);
        toolbar::hide(st.tb);
        compose(hwnd, st);
    } else {
        DestroyWindow(hwnd);
    }
}

unsafe fn undo(st: &mut OvState) {
    if st.text.is_some() {
        st.text = None;
    } else if let Some(s) = st.shapes.pop() {
        st.redo.push(s);
    }
}

unsafe fn redo(st: &mut OvState) {
    if let Some(s) = st.redo.pop() {
        st.shapes.push(s);
    }
}

unsafe fn on_key(hwnd: HWND, st: &mut OvState, vk: u32) {
    const ESC: u32 = VK_ESCAPE as u32;
    const TAB: u32 = VK_TAB as u32;
    const RET: u32 = VK_RETURN as u32;
    const LEFT: u32 = VK_LEFT as u32;
    const RIGHT: u32 = VK_RIGHT as u32;
    const UP: u32 = VK_UP as u32;
    const DOWN: u32 = VK_DOWN as u32;
    let editing = st.text.is_some();
    let ctrl = vk_down(VK_CONTROL);
    let shift = vk_down(VK_SHIFT);
    if editing && !(ctrl && matches!(vk, 0x43 | 0x53 | 0x5A | 0x59 | 0x41)) {
        return;
    }
    match vk {
        ESC => {
            if st.text.is_some() {
                st.text = None;
                KillTimer(hwnd, 2);
                compose(hwnd, st);
            } else {
                DestroyWindow(hwnd);
            }
        }
        TAB => {
            if !st.has_sel && config::get().loupe {
                st.loupe = !st.loupe;
                compose(hwnd, st);
            }
        }
        RET => {
            if st.text.is_some() {
                commit_text(st);
                compose(hwnd, st);
            } else if st.has_sel {
                finish_copy(hwnd, st);
            }
        }
        0x20 => {
            if st.has_sel {
                finish_copy(hwnd, st);
            }
        }
        0x43 if ctrl => finish_copy(hwnd, st),
        0x53 if ctrl => finish_save(hwnd, st),
        0x5A if ctrl => {
            if shift {
                redo(st);
            } else {
                undo(st);
            }
            sync_toolbar(st);
            compose(hwnd, st);
        }
        0x59 if ctrl => {
            redo(st);
            sync_toolbar(st);
            compose(hwnd, st);
        }
        0x41 if ctrl => {
            let (w, h) = (st.shot.w(), st.shot.h());
            freeze(hwnd, st, (0, 0, w, h));
        }
        LEFT | RIGHT | UP | DOWN => {
            if st.has_sel {
                let step = if shift { 10 } else { 1 };
                let (dx, dy) = match vk {
                    LEFT => (-step, 0),
                    RIGHT => (step, 0),
                    UP => (0, -step),
                    _ => (0, step),
                };
                let (w, h) = (st.shot.w(), st.shot.h());
                st.sel.0 = (st.sel.0 + dx).max(0).min(w - st.sel.2);
                st.sel.1 = (st.sel.1 + dy).max(0).min(h - st.sel.3);
                compose(hwnd, st);
                place_toolbar(st);
            }
        }
        _ => {}
    }
}

unsafe fn on_char(hwnd: HWND, st: &mut OvState, ch: u16) {
    if vk_down(VK_CONTROL) {
        return;
    }
    if let Some(t) = st.text.as_mut() {
        match ch {
            8 => {
                t.buf.pop();
            }
            13 => {
                commit_text(st);
                KillTimer(hwnd, 2);
            }
            27 => {
                st.text = None;
                KillTimer(hwnd, 2);
            }
            c if c >= 0x20 => t.buf.push(c),
            _ => {}
        }
        compose(hwnd, st);
    }
}

unsafe fn commit_text(st: &mut OvState) {
    if let Some(t) = st.text.take() {
        let s = String::from_utf16_lossy(&t.buf);
        if !s.trim().is_empty() {
            st.shapes.push(Shape::Text { x: t.x, y: t.y, text: s, color: st.color, size: text_sizes()[st.size_idx] });
            st.redo.clear();
        }
    }
}

unsafe fn sync_toolbar(st: &OvState) {
    if !st.tb.is_null() {
        toolbar::set_state(st.tb, st.tool.idx(), st.color_idx as i32, st.size_idx as i32, st.shapes.len() as i32, st.redo.len() as i32);
    }
}

unsafe fn place_toolbar(st: &OvState) {
    if st.tb.is_null() {
        return;
    }
    if !st.has_sel {
        toolbar::hide(st.tb);
        return;
    }
    let (vx, vy, vw, vh) = (st.shot.vx, st.shot.vy, st.shot.w(), st.shot.h());
    let (tw, th) = toolbar::panel_size();
    let sel = st.sel;
    let gap = ui::si(10.0);
    let below = vy + sel.1 + sel.3 + gap;
    let above = vy + sel.1 - gap - th;
    let mut y = if config::get().toolbar_top { above } else { below };
    if y + th > vy + vh || y < vy {
        y = if y == below { above } else { below };
    }
    y = y.max(vy + ui::si(8.0)).min(vy + vh - th - ui::si(8.0));
    let mut x = vx + sel.0 + sel.2 / 2 - tw / 2;
    x = x.max(vx + ui::si(8.0)).min(vx + vw - tw - ui::si(8.0));
    toolbar::place(st.tb, x, y, st.comp.bits, st.comp.w, st.comp.h, vx, vy);
}

unsafe fn toolbar_action(hwnd: HWND, st: &mut OvState, id: usize) {
    match id {
        0..=6 => {
            let t = Tool::from_idx(id);
            st.tool = if st.tool == t { Tool::None } else { t };
            if st.tool != Tool::Text {
                st.text = None;
            }
        }
        id if (toolbar::COLOR_BASE..=toolbar::COLOR_BASE + 5).contains(&id) => {
            st.color_idx = id - toolbar::COLOR_BASE;
            st.color = palette()[st.color_idx];
        }
        id if (toolbar::SIZE_BASE..=toolbar::SIZE_BASE + 2).contains(&id) => st.size_idx = id - toolbar::SIZE_BASE,
        toolbar::ACT_UNDO => {
            undo(st);
            KillTimer(hwnd, 2);
        }
        toolbar::ACT_REDO => redo(st),
        toolbar::ACT_COPY => {
            finish_copy(hwnd, st);
            return;
        }
        toolbar::ACT_SAVE => {
            finish_save(hwnd, st);
            return;
        }
        toolbar::ACT_CLOSE => {
            DestroyWindow(hwnd);
            return;
        }
        _ => {}
    }
    sync_toolbar(st);
    compose(hwnd, st);
    place_toolbar(st);
}

unsafe fn compose(hwnd: HWND, st: &mut OvState) {
    let (w, h) = (st.shot.w(), st.shot.h());
    std::ptr::copy_nonoverlapping(st.dim.as_ptr(), st.comp.bits, st.dim.len());
    let live = matches!(st.drag, Some(Drag::Sel { .. })) && st.moved;
    let keep = if st.has_sel || live { Some(st.sel) } else { None };
    if let Some(r) = keep {
        if r.2 > 0 && r.3 > 0 {
            gp::copy_rounded(st.comp.bits, w as usize, st.shot.c.bits, r, ui::s(18.0));
        }
    }
    let g = match gp::Gfx::from_dc(st.comp.dc) {
        Some(g) => g,
        None => return,
    };
    gp::setup(g.raw);
    if let Some(r) = keep {
        if r.2 > 0 && r.3 > 0 {
            for s in &st.shapes {
                if let Shape::Mosaic { .. } = s {
                    shapes::draw(st.comp.bits, w as usize, h as usize, g.raw, (0.0, 0.0), Some(r), s);
                }
            }
            let path = gp::round_path(r.0 as f32, r.1 as f32, r.2 as f32, r.3 as f32, ui::s(18.0));
            GdipSetClipPath(g.raw, path, CombineModeReplace);
            GdipDeletePath(path);
            for s in &st.shapes {
                if !matches!(s, Shape::Mosaic { .. }) {
                    shapes::draw(st.comp.bits, w as usize, h as usize, g.raw, (0.0, 0.0), Some(r), s);
                }
            }
            if let Some(d) = &st.drawing {
                shapes::draw(st.comp.bits, w as usize, h as usize, g.raw, (0.0, 0.0), Some(r), d);
            }
            if let Some(t) = &st.text {
                let s = String::from_utf16_lossy(&t.buf);
                let (tw, th) = gp::text_px(st.comp.bits, w as usize, h as usize, &s, t.x as i32, t.y as i32, text_sizes()[st.size_idx] * ui::scale(), false, st.color, DT_LEFT | DT_TOP, Some(r));
                if st.caret {
                    gp::line(g.raw, t.x + tw as f32 + 1.0, t.y, t.x + tw as f32 + 1.0, t.y + th as f32, 0xFF66B2FF, 1.5 * ui::scale(), true);
                }
            }
            GdipResetClip(g.raw);
            draw_selection(&st.comp, g.raw, r, !live && st.has_sel);
        }
    }
    if !st.has_sel {
        if st.drag.is_none() {
            if let Some(t) = &st.hover {
                draw_window_hint(&st.comp, g.raw, st, t);
            }
            if st.inside {
                draw_hud(&st.comp, g.raw, st);
            }
        }
        if st.inside {
            draw_crosshair(g.raw, st.mouse.0, st.mouse.1);
        }
        draw_top_hint(&st.comp, g.raw, w);
    } else if st.tool != Tool::None && st.tool != Tool::Text && st.inside && st.drag.is_none() && st.text.is_none() {
        draw_crosshair(g.raw, st.mouse.0, st.mouse.1);
    }
    drop(g);
    st.comp.present(hwnd, st.shot.vx, st.shot.vy, st.alpha);
}

unsafe fn draw_selection(c: &gp::Canvas, g: *mut GpGraphics, sel: (i32, i32, i32, i32), handles_on: bool) {
    let k = ui::scale();
    let (x, y, w, h) = sel;
    let r = ui::s(18.0);
    let path = gp::round_path(x as f32, y as f32, w as f32, h as f32, r);
    let dark = gp::pen(0x80000000, 2.0 * k, false);
    GdipDrawPath(g, dark, path);
    GdipDeletePen(dark);
    let light = gp::pen(0xE6FFFFFF, 1.0 * k, false);
    GdipDrawPath(g, light, path);
    GdipDeletePen(light);
    GdipDeletePath(path);
    if handles_on {
        for (hx, hy) in handles(sel) {
            gp::circle(g, hx as f32, hy as f32, 5.5 * k, 0x66000000, true, 1.0);
            gp::circle(g, hx as f32, hy as f32, 4.5 * k, 0xFFFFFFFF, true, 1.0);
        }
    } else {
        let (tw, _) = gp::measure(&format!("{} \u{00D7} {}", w, h), ui::s(12.0), true);
        let (pw, ph) = (tw as f32 + ui::s(20.0), ui::s(26.0));
        let bx = (x + w) as f32 - pw;
        let by = if (y + h) as f32 + ui::s(8.0) + ph <= c.h as f32 { (y + h) as f32 + ui::s(8.0) } else { (y + h) as f32 - ph - ui::s(8.0) };
        wg::glass(g, bx, by, pw, ph, ph / 2.0, false);
        gp::text(c, &format!("{} \u{00D7} {}", w, h), (bx + pw / 2.0) as i32, (by + ph / 2.0) as i32, ui::s(12.0), true, ui::TEXT, DT_CENTER | DT_VCENTER);
    }
}

unsafe fn draw_window_hint(c: &gp::Canvas, g: *mut GpGraphics, st: &OvState, t: &Target) {
    let k = ui::scale();
    let (x, y, w, h) = (t.rect.0 - st.shot.vx, t.rect.1 - st.shot.vy, t.rect.2, t.rect.3);
    gp::stroke_round_dash(g, x as f32, y as f32, w as f32, h as f32, ui::s(12.0), 0xB3FFFFFF, 1.5 * k, &[8.0 * k, 4.0 * k]);
    let title = if t.title.is_empty() { "Window".to_string() } else { t.title.clone() };
    let title = gp::fit(&title, ui::si(320.0), ui::s(12.0), false);
    let size = format!("{} \u{00D7} {}", w, h);
    let (tw1, _) = gp::measure(&title, ui::s(12.0), false);
    let (tw2, _) = gp::measure(&size, ui::s(12.0), true);
    let (pw, ph) = (tw1 as f32 + tw2 as f32 + ui::s(40.0), ui::s(26.0));
    let mut bx = (x + w) as f32 - pw;
    let mut by = (y + h) as f32 + ui::s(8.0);
    if by + ph > c.h as f32 {
        by = (y + h) as f32 - ph - ui::s(8.0);
    }
    if by < 0.0 {
        by = y as f32 + ui::s(8.0);
    }
    if bx < 0.0 {
        bx = 0.0;
    }
    wg::glass(g, bx, by, pw, ph, ph / 2.0, false);
    let cy = by + ph / 2.0;
    let mut cx = bx + ui::s(12.0);
    cx += gp::text(c, &title, cx as i32, cy as i32, ui::s(12.0), false, ui::TEXT_2, DT_LEFT | DT_VCENTER).0 as f32 + ui::s(14.0);
    gp::text(c, &size, cx as i32, cy as i32, ui::s(12.0), true, ui::TEXT, DT_LEFT | DT_VCENTER);
}

unsafe fn draw_loupe(g: *mut GpGraphics, st: &OvState, cx: f32, cy: f32, r: f32) {
    let k = ui::scale();
    let (w, h) = (st.shot.w(), st.shot.h());
    let src = (28.0 * k).max(4.0);
    let sx = ((st.mouse.0 as f32 - src / 2.0).clamp(0.0, (w as f32 - src).max(0.0))) as i32;
    let sy = ((st.mouse.1 as f32 - src / 2.0).clamp(0.0, (h as f32 - src).max(0.0))) as i32;
    let mut path: *mut GpPath = std::ptr::null_mut();
    GdipCreatePath(FillModeAlternate, &mut path);
    GdipAddPathEllipse(path, cx - r, cy - r, r * 2.0, r * 2.0);
    GdipSetClipPath(g, path, CombineModeReplace);
    GdipDeletePath(path);
    let mut bmp: *mut GpBitmap = std::ptr::null_mut();
    let off = ((sy * w + sx) * 4) as usize;
    GdipCreateBitmapFromScan0(src as i32, src as i32, w * 4, PXF_32ARGB, st.shot.c.bits.add(off), &mut bmp);
    if !bmp.is_null() {
        GdipSetInterpolationMode(g, InterpolationModeNearestNeighbor);
        GdipDrawImageRectRectI(g, bmp as *mut GpImage, (cx - r) as i32, (cy - r) as i32, (r * 2.0) as i32, (r * 2.0) as i32, 0, 0, src as i32, src as i32, UnitPixel, std::ptr::null(), 0, std::ptr::null_mut());
        GdipSetInterpolationMode(g, InterpolationModeDefault);
        GdipDisposeImage(bmp as *mut GpImage);
    }
    GdipResetClip(g);
    gp::circle(g, cx, cy, r - 0.5 * k, 0xCC000000, false, 1.0 * k);
    gp::circle(g, cx, cy, r + 0.5 * k, 0x99FFFFFF, false, 1.0 * k);
    let (a, b) = (r * 0.34, ui::s(6.0));
    for (col, wd) in [(0x66000000u32, 3.0 * k), (0xF2FFFFFFu32, 1.2 * k)] {
        gp::line(g, cx - a, cy, cx - b, cy, col, wd, true);
        gp::line(g, cx + b, cy, cx + a, cy, col, wd, true);
        gp::line(g, cx, cy - a, cx, cy - b, col, wd, true);
        gp::line(g, cx, cy + b, cx, cy + a, col, wd, true);
    }
    gp::circle(g, cx, cy, 2.0 * k, 0x66000000, true, 1.0);
    gp::circle(g, cx, cy, 1.4 * k, ui::argb(255, ui::ACCENT), true, 1.0);
}

unsafe fn draw_hud(c: &gp::Canvas, g: *mut GpGraphics, st: &OvState) {
    let (mx, my) = st.mouse;
    let (w, h) = (st.shot.w(), st.shot.h());
    let (px, py) = (mx.clamp(0, w - 1), my.clamp(0, h - 1));
    let o = ((py * w + px) * 4) as usize;
    let (b, gr, r) = (*st.shot.c.bits.add(o), *st.shot.c.bits.add(o + 1), *st.shot.c.bits.add(o + 2));
    let hex = format!("#{:02X}{:02X}{:02X}", r, gr, b);
    let pos = format!("{}, {}", mx + st.shot.vx, my + st.shot.vy);
    let f13 = ui::s(13.0);
    let f12 = ui::s(12.0);
    let (pw, _) = gp::measure(&pos, f13, true);
    let (hw, _) = gp::measure(&hex, f13, false);
    let le = crate::config::get().loupe;
    let kw = if le { gp::measure("Tab", ui::s(11.0), true).0 as f32 + ui::s(14.0) } else { 0.0 };
    let lw = if le { gp::measure("Loupe", f12, false).0 as f32 } else { 0.0 };
    let (ph, gap) = (ui::s(32.0), ui::s(12.0));
    let pill_w = ui::s(14.0) + pw as f32 + gap + ui::s(12.0) + gap + hw as f32 + if le { gap + kw + ui::s(6.0) + lw } else { 0.0 } + ui::s(14.0);
    let d = ui::s(112.0);
    let rr = d / 2.0;
    let mut hx;
    let mut hy;
    if st.loupe {
        let mut cx = mx as f32 + ui::s(16.0) + rr;
        let mut cy = my as f32 - ui::s(16.0) - rr;
        if cx + rr > w as f32 {
            cx = mx as f32 - ui::s(16.0) - rr;
        }
        if cy - rr < 0.0 {
            cy = my as f32 + ui::s(16.0) + rr;
        }
        hx = cx - pill_w / 2.0;
        hy = cy + rr + ui::s(8.0);
        if hy + ph > h as f32 {
            hy = cy - rr - ui::s(8.0) - ph;
        }
        draw_loupe(g, st, cx, cy, rr);
    } else {
        hx = mx as f32 + ui::s(20.0);
        hy = my as f32 + ui::s(20.0);
        if hx + pill_w > w as f32 {
            hx = mx as f32 - ui::s(20.0) - pill_w;
        }
        if hy + ph > h as f32 {
            hy = my as f32 - ui::s(20.0) - ph;
        }
    }
    hx = hx.max(ui::s(8.0)).min(w as f32 - pill_w - ui::s(8.0));
    hy = hy.max(ui::s(8.0)).min(h as f32 - ph - ui::s(8.0));
    wg::glass(g, hx, hy, pill_w, ph, ph / 2.0, true);
    let cy = hy + ph / 2.0;
    let mut x = hx + ui::s(14.0);
    x += gp::text(c, &pos, x as i32, cy as i32, f13, true, ui::TEXT, DT_LEFT | DT_VCENTER).0 as f32 + gap;
    let col = (r as u32) << 16 | (gr as u32) << 8 | b as u32;
    gp::circle(g, x + ui::s(6.0), cy, ui::s(7.0), 0x40000000, true, 1.0);
    gp::circle(g, x + ui::s(6.0), cy, ui::s(6.0), ui::argb(255, col), true, 1.0);
    gp::circle(g, x + ui::s(6.0), cy, ui::s(6.0), 0x66FFFFFF, false, 1.0);
    x += ui::s(12.0) + gap;
    x += gp::text(c, &hex, x as i32, cy as i32, f13, false, ui::TEXT_2, DT_LEFT | DT_VCENTER).0 as f32 + gap;
    if le {
        x += wg::keycap(c, g, x, cy, "Tab", false) + ui::s(6.0);
        gp::text(c, "Loupe", x as i32, cy as i32, f12, false, ui::TEXT_3, DT_LEFT | DT_VCENTER);
    }
}

unsafe fn draw_top_hint(c: &gp::Canvas, g: *mut GpGraphics, w: i32) {
    let a = "Drag to select an area";
    let b = "  \u{00B7}  ";
    let c2 = "Click a window to capture it";
    let d = "  \u{00B7}  ";
    let e = "Esc to cancel";
    let f13 = ui::s(13.0);
    let (wa, _) = gp::measure(a, f13, false);
    let (wb, _) = gp::measure(b, f13, false);
    let (wc, _) = gp::measure(c2, f13, false);
    let (wd, _) = gp::measure(d, f13, false);
    let (we, _) = gp::measure(e, f13, false);
    let (pw, ph) = (wa + wb + wc + wd + we + ui::si(32.0), ui::si(30.0));
    let x = (w - pw) as f32 / 2.0;
    let y = ui::s(24.0);
    wg::glass(g, x, y, pw as f32, ph as f32, ph as f32 / 2.0, true);
    let cy = y + ph as f32 / 2.0;
    let mut cx = x + ui::s(16.0);
    cx += gp::text(c, a, cx as i32, cy as i32, f13, false, ui::TEXT, DT_LEFT | DT_VCENTER).0 as f32;
    cx += gp::text(c, b, cx as i32, cy as i32, f13, false, ui::TEXT_3, DT_LEFT | DT_VCENTER).0 as f32;
    cx += gp::text(c, c2, cx as i32, cy as i32, f13, false, ui::TEXT_2, DT_LEFT | DT_VCENTER).0 as f32;
    cx += gp::text(c, d, cx as i32, cy as i32, f13, false, ui::TEXT_3, DT_LEFT | DT_VCENTER).0 as f32;
    gp::text(c, e, cx as i32, cy as i32, f13, false, ui::TEXT_2, DT_LEFT | DT_VCENTER);
}

unsafe fn finish_copy(hwnd: HWND, st: &mut OvState) {
    if !st.has_sel {
        return;
    }
    let sel = st.sel;
    let (vx, vy) = (st.shot.vx, st.shot.vy);
    let screen_rect = (sel.0 + vx, sel.1 + vy, sel.2, sel.3);
    let crop = capture::crop(&st.shot, screen_rect, &st.shapes);
    let dib = capture::dib_bytes(&crop);
    let png = capture::encode(&crop, false).unwrap_or_default();
    let ok = capture::clipboard_put(&dib, &png);
    config::update(|c| c.last = Some([screen_rect.0, screen_rect.1, screen_rect.2, screen_rect.3]));
    let (cx, cy) = (screen_rect.0 + screen_rect.2 / 2, screen_rect.1 + screen_rect.3 / 2);
    DestroyWindow(hwnd);
    if config::get().sound && ok {
        MessageBeep(0);
    }
    toast::show(
        if ok { "Copied to clipboard" } else { "Copy failed" },
        if ok { Kind::Ok } else { Kind::Err },
        cx,
        cy + sel.3 / 2 + ui::si(16.0),
    );
}

unsafe fn finish_save(hwnd: HWND, st: &mut OvState) {
    if !st.has_sel {
        return;
    }
    let sel = st.sel;
    let (vx, vy) = (st.shot.vx, st.shot.vy);
    let screen_rect = (sel.0 + vx, sel.1 + vy, sel.2, sel.3);
    let cfg = config::get();
    let dir = cfg.save_dir();
    let name = format!("Kerf-{}.{}", capture::timestamp(), cfg.ext());
    let mut path = dir.join(&name);
    if cfg.ask_name {
        let mut buf = [0u16; 1024];
        let wstr: Vec<u16> = path.to_string_lossy().encode_utf16().collect();
        let n = wstr.len().min(1023);
        buf[..n].copy_from_slice(&wstr[..n]);
        let mut filter: Vec<u16> = Vec::new();
        for part in ["Image", "*.png;*.jpg", "All files", "*.*"] {
            filter.extend(part.encode_utf16());
            filter.push(0);
        }
        filter.push(0);
        let dirw = w(&dir.to_string_lossy());
        let title = w("Save screenshot");
        let def_ext = w(cfg.ext());
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = hwnd;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = buf.as_mut_ptr();
        ofn.nMaxFile = 1024;
        ofn.lpstrInitialDir = dirw.as_ptr();
        ofn.lpstrTitle = title.as_ptr();
        ofn.lpstrDefExt = def_ext.as_ptr();
        ofn.Flags = (OFN_OVERWRITEPROMPT | OFN_NOCHANGEDIR) as u32;
        if GetSaveFileNameW(&mut ofn) == 0 {
            return;
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(0);
        path = std::path::PathBuf::from(String::from_utf16_lossy(&buf[..end]));
    }
    let crop = capture::crop(&st.shot, screen_rect, &st.shapes);
    let ok = capture::save(&crop, &path.to_string_lossy(), cfg.jpg);
    config::update(|c| c.last = Some([screen_rect.0, screen_rect.1, screen_rect.2, screen_rect.3]));
    let (cx, cy) = (screen_rect.0 + screen_rect.2 / 2, screen_rect.1 + screen_rect.3 / 2);
    DestroyWindow(hwnd);
    if config::get().sound && ok {
        MessageBeep(0);
    }
    toast::show(
        if ok { "Screenshot saved" } else { "Save failed" },
        if ok { Kind::Ok } else { Kind::Err },
        cx,
        cy + sel.3 / 2 + ui::si(16.0),
    );
}
