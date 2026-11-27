use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;

use crate::gfx as gp;
use crate::icons;
use crate::shapes::Tool;
use crate::ui;
use crate::widgets as wg;
use crate::win::*;

pub const ACT_UNDO: usize = 10;
pub const ACT_REDO: usize = 11;
pub const ACT_COPY: usize = 12;
pub const ACT_SAVE: usize = 13;
pub const ACT_CLOSE: usize = 14;
pub const COLOR_BASE: usize = 20;
pub const SIZE_BASE: usize = 30;

static TB_HWND: AtomicUsize = AtomicUsize::new(0);
static INIT: Once = Once::new();

#[derive(Clone, Copy, PartialEq)]
enum It {
    Tool(usize),
    Color(usize),
    Size(usize),
    Act(usize),
    Sep,
}

struct Item {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    it: It,
    t: f32,
}

struct Layout {
    btn: f32,
    tgap: f32,
    pad: f32,
    sw: f32,
    size_w: f32,
    tip: f32,
    margin: f32,
    radius: f32,
    tools_w: f32,
    actions_w: f32,
    row_h: f32,
    act_h: f32,
    hint_y: f32,
    tools_y: f32,
    actions_y: f32,
    win_w: i32,
    win_h: i32,
}

impl Copy for Layout {}

impl Clone for Layout {
    fn clone(&self) -> Self {
        *self
    }
}

fn actions() -> Vec<wg::Action> {
    vec![
        wg::Action { label: "Copy", key: Some("Enter"), icon: ACT_COPY, primary: true },
        wg::Action { label: "Save", key: Some("Ctrl+S"), icon: ACT_SAVE, primary: false },
        wg::Action { label: "Close", key: Some("Esc"), icon: ACT_CLOSE, primary: false },
    ]
}

fn layout() -> Layout {
    let btn = ui::s(30.0);
    let tgap = ui::s(2.0);
    let pad = ui::s(6.0);
    let sw = ui::s(16.0);
    let swgap = ui::s(4.0);
    let size_w = ui::s(28.0);
    let sep = ui::s(10.0);
    let tip = ui::s(30.0);
    let margin = ui::s(16.0);
    let radius = ui::s(12.0);
    let tools_w = pad * 2.0 + 7.0 * btn + 6.0 * tgap + sep + 6.0 * sw + 5.0 * swgap + sep + 3.0 * size_w + 2.0 * tgap + sep + 2.0 * btn + tgap;
    let acts = actions();
    let aw: f32 = acts.iter().map(|a| unsafe { wg::action_w(a) }).sum();
    let actions_w = pad * 2.0 + aw + ui::s(8.0);
    let hint_h = ui::s(28.0);
    let hint_y = tip + margin;
    let tools_y = hint_y + hint_h + ui::s(8.0);
    let actions_y = tools_y + btn + pad * 2.0 + ui::s(6.0);
    let act_h = ui::s(36.0) + pad * 2.0;
    let win_w = (tools_w.max(actions_w) + margin * 2.0) as i32;
    let win_h = (actions_y + act_h + margin) as i32;
    Layout { btn, tgap, pad, sw, size_w, tip, margin, radius, tools_w, actions_w, row_h: btn + pad * 2.0, act_h, hint_y, tools_y, actions_y, win_w, win_h }
}

