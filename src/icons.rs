use crate::gfx as gp;
use crate::shapes::Tool;
use crate::toolbar;
use crate::ui;
use crate::win::*;

struct Grid {
    k: f32,
    cx: f32,
    cy: f32,
}

impl Grid {
    fn new(cx: f32, cy: f32) -> Grid {
        Grid { k: ui::scale(), cx, cy }
    }

    fn x(&self, v: f32) -> f32 {
        self.cx + (v - 10.0) * self.k
    }

    fn y(&self, v: f32) -> f32 {
        self.cy + (v - 10.0) * self.k
    }

    fn w(&self, v: f32) -> f32 {
        v * self.k
    }

    unsafe fn line(&self, g: *mut GpGraphics, a: (f32, f32), b: (f32, f32), color: u32, wd: f32, round: bool) {
        gp::line(g, self.x(a.0), self.y(a.1), self.x(b.0), self.y(b.1), color, self.w(wd), round);
    }

    unsafe fn box_(&self, g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32, wd: f32) {
        gp::stroke_round(g, self.x(x), self.y(y), self.w(w), self.w(h), self.w(r), color, self.w(wd));
    }

    unsafe fn dot(&self, g: *mut GpGraphics, x: f32, y: f32, r: f32, color: u32) {
        gp::circle(g, self.x(x), self.y(y), self.w(r), color, true, 1.0);
    }

    unsafe fn arc(&self, g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, start: f32, sweep: f32, color: u32, wd: f32) {
        let p = gp::pen(color, self.w(wd), true);
        GdipDrawArc(g, p, self.x(x), self.y(y), self.w(w), self.w(h), start, sweep);
        GdipDeletePen(p);
    }
}

const ST: f32 = 1.7;

pub unsafe fn tool_icon(g: *mut GpGraphics, tool: Tool, cx: f32, cy: f32, color: u32) {
    let r = Grid::new(cx, cy);
    match tool {
        Tool::None => {
            let pts = [(4.5, 4.0), (4.5, 15.0), (7.6, 12.2), (9.6, 16.5), (11.6, 15.6), (9.7, 11.4), (13.8, 11.2)];
            gp::poly(g, &pts.iter().map(|(x, y)| (r.x(*x), r.y(*y))).collect::<Vec<_>>(), color, r.w(ST), true);
            gp::line(g, r.x(4.5), r.y(4.0), r.x(12.2), r.y(11.0), color, r.w(ST), true);
        }
        Tool::Pen => {
            r.line(g, (4.0, 16.0), (5.0, 12.8), color, ST, true);
            r.line(g, (4.0, 16.0), (7.2, 15.0), color, ST, true);
            r.line(g, (5.0, 12.8), (13.9, 3.9), color, ST, true);
            r.line(g, (7.2, 15.0), (16.1, 6.1), color, ST, true);
            r.line(g, (13.9, 3.9), (16.1, 6.1), color, ST, true);
        }
        Tool::Arrow => {
            r.line(g, (5.5, 14.5), (14.5, 5.5), color, ST, true);
            r.line(g, (14.5, 5.5), (9.6, 5.5), color, ST, true);
            r.line(g, (14.5, 5.5), (14.5, 10.4), color, ST, true);
        }
        Tool::Rect => r.box_(g, 4.5, 5.5, 11.0, 9.0, 2.0, color, ST),
        Tool::Ellipse => {
            let p = gp::pen(color, r.w(ST), false);
            GdipDrawEllipse(g, p, r.x(4.0), r.y(5.0), r.w(12.0), r.w(10.0));
            GdipDeletePen(p);
        }
        Tool::Mosaic => {
            let s = 2.6;
            let mut y = 4.6;
            while y < 15.0 {
                let mut x = 4.6;
                while x < 15.0 {
                    let on = ((x * 3.1 + y * 1.7) as i32) % 3 != 0;
                    if on {
                        gp::fill_rect(g, r.x(x), r.y(y), r.w(s - 0.5), r.w(s - 0.5), color);
                    }
                    x += s;
                }
                y += s;
            }
        }
        Tool::Text => {
            r.line(g, (5.0, 5.0), (15.0, 5.0), color, 2.0, true);
            r.line(g, (10.0, 5.0), (10.0, 15.5), color, 2.0, true);
        }
    }
}

pub unsafe fn action_icon(g: *mut GpGraphics, id: usize, cx: f32, cy: f32, color: u32) {
    let r = Grid::new(cx, cy);
    match id {
        toolbar::ACT_UNDO => {
            r.arc(g, 5.5, 5.0, 11.0, 11.0, 180.0, 290.0, color, ST);
            r.line(g, (5.5, 10.5), (2.8, 13.4), color, ST, true);
            r.line(g, (5.5, 10.5), (8.2, 13.4), color, ST, true);
        }
        toolbar::ACT_REDO => {
            r.arc(g, 3.5, 5.0, 11.0, 11.0, 0.0, -290.0, color, ST);
            r.line(g, (14.5, 10.5), (11.8, 13.4), color, ST, true);
            r.line(g, (14.5, 10.5), (17.2, 13.4), color, ST, true);
        }
        toolbar::ACT_COPY => {
            r.box_(g, 7.5, 4.5, 8.0, 8.0, 2.0, color, ST);
            r.box_(g, 4.5, 7.5, 8.0, 8.0, 2.0, color, ST);
        }
        toolbar::ACT_SAVE => {
            r.line(g, (4.0, 12.5), (4.0, 16.0), color, ST, true);
            r.line(g, (4.0, 16.0), (16.0, 16.0), color, ST, true);
            r.line(g, (16.0, 16.0), (16.0, 12.5), color, ST, true);
            r.line(g, (10.0, 4.0), (10.0, 12.0), color, ST, true);
            r.line(g, (6.8, 8.8), (10.0, 12.0), color, ST, true);
            r.line(g, (10.0, 12.0), (13.2, 8.8), color, ST, true);
        }
        toolbar::ACT_CLOSE => {
            r.line(g, (5.5, 5.5), (14.5, 14.5), color, 2.0, true);
            r.line(g, (14.5, 5.5), (5.5, 14.5), color, 2.0, true);
        }
        _ => {}
    }
}

