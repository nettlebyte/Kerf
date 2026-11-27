use crate::gfx as gp;
use crate::ui;
use crate::win::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    None,
    Pen,
    Arrow,
    Rect,
    Ellipse,
    Mosaic,
    Text,
}

impl Tool {
    pub fn idx(&self) -> i32 {
        match self {
            Tool::None => 0,
            Tool::Pen => 1,
            Tool::Arrow => 2,
            Tool::Rect => 3,
            Tool::Ellipse => 4,
            Tool::Mosaic => 5,
            Tool::Text => 6,
        }
    }

    pub fn from_idx(i: usize) -> Tool {
        match i {
            1 => Tool::Pen,
            2 => Tool::Arrow,
            3 => Tool::Rect,
            4 => Tool::Ellipse,
            5 => Tool::Mosaic,
            6 => Tool::Text,
            _ => Tool::None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Tool::None => "Pointer",
            Tool::Pen => "Pen",
            Tool::Arrow => "Arrow",
            Tool::Rect => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Mosaic => "Mosaic",
            Tool::Text => "Text",
        }
    }

    pub fn is_draw(&self) -> bool {
        !matches!(self, Tool::None | Tool::Text)
    }
}

#[derive(Clone)]
pub enum Shape {
    Free { pts: Vec<(f32, f32)>, color: u32, width: f32 },
    Arrow { a: (f32, f32), b: (f32, f32), color: u32, width: f32 },
    Rect { r: (f32, f32, f32, f32), color: u32, width: f32 },
    Ellipse { r: (f32, f32, f32, f32), color: u32, width: f32 },
    Mosaic { r: (f32, f32, f32, f32), block: i32 },
    Text { x: f32, y: f32, text: String, color: u32, size: f32 },
}

pub fn norm(a: (f32, f32), b: (f32, f32)) -> (f32, f32, f32, f32) {
    let x = a.0.min(b.0);
    let y = a.1.min(b.1);
    (x, y, (a.0 - b.0).abs(), (a.1 - b.1).abs())
}

pub unsafe fn draw(bits: *mut u8, w: usize, h: usize, g: *mut GpGraphics, origin: (f32, f32), clip: Option<(i32, i32, i32, i32)>, s: &Shape) {
    let (ox, oy) = origin;
    let bounds = clip.unwrap_or((0, 0, w as i32, h as i32));
    match s {
        Shape::Free { pts, color, width } => {
            if pts.len() < 2 {
                return;
            }
            let moved: Vec<(f32, f32)> = pts.iter().map(|(x, y)| (x + ox, y + oy)).collect();
            gp::poly(g, &moved, 0xFF000000 | *color, *width * ui::scale(), true);
        }
        Shape::Arrow { a, b, color, width } => {
            let c = 0xFF000000 | *color;
            let wd = *width * ui::scale();
            let (a, b) = ((a.0 + ox, a.1 + oy), (b.0 + ox, b.1 + oy));
            gp::line(g, a.0, a.1, b.0, b.1, c, wd, true);
            let dx = b.0 - a.0;
            let dy = b.1 - a.1;
            let len = (dx * dx + dy * dy).sqrt();
            if len > 2.0 {
                let ux = dx / len;
                let uy = dy / len;
                let hl = (wd * 4.5).min(len / 2.0);
                for ang in [-2.45f32, 2.45f32] {
                    let ca = ang.cos();
                    let sa = ang.sin();
                    let hx = b.0 + (ux * ca - uy * sa) * hl;
                    let hy = b.1 + (ux * sa + uy * ca) * hl;
                    gp::line(g, b.0, b.1, hx, hy, c, wd, true);
                }
            }
        }
        Shape::Rect { r, color, width } => {
            gp::stroke_rect(g, r.0 + ox, r.1 + oy, r.2, r.3, 0xFF000000 | *color, *width * ui::scale());
        }
        Shape::Ellipse { r, color, width } => {
            let p = gp::pen(0xFF000000 | *color, *width * ui::scale(), false);
            GdipDrawEllipse(g, p, r.0 + ox, r.1 + oy, r.2, r.3);
            GdipDeletePen(p);
        }
        Shape::Mosaic { r, block } => {
            let ir = (r.0 as i32, r.1 as i32, r.2 as i32 + 1, r.3 as i32 + 1);
            let ir = (ir.0 - ox as i32, ir.1 - oy as i32, ir.2, ir.3);
            gp::mosaic(bits, w, h, ir, (*block as f32 * ui::scale()) as i32, bounds);
        }
        Shape::Text { x, y, text, color, size } => {
            gp::text_px(
                bits,
                w,
                h,
                text,
                (x + ox) as i32,
                (y + oy) as i32,
                *size * ui::scale(),
                false,
                *color,
                DT_LEFT | DT_TOP,
                clip,
            );
        }
    }
}