fn tools_items(l: &Layout) -> Vec<Item> {
    let mut v: Vec<Item> = Vec::new();
    let mut x = l.margin + l.pad;
    let y = l.tools_y + l.pad;
    for i in 0..7 {
        v.push(Item { x, y, w: l.btn, h: l.btn, it: It::Tool(i), t: 0.0 });
        x += l.btn + l.tgap;
    }
    v.push(Item { x: x + ui::s(4.0), y: y + l.btn * 0.2, w: 1.0, h: l.btn * 0.6, it: It::Sep, t: 0.0 });
    x += ui::s(10.0);
    let sy = y + (l.btn - l.sw) / 2.0;
    for i in 0..6 {
        v.push(Item { x, y: sy, w: l.sw, h: l.sw, it: It::Color(i), t: 0.0 });
        x += l.sw + ui::s(4.0);
    }
    v.push(Item { x: x + ui::s(2.0), y: y + l.btn * 0.2, w: 1.0, h: l.btn * 0.6, it: It::Sep, t: 0.0 });
    x += ui::s(10.0);
    for i in 0..3 {
        v.push(Item { x, y, w: l.size_w, h: l.btn, it: It::Size(i), t: 0.0 });
        x += l.size_w + l.tgap;
    }
    v.push(Item { x: x + ui::s(4.0), y: y + l.btn * 0.2, w: 1.0, h: l.btn * 0.6, it: It::Sep, t: 0.0 });
    x += ui::s(10.0);
    for a in [ACT_UNDO, ACT_REDO] {
        v.push(Item { x, y, w: l.btn, h: l.btn, it: It::Act(a), t: 0.0 });
        x += l.btn + l.tgap;
    }
    v
}

fn action_items(l: &Layout) -> Vec<Item> {
    let acts = actions();
    let mut v: Vec<Item> = Vec::new();
    let mut x = (l.win_w as f32 - l.actions_w) / 2.0 + l.pad;
    let y = l.actions_y + l.pad;
    for a in &acts {
        let w = unsafe { wg::action_w(a) };
        v.push(Item { x, y, w, h: ui::s(36.0), it: It::Act(a.icon), t: 0.0 });
        x += w + ui::s(4.0);
    }
    v
}

fn tip_for(it: It) -> Option<String> {
    match it {
        It::Tool(i) => Some(
            match i {
                0 => "Pointer    Esc".to_string(),
                6 => "Text    click inside to place".to_string(),
                _ => Tool::from_idx(i).name().to_string(),
            }
        ),
        It::Color(i) => Some(crate::overlay::COLOR_NAMES[i].to_string()),
        It::Size(_) => Some("Stroke size".to_string()),
        It::Act(a) => Some(
            match a {
                ACT_UNDO => "Undo    Ctrl+Z",
                ACT_REDO => "Redo    Ctrl+Y",
                ACT_COPY => "Copy    Enter",
                ACT_SAVE => "Save    Ctrl+S",
                _ => "Close    Esc",
            }
            .to_string(),
        ),
        It::Sep => None,
    }
}

struct TbState {
    owner: HWND,
    c: gp::Canvas,
    lay: Layout,
    tools: Vec<Item>,
    acts: Vec<Item>,
    hover: i32,
    pressed: i32,
    tool: i32,
    color: i32,
    size: i32,
    undo_n: i32,
    redo_n: i32,
    alpha: u8,
    fading: bool,
    running: bool,
    tip_ms: u32,
    tip_t: f32,
    sel_size: (i32, i32),
    back_tools: Vec<u8>,
    back_acts: Vec<u8>,
    bw: usize,
    bh: usize,
    aw: usize,
    ah: usize,
    px: i32,
    py: i32,
}

pub fn panel_size() -> (i32, i32) {
    let l = layout();
    (l.win_w as i32, (l.actions_y + l.act_h - l.hint_y) as i32)
}