pub unsafe fn close_icon(g: *mut GpGraphics, cx: f32, cy: f32, size: f32, color: u32) {
    let h = size * 0.5;
    gp::line(g, cx - h, cy - h, cx + h, cy + h, color, ui::s(1.6), true);
    gp::line(g, cx + h, cy - h, cx - h, cy + h, color, ui::s(1.6), true);
}

pub unsafe fn menu_icon(g: *mut GpGraphics, idx: usize, cx: f32, cy: f32, color: u32) {
    let r = Grid::new(cx, cy);
    match idx {
        0 => {
            r.line(g, (4.0, 8.0), (4.0, 4.0), color, ST, true);
            r.line(g, (4.0, 4.0), (8.0, 4.0), color, ST, true);
            r.line(g, (12.0, 4.0), (16.0, 4.0), color, ST, true);
            r.line(g, (16.0, 4.0), (16.0, 8.0), color, ST, true);
            r.line(g, (16.0, 12.0), (16.0, 16.0), color, ST, true);
            r.line(g, (16.0, 16.0), (12.0, 16.0), color, ST, true);
            r.line(g, (8.0, 16.0), (4.0, 16.0), color, ST, true);
            r.line(g, (4.0, 16.0), (4.0, 12.0), color, ST, true);
        }
        1 => {
            r.box_(g, 3.5, 4.5, 13.0, 9.5, 1.8, color, ST);
            r.line(g, (10.0, 14.0), (10.0, 16.0), color, ST, true);
            r.line(g, (7.0, 16.0), (13.0, 16.0), color, ST, true);
        }
        2 => {
            r.arc(g, 3.5, 5.0, 11.0, 11.0, 0.0, -290.0, color, ST);
            r.line(g, (14.5, 10.5), (11.8, 13.4), color, ST, true);
            r.line(g, (14.5, 10.5), (17.2, 13.4), color, ST, true);
        }
        3 => {
            r.line(g, (4.0, 6.5), (10.4, 6.5), color, 1.7, true);
            r.dot(g, 13.0, 6.5, 2.1, color);
            r.line(g, (15.6, 6.5), (16.0, 6.5), color, 1.7, true);
            r.line(g, (4.0, 13.5), (4.4, 13.5), color, 1.7, true);
            r.dot(g, 7.0, 13.5, 2.1, color);
            r.line(g, (9.6, 13.5), (16.0, 13.5), color, 1.7, true);
        }
        _ => {
            r.arc(g, 4.5, 5.5, 11.0, 11.0, -60.0, 300.0, color, ST);
            r.line(g, (10.0, 3.5), (10.0, 9.5), color, ST, true);
        }
    }
}

pub unsafe fn logo(g: *mut GpGraphics, x: f32, y: f32, size: f32, tile: bool) {
    let s = size;
    if tile {
        gp::fill_round(g, x, y, s, s, s * 0.26, 0xFF2A2A31);
        gp::stroke_round(g, x + 0.5, y + 0.5, s - 1.0, s - 1.0, s * 0.26, 0x30FFFFFF, 1.0);
    }
    let (kx, ky) = (x + s * 0.5, y + s * 0.5);
    let px = |v: f32| kx + (v - 10.0) * s / 20.0;
    let py = |v: f32| ky + (v - 10.0) * s / 20.0;
    let wd = s * 0.115;
    gp::line(g, px(6.15), py(4.6), px(6.15), py(15.4), 0xFFF3F3F6, wd, true);
    gp::line(g, px(13.85), py(4.6), px(8.0), py(10.0), ui::argb(255, ui::ACCENT), wd, true);
    gp::line(g, px(8.0), py(10.0), px(13.85), py(15.4), ui::argb(255, ui::ACCENT), wd, true);
}

pub unsafe fn app_icon(size: i32) -> HICON {
    let c = gp::Canvas::new(size, size);
    let s = size as f32;
    if let Some(g) = gp::Gfx::from_dc(c.dc) {
        gp::setup(g.raw);
        gp::fill_round(g.raw, 0.0, 0.0, s, s, s * 0.26, 0xFF1C1C21);
        gp::stroke_round(g.raw, 0.5, 0.5, s - 1.0, s - 1.0, s * 0.26, 0x30FFFFFF, (s * 0.03).max(1.0));
        let px = |v: f32| v * s;
        let py = |v: f32| v * s;
        let wd = s * 0.115;
        gp::line(g.raw, px(0.305), py(0.23), px(0.305), py(0.77), 0xFFF3F3F6, wd, true);
        gp::line(g.raw, px(0.695), py(0.23), px(0.40), py(0.50), ui::argb(255, ui::ACCENT), wd, true);
        gp::line(g.raw, px(0.40), py(0.50), px(0.695), py(0.77), ui::argb(255, ui::ACCENT), wd, true);
    }
    let mask = windows_sys::Win32::Graphics::Gdi::CreateBitmap(size, size, 1, 1, std::ptr::null());
    let ii = ICONINFO {
        fIcon: TRUE,
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: mask,
        hbmColor: c.hbmp,
    };
    let icon = CreateIconIndirect(&ii);
    windows_sys::Win32::Graphics::Gdi::DeleteObject(mask);
    icon
}
