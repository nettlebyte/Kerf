use crate::gfx as gp;
use crate::ui;
use crate::win::*;

pub unsafe fn glass(g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, r: f32, shadow: bool) {
    if shadow {
        gp::shadow(g, x, y, w, h, r, 0.5);
    }
    gp::fill_round(g, x, y, w, h, r, ui::argb(0xE8, ui::BG));
    gp::stroke_round(g, x + 0.5, y + 0.5, w - 1.0, h - 1.0, r, 0x2AFFFFFF, 1.0);
}

pub unsafe fn keycap(c: &gp::Canvas, g: *mut GpGraphics, x: f32, cy: f32, text: &str, dark: bool) -> f32 {
    let (tw, _) = gp::measure(text, ui::s(11.0), true);
    let (w, h) = (tw as f32 + ui::s(14.0), ui::s(20.0));
    let y = cy - h / 2.0;
    if dark {
        gp::fill_round(g, x, y, w, h, ui::s(5.0), 0x4D000000);
    } else {
        gp::fill_round(g, x, y, w, h, ui::s(5.0), 0x1FFFFFFF);
        gp::stroke_round(g, x + 0.5, y + 0.5, w - 1.0, h - 1.0, ui::s(5.0), 0x30FFFFFF, 1.0);
    }
    gp::text(c, text, (x + w / 2.0) as i32, cy as i32, ui::s(11.0), true, if dark { 0xFFFFFF } else { ui::TEXT_2 }, DT_CENTER | DT_VCENTER);
    w
}

pub unsafe fn toggle(_c: &gp::Canvas, g: *mut GpGraphics, x: f32, y: f32, on: bool, t: f32, hover: f32) -> (f32, f32) {
    let (w, h) = (ui::s(40.0), ui::s(22.0));
    let r = h / 2.0;
    if on {
        gp::fill_grad(g, x, y, w, h, r, ui::argb(255, ui::ACCENT), ui::argb(255, ui::ACCENT_2));
    } else {
        gp::fill_round(g, x, y, w, h, r, ui::argb(0x30 + (0x18 as f32 * hover) as u32, 0xFFFFFF));
        gp::stroke_round(g, x + 0.5, y + 0.5, w - 1.0, h - 1.0, r, 0x3CFFFFFF, 1.0);
    }
    let kr = ui::s(8.0);
    let kx = x + ui::s(3.0) + kr + t * (w - ui::s(6.0) - kr * 2.0);
    gp::circle(g, kx, y + h / 2.0, kr + ui::s(0.8), 0x40000000, true, 1.0);
    gp::circle(g, kx, y + h / 2.0, kr, 0xFFFFFFFF, true, 1.0);
    (w, h)
}

pub unsafe fn segmented_w(opts: &[&str]) -> f32 {
    let font = ui::s(13.0);
    opts.iter().map(|o| gp::measure(o, font, false).0 as f32 + ui::s(22.0)).sum::<f32>() + ui::s(6.0)
}

pub unsafe fn segmented(c: &gp::Canvas, g: *mut GpGraphics, x: f32, y: f32, opts: &[&str], sel: usize, hover: i32) -> f32 {
    let h = ui::s(30.0);
    let font = ui::s(13.0);
    let widths: Vec<f32> = opts.iter().map(|o| gp::measure(o, font, false).0 as f32 + ui::s(22.0)).collect();
    let w: f32 = widths.iter().sum::<f32>() + ui::s(6.0);
    gp::fill_round(g, x, y, w, h, ui::s(8.0), 0x1FFFFFFF);
    gp::stroke_round(g, x + 0.5, y + 0.5, w - 1.0, h - 1.0, ui::s(8.0), 0x2AFFFFFF, 1.0);
    let mut cx = x + ui::s(3.0);
    for (i, o) in opts.iter().enumerate() {
        let ow = widths[i];
        if i == sel {
            gp::fill_round(g, cx, y + ui::s(3.0), ow, h - ui::s(6.0), ui::s(6.0), 0x38FFFFFF);
        } else if hover == i as i32 {
            gp::fill_round(g, cx, y + ui::s(3.0), ow, h - ui::s(6.0), ui::s(6.0), 0x14FFFFFF);
        }
        let col = if i == sel { ui::TEXT } else if hover == i as i32 { ui::mix(ui::TEXT_2, ui::TEXT, 1.0) } else { ui::TEXT_2 };
        gp::text(c, o, (cx + ow / 2.0) as i32, (y + h / 2.0) as i32, font, false, col, DT_CENTER | DT_VCENTER);
        cx += ow;
    }
    w
}

pub struct Action {
    pub label: &'static str,
    pub key: Option<&'static str>,
    pub icon: usize,
    pub primary: bool,
}

pub unsafe fn action_w(a: &Action) -> f32 {
    let lw = gp::measure(a.label, ui::s(13.0), false).0 as f32;
    let kw = match a.key {
        Some(k) => gp::measure(k, ui::s(11.0), true).0 as f32 + ui::s(14.0) + ui::s(8.0),
        None => 0.0,
    };
    ui::s(14.0) * 2.0 + ui::s(18.0) + ui::s(8.0) + lw + kw
}

pub unsafe fn action(c: &gp::Canvas, g: *mut GpGraphics, x: f32, y: f32, a: &Action, hover: f32) -> f32 {
    let w = action_w(a);
    let h = ui::s(36.0);
    let r = ui::s(9.0);
    if a.primary {
        gp::fill_grad(g, x, y, w, h, r, ui::argb(255, ui::ACCENT), ui::argb(255, ui::ACCENT_2));
        if hover > 0.01 {
            gp::fill_round(g, x, y, w, h, r, ui::argb((0x2E as f32 * hover) as u32, 0xFFFFFF));
        }
    } else {
        gp::fill_round(g, x, y, w, h, r, ui::argb(0x1C as u32 + (0x14 as f32 * hover) as u32, 0xFFFFFF));
        gp::stroke_round(g, x + 0.5, y + 0.5, w - 1.0, h - 1.0, r, 0x2EFFFFFF, 1.0);
    }
    let cy = y + h / 2.0;
    crate::icons::action_icon(g, a.icon, x + ui::s(23.0), cy, if a.primary { 0xFF15151A } else { ui::TEXT });
    let tx = x + ui::s(40.0);
    let tw = gp::text(c, a.label, tx as i32, cy as i32, ui::s(13.0), false, if a.primary { 0xFF15151A } else { ui::TEXT }, DT_LEFT | DT_VCENTER).0 as f32;
    if let Some(k) = a.key {
        keycap(c, g, tx + tw + ui::s(8.0), cy, k, a.primary);
    }
    w
}