pub unsafe fn create(owner: HWND) -> HWND {
    INIT.call_once(|| {
        let cls_name = w("KerfToolbar");
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
    let lay = layout();
    let tools = tools_items(&lay);
    let acts = action_items(&lay);
    let state = Box::new(TbState {
        owner,
        c: gp::Canvas::new(lay.win_w, lay.win_h),
        lay,
        tools,
        acts,
        hover: -1,
        pressed: -1,
        tool: 0,
        color: 3,
        size: 1,
        undo_n: 0,
        redo_n: 0,
        alpha: 0,
        fading: false,
        running: false,
        tip_ms: 0,
        tip_t: 0.0,
        sel_size: (0, 0),
        back_tools: Vec::new(),
        back_acts: Vec::new(),
        bw: 0,
        bh: 0,
        aw: 0,
        ah: 0,
        px: 0,
        py: 0,
    });
    let lp = Box::into_raw(state);
    let hwnd = CreateWindowExW(
        WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
        w("KerfToolbar").as_ptr(),
        w("").as_ptr(),
        WS_POPUP,
        0,
        0,
        (*lp).c.w,
        (*lp).c.h,
        owner,
        std::ptr::null_mut(),
        GetModuleHandleW(std::ptr::null_mut()),
        lp as *const std::ffi::c_void,
    );
    if hwnd.is_null() {
        drop(Box::from_raw(lp));
        return std::ptr::null_mut();
    }
    TB_HWND.store(hwnd as usize, Ordering::Relaxed);
    compose(hwnd);
    hwnd
}

unsafe fn state_ptr(hwnd: HWND) -> *mut TbState {
    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TbState
}

fn hit_at(items: &[Item], x: f32, y: f32) -> i32 {
    for (i, it) in items.iter().enumerate() {
        if it.it == It::Sep {
            continue;
        }
        let pad = if matches!(it.it, It::Color(_)) { 4.0 } else { 1.0 };
        if x >= it.x - pad && x <= it.x + it.w + pad && y >= it.y - pad && y <= it.y + it.h + pad {
            return i as i32;
        }
    }
    -1
}

fn id_at(items: &[Item], idx: i32) -> i32 {
    if idx < 0 || idx as usize >= items.len() {
        return -1;
    }
    match items[idx as usize].it {
        It::Tool(i) => i as i32,
        It::Color(i) => (COLOR_BASE + i) as i32,
        It::Size(i) => (SIZE_BASE + i) as i32,
        It::Act(i) => i as i32,
        It::Sep => -1,
    }
}

fn hit(s: &TbState, x: f32, y: f32) -> (i32, i32) {
    let t = hit_at(&s.tools, x, y);
    if t >= 0 {
        return (0, t);
    }
    let a = hit_at(&s.acts, x, y);
    if a >= 0 {
        return (1, a);
    }
    (-1, -1)
}

unsafe fn wake(hwnd: HWND) {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return;
    }
    if !(*st).running {
        (*st).running = true;
        SetTimer(hwnd, 1, 16, None);
    }
}

pub unsafe fn set_state(hwnd: HWND, tool: i32, color: i32, size: i32, undo_n: i32, redo_n: i32) {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return;
    }
    (*st).tool = tool;
    (*st).color = color;
    (*st).size = size;
    (*st).undo_n = undo_n;
    (*st).redo_n = redo_n;
    compose(hwnd);
}

pub unsafe fn set_size(hwnd: HWND, w: i32, h: i32) {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return;
    }
    (*st).sel_size = (w, h);
    compose(hwnd);
}

pub unsafe fn show_fade(hwnd: HWND) {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return;
    }
    (*st).alpha = 0;
    (*st).fading = true;
    compose(hwnd);
    wake(hwnd);
}

pub unsafe fn hide(hwnd: HWND) {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return;
    }
    KillTimer(hwnd, 1);
    (*st).running = false;
    (*st).fading = false;
    (*st).alpha = 0;
    (*st).hover = -1;
    (*st).pressed = -1;
    (*st).tip_ms = 0;
    (*st).tip_t = 0.0;
    ShowWindow(hwnd, SW_HIDE);
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

pub unsafe fn place(hwnd: HWND, x: i32, y: i32, comp: *const u8, cw: i32, ch: i32, vx: i32, vy: i32) {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return;
    }
    let s = &mut *st;
    let l = s.lay;
    s.px = x - l.margin as i32;
    s.py = y - l.hint_y as i32;
    let (pw, ph) = (l.tools_w as i32, l.row_h as i32);
    let (aw, ah) = (l.actions_w as i32, l.act_h as i32);
    let blur = ui::si(16.0) as usize;
    let (bt, bw, bh) = gp::acrylic(comp, cw, ch, (x - vx, y - vy, pw, ph), l.radius, ui::BG, 0.68, blur);
    let ax = x + (pw - aw) / 2;
    let ay = y + l.row_h as i32 + ui::si(6.0);
    let (ba, aw2, ah2) = gp::acrylic(comp, cw, ch, (ax - vx, ay - vy, aw, ah), l.radius, ui::BG, 0.68, blur);
    s.back_tools = bt;
    s.back_acts = ba;
    s.bw = bw;
    s.bh = bh;
    s.aw = aw2;
    s.ah = ah2;
    SetWindowPos(hwnd, HWND_TOPMOST, s.px, s.py, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    compose(hwnd);
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
            track_leave(hwnd);
            let (row, idx) = hit(&*st, lp_x(lp) as f32, lp_y(lp) as f32);
            let h = if row < 0 { -1 } else { row * 1000 + idx };
            if h != (*st).hover {
                (*st).hover = h;
                (*st).tip_ms = 0;
                wake(hwnd);
            }
            0
        }
        WM_MOUSELEAVE => {
            (*st).hover = -1;
            (*st).pressed = -1;
            wake(hwnd);
            0
        }
        WM_LBUTTONDOWN => {
            let (row, idx) = hit(&*st, lp_x(lp) as f32, lp_y(lp) as f32);
            (*st).pressed = if row < 0 { -1 } else { row * 1000 + idx };
            wake(hwnd);
            0
        }
        WM_LBUTTONUP => {
            let (row, idx) = hit(&*st, lp_x(lp) as f32, lp_y(lp) as f32);
            let h = if row < 0 { -1 } else { row * 1000 + idx };
            if (*st).pressed == h && h >= 0 {
                let id = if row == 0 { id_at(&(*st).tools, idx) } else { id_at(&(*st).acts, idx) };
                if id >= 0 {
                    PostMessageW((*st).owner, WM_APP + 1, id as WPARAM, 0);
                }
            }
            (*st).pressed = -1;
            wake(hwnd);
            0
        }
        WM_TIMER => {
            tick(hwnd);
            0
        }
        WM_DESTROY => {
            TB_HWND.store(0, Ordering::Relaxed);
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

fn anim_item(it: &mut Item, active: bool) -> bool {
    let target = if active { 1.0 } else { 0.0 };
    if (it.t - target).abs() > 0.015 {
        it.t += (target - it.t) * 0.32;
        if (it.t - target).abs() < 0.03 {
            it.t = target;
        }
        return true;
    }
    it.t = target;
    false
}

unsafe fn tick(hwnd: HWND) {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return;
    }
    let s = &mut *st;
    let mut busy = false;
    if s.fading {
        s.alpha = (s.alpha as u32 + 26).min(255) as u8;
        if s.alpha >= 255 {
            s.fading = false;
        } else {
            busy = true;
        }
    }
    let (hover, pressed) = (s.hover, s.pressed);
    for (i, it) in s.tools.iter_mut().enumerate() {
        if it.it != It::Sep && anim_item(it, hover == i as i32 || pressed == i as i32) {
            busy = true;
        }
    }
    for (i, it) in s.acts.iter_mut().enumerate() {
        if anim_item(it, hover == 1000 + i as i32 || pressed == 1000 + i as i32) {
            busy = true;
        }
    }
    if hover >= 0 {
        if s.tip_ms < 420 {
            s.tip_ms += 16;
            busy = true;
        } else if s.tip_t < 1.0 {
            s.tip_t = (s.tip_t + 0.14).min(1.0);
            busy = true;
        }
    } else if s.tip_t > 0.0 {
        s.tip_t = (s.tip_t - 0.22).max(0.0);
        busy = true;
    }
    if !busy {
        KillTimer(hwnd, 1);
        s.running = false;
    }
    compose(hwnd);
}

unsafe fn draw_button(g: *mut GpGraphics, it: &Item, selected: bool, dim: bool, danger_hover: bool) {
    let r = ui::s(8.0);
    if selected {
        gp::fill_round(g, it.x, it.y, it.w, it.h, r, ui::argb(0x26, ui::ACCENT));
        gp::stroke_round(g, it.x + 0.5, it.y + 0.5, it.w - 1.0, it.h - 1.0, r, ui::argb(0x8C, ui::ACCENT), 1.0);
    } else if it.t > 0.01 {
        let bg = if danger_hover {
            ui::argb((0x30 as f32 * it.t) as u32, ui::DANGER)
        } else {
            ui::argb((0x16 as f32 * it.t) as u32, 0xFFFFFF)
        };
        gp::fill_round(g, it.x, it.y, it.w, it.h, r, bg);
    }
    let col = if dim {
        ui::argb(0x48, 0xFFFFFF)
    } else if danger_hover {
        ui::argb(255, ui::mix(0xFFFFFF, ui::DANGER, it.t))
    } else {
        ui::argb((0xA0 as f32 + 0x5F as f32 * it.t.max(if selected { 1.0 } else { 0.0 })) as u32, 0xFFFFFF)
    };
    let (cx, cy) = (it.x + it.w / 2.0, it.y + it.h / 2.0);
    match it.it {
        It::Tool(i) => icons::tool_icon(g, Tool::from_idx(i), cx, cy, col),
        It::Act(a) => icons::action_icon(g, a, cx, cy, col),
        _ => {}
    }
}

unsafe fn compose(hwnd: HWND) {
    let st = state_ptr(hwnd);
    if st.is_null() {
        return;
    }
    let s = &mut *st;
    let l = s.lay;
    let (tx, ty, tw, th) = (l.margin, l.tools_y, l.tools_w, l.row_h);
    let ax = (l.win_w as f32 - l.actions_w) / 2.0;
    let (ay, aw, ah) = (l.actions_y, l.actions_w, l.act_h);
    s.c.clear();
    if let Some(g) = gp::Gfx::from_dc(s.c.dc) {
        gp::setup(g.raw);
        wg::glass(g.raw, tx, ty, tw, th, l.radius, true);
        wg::glass(g.raw, ax, ay, aw, ah, l.radius, true);
    }
    if !s.back_tools.is_empty() {
        let (cw, ch) = (s.c.w as usize, s.c.h as usize);
        gp::over(s.c.px(), cw, ch, &s.back_tools, s.bw, s.bh, tx as usize, ty as usize);
        gp::over(s.c.px(), cw, ch, &s.back_acts, s.aw, s.ah, ax as usize, ay as usize);
    }
    let g = match gp::Gfx::from_dc(s.c.dc) {
        Some(g) => g,
        None => return,
    };
    gp::setup(g.raw);
    let pal = crate::overlay::palette();
    for (i, it) in s.tools.iter().enumerate() {
        let active = s.hover == i as i32 || s.pressed == i as i32;
        match it.it {
            It::Sep => gp::fill_rect(g.raw, it.x, it.y, 1.0, it.h, 0x3DFFFFFF),
            It::Tool(ti) => draw_button(g.raw, it, s.tool == ti as i32, false, false),
            It::Act(ai) => {
                let dim = (ai == ACT_UNDO && s.undo_n == 0) || (ai == ACT_REDO && s.redo_n == 0);
                draw_button(g.raw, it, false, dim, false);
            }
            It::Color(ci) => {
                let col = pal[ci];
                let (cx, cy) = (it.x + it.w / 2.0, it.y + it.h / 2.0);
                let r = it.w / 2.0;
                if s.color == ci as i32 {
                    gp::circle(g.raw, cx, cy, r + 3.0, 0xE6FFFFFF, false, 1.4);
                } else if active {
                    gp::circle(g.raw, cx, cy, r + 3.0, 0x40FFFFFF, false, 1.2);
                }
                gp::circle(g.raw, cx, cy, r, ui::argb(255, col), true, 1.0);
                if ui::luma(col) < 0.2 {
                    gp::circle(g.raw, cx, cy, r, 0x59FFFFFF, false, 1.0);
                }
            }
            It::Size(si) => {
                let sel = s.size == si as i32;
                if sel {
                    gp::fill_round(g.raw, it.x, it.y, it.w, it.h, ui::s(8.0), 0x3AFFFFFF);
                } else if it.t > 0.01 {
                    gp::fill_round(g.raw, it.x, it.y, it.w, it.h, ui::s(8.0), ui::argb((0x16 as f32 * it.t) as u32, 0xFFFFFF));
                }
                gp::text(&s.c, ["S", "M", "L"][si], (it.x + it.w / 2.0) as i32, (it.y + it.h / 2.0) as i32, ui::s(12.0), sel, if sel { ui::TEXT } else { ui::TEXT_2 }, DT_CENTER | DT_VCENTER);
            }
        }
    }
    let acts = actions();
    for (i, it) in s.acts.iter().enumerate() {
        let a = &acts[i];
        let spec = wg::Action { label: a.label, key: a.key, icon: a.icon, primary: a.primary };
        wg::action(&s.c, g.raw, it.x, it.y, &spec, it.t);
    }
    let hy = l.hint_y;
    let cy = hy + ui::s(14.0);
    let ratio = if s.sel_size.1 > 0 { s.sel_size.0 as f32 / s.sel_size.1 as f32 } else { 0.0 };
    let hint = format!("{} \u{00D7} {}  \u{00B7}  {:.2}:1", s.sel_size.0, s.sel_size.1, ratio);
    let (hw1, _) = gp::measure(&hint, ui::s(12.0), true);
    let hint_w = ui::s(24.0) + hw1 as f32;
    let hx = (l.win_w as f32 - hint_w) / 2.0;
    wg::glass(g.raw, hx, hy, hint_w, ui::s(28.0), ui::s(14.0), false);
    gp::text(&s.c, &hint, (hx + ui::s(12.0)) as i32, cy as i32, ui::s(12.0), true, ui::TEXT, DT_LEFT | DT_VCENTER);
    let hovered = if s.hover >= 1000 { s.acts.get((s.hover - 1000) as usize) } else { s.tools.get(s.hover as usize) };
    if s.tip_t > 0.0 {
        if let Some(it) = hovered {
            if let Some(t) = tip_for(it.it) {
                let (tw, _) = gp::measure(&t, ui::s(12.0), false);
                let (tw, th) = (tw as f32 + ui::s(18.0), ui::s(24.0));
                let mut tx = it.x + it.w / 2.0 - tw / 2.0;
                tx = tx.max(l.margin).min(l.win_w as f32 - l.margin - tw);
                let ty = l.tip - th;
                gp::fill_round(g.raw, tx, ty, tw, th, th / 2.0, ((s.tip_t * 0xF4 as f32) as u32) << 24 | ui::BG);
                gp::stroke_round(g.raw, tx + 0.5, ty + 0.5, tw - 1.0, th - 1.0, th / 2.0, ((s.tip_t * 255.0) as u32) << 24 | ui::LINE_2, 1.0);
                if s.tip_t >= 0.999 {
                    gp::text(&s.c, &t, (tx + tw / 2.0) as i32, (ty + th / 2.0) as i32, ui::s(12.0), false, ui::TEXT, DT_CENTER | DT_VCENTER);
                }
            }
        }
    }
    drop(g);
    let (x, y, a) = (s.px, s.py, s.alpha);
    s.c.present(hwnd, x, y, a);
}
