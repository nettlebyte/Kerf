use std::cell::RefCell;
use std::collections::HashMap;

use crate::win::*;

pub struct Canvas {
    pub dc: HDC,
    pub hbmp: HBITMAP,
    pub bits: *mut u8,
    pub w: i32,
    pub h: i32,
}

impl Canvas {
    pub unsafe fn new(w: i32, h: i32) -> Canvas {
        let w = w.max(1);
        let h = h.max(1);
        let sdc = GetDC(std::ptr::null_mut());
        let mdc = CreateCompatibleDC(sdc);
        let mut bi = BITMAPINFO {
            bmiHeader: std::mem::zeroed(),
            bmiColors: std::mem::zeroed(),
        };
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = w;
        bi.bmiHeader.biHeight = -h;
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB;
        bi.bmiHeader.biSizeImage = (w * h * 4) as u32;
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbmp = CreateDIBSection(mdc, &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
        SelectObject(mdc, hbmp);
        ReleaseDC(std::ptr::null_mut(), sdc);
        let c = Canvas { dc: mdc, hbmp, bits: bits as *mut u8, w, h };
        c.clear();
        c
    }

    pub fn len(&self) -> usize {
        self.w as usize * self.h as usize * 4
    }

    pub unsafe fn px(&self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.bits, self.len())
    }

    pub unsafe fn clear(&self) {
        if !self.bits.is_null() {
            std::ptr::write_bytes(self.bits, 0, self.len());
        }
    }

    pub unsafe fn present(&self, hwnd: HWND, x: i32, y: i32, alpha: u8) {
        self.present_size(hwnd, x, y, self.w, self.h, alpha)
    }

    pub unsafe fn present_size(&self, hwnd: HWND, x: i32, y: i32, w: i32, h: i32, alpha: u8) {
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: alpha,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let sz = SIZE { cx: w, cy: h };
        let d = POINT { x, y };
        let s = POINT { x: 0, y: 0 };
        let sdc = GetDC(std::ptr::null_mut());
        UpdateLayeredWindow(hwnd, sdc, &d, &sz, self.dc, &s, 0, &blend, ULW_ALPHA);
        ReleaseDC(std::ptr::null_mut(), sdc);
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            if !self.hbmp.is_null() {
                DeleteObject(self.hbmp);
            }
            if !self.dc.is_null() {
                DeleteDC(self.dc);
            }
        }
    }
}

pub fn startup() {
    unsafe {
        let mut input: GdiplusStartupInput = std::mem::zeroed();
        input.GdiplusVersion = 1;
        let mut token: usize = 0;
        GdiplusStartup(&mut token, &input, std::ptr::null_mut());
    }
}

pub struct Gfx {
    pub raw: *mut GpGraphics,
}

impl Gfx {
    pub unsafe fn from_dc(dc: HDC) -> Option<Gfx> {
        let mut g: *mut GpGraphics = std::ptr::null_mut();
        if GdipCreateFromHDC(dc, &mut g) == 0 {
            Some(Gfx { raw: g })
        } else {
            None
        }
    }
}

impl Drop for Gfx {
    fn drop(&mut self) {
        unsafe {
            if !self.raw.is_null() {
                GdipFlush(self.raw, FlushIntentionFlush);
                GdipDeleteGraphics(self.raw);
            }
        }
    }
}

pub unsafe fn setup(g: *mut GpGraphics) {
    GdipSetSmoothingMode(g, SmoothingModeAntiAlias);
    GdipSetPixelOffsetMode(g, PixelOffsetModeHalf);
}

pub unsafe fn brush(color: u32) -> *mut GpSolidFill {
    let mut b: *mut GpSolidFill = std::ptr::null_mut();
    GdipCreateSolidFill(color, &mut b);
    b
}

pub unsafe fn pen(color: u32, width: f32, round: bool) -> *mut GpPen {
    let mut p: *mut GpPen = std::ptr::null_mut();
    GdipCreatePen1(color, width, UnitPixel, &mut p);
    let cap = if round { LineCapRound } else { LineCapFlat };
    GdipSetPenStartCap(p, cap);
    GdipSetPenEndCap(p, cap);
    p
}

pub unsafe fn fill_rect(g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, color: u32) {
    if w <= 0.0 || h <= 0.0 || (color >> 24) == 0 {
        return;
    }
    let b = brush(color);
    GdipFillRectangle(g, b as *mut GpBrush, x, y, w, h);
    GdipDeleteBrush(b as *mut GpBrush);
}

pub unsafe fn stroke_rect(g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, color: u32, wd: f32) {
    let p = pen(color, wd, false);
    GdipDrawRectangle(g, p, x, y, w, h);
    GdipDeletePen(p);
}

pub unsafe fn line(g: *mut GpGraphics, x1: f32, y1: f32, x2: f32, y2: f32, color: u32, wd: f32, round: bool) {
    let p = pen(color, wd, round);
    GdipDrawLine(g, p, x1, y1, x2, y2);
    GdipDeletePen(p);
}

pub unsafe fn poly(g: *mut GpGraphics, pts: &[(f32, f32)], color: u32, wd: f32, round: bool) {
    if pts.len() < 2 {
        return;
    }
    let arr: Vec<PointF> = pts.iter().map(|(x, y)| PointF { X: *x, Y: *y }).collect();
    let p = pen(color, wd, round);
    GdipDrawLines(g, p, arr.as_ptr(), arr.len() as i32);
    GdipDeletePen(p);
}

pub unsafe fn round_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> *mut GpPath {
    let mut p: *mut GpPath = std::ptr::null_mut();
    GdipCreatePath(FillModeAlternate, &mut p);
    let r = r.max(0.0).min(w / 2.0).min(h / 2.0);
    if r < 0.5 {
        GdipAddPathArc(p, x, y, 0.01, 0.01, 180.0, 90.0);
        GdipAddPathArc(p, x + w, y, 0.01, 0.01, 270.0, 90.0);
        GdipAddPathArc(p, x + w, y + h, 0.01, 0.01, 0.0, 90.0);
        GdipAddPathArc(p, x, y + h, 0.01, 0.01, 90.0, 90.0);
    } else {
        GdipAddPathArc(p, x, y, r * 2.0, r * 2.0, 180.0, 90.0);
        GdipAddPathArc(p, x + w - r * 2.0, y, r * 2.0, r * 2.0, 270.0, 90.0);
        GdipAddPathArc(p, x + w - r * 2.0, y + h - r * 2.0, r * 2.0, r * 2.0, 0.0, 90.0);
        GdipAddPathArc(p, x, y + h - r * 2.0, r * 2.0, r * 2.0, 90.0, 90.0);
    }
   p
}

pub unsafe fn fill_round(g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32) {
    if w <= 0.0 || h <= 0.0 || (color >> 24) == 0 {
        return;
    }
    let p = round_path(x, y, w, h, r);
    let b = brush(color);
    GdipFillPath(g, b as *mut GpBrush, p);
    GdipDeleteBrush(b as *mut GpBrush);
    GdipDeletePath(p);
}

pub unsafe fn stroke_round(g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32, wd: f32) {
    if w <= 0.0 || h <= 0.0 || (color >> 24) == 0 {
        return;
    }
    let p = round_path(x, y, w, h, r);
    let pn = pen(color, wd, false);
    GdipDrawPath(g, pn, p);
    GdipDeletePen(pn);
    GdipDeletePath(p);
}

pub unsafe fn stroke_round_dash(g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32, wd: f32, dash: &[f32]) {
    if w <= 0.0 || h <= 0.0 || (color >> 24) == 0 {
        return;
    }
    let p = round_path(x, y, w, h, r);
    let pn = pen(color, wd, false);
    GdipSetPenDashStyle(pn, DashStyleCustom);
    GdipSetPenDashArray(pn, dash.as_ptr(), dash.len() as i32);
    GdipDrawPath(g, pn, p);
    GdipDeletePen(pn);
    GdipDeletePath(p);
}

pub unsafe fn circle(g: *mut GpGraphics, cx: f32, cy: f32, r: f32, color: u32, fill: bool, wd: f32) {
    if (color >> 24) == 0 {
        return;
    }
    if fill {
        let b = brush(color);
        GdipFillEllipse(g, b as *mut GpBrush, cx - r, cy - r, r * 2.0, r * 2.0);
        GdipDeleteBrush(b as *mut GpBrush);
    } else {
        let p = pen(color, wd, false);
        GdipDrawEllipse(g, p, cx - r, cy - r, r * 2.0, r * 2.0);
        GdipDeletePen(p);
    }
}

pub unsafe fn fill_grad(g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, r: f32, c1: u32, c2: u32) {
    let p = round_path(x, y, w, h, r);
    let rect = RectF { X: x, Y: y, Width: w, Height: h };
    let mut br: *mut GpLineGradient = std::ptr::null_mut();
    GdipCreateLineBrushFromRect(&rect, c1, c2, LinearGradientModeVertical, WrapModeTile, &mut br);
    if !br.is_null() {
        GdipFillPath(g, br as *mut GpBrush, p);
        GdipDeleteBrush(br as *mut GpBrush);
    }
    GdipDeletePath(p);
}

pub unsafe fn shadow(g: *mut GpGraphics, x: f32, y: f32, w: f32, h: f32, r: f32, strength: f32) {
    for i in 0..8 {
        let e = (8 - i) as f32 * 1.6;
        let a = (strength * (i + 1) as f32 * 255.0 / 36.0).round() as u32;
        fill_round(g, x - e, y - e + 2.0, w + e * 2.0, h + e * 2.0, r + e, a << 24);
    }
}

struct Scratch {
    dc: HDC,
    hbmp: HBITMAP,
    bits: *mut u8,
    w: usize,
    h: usize,
    stride: usize,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.hbmp);
            DeleteDC(self.dc);
        }
    }
}

thread_local! {
    static SCRATCH: RefCell<Option<Scratch>> = const { RefCell::new(None) };
    static FONTS: RefCell<Option<HashMap<(i32, bool), HFONT>>> = const { RefCell::new(None) };
}

unsafe fn scratch(w: usize, h: usize) -> Scratch {
    let sdc = GetDC(std::ptr::null_mut());
    let mdc = CreateCompatibleDC(sdc);
    let mut bi = BITMAPINFO {
        bmiHeader: std::mem::zeroed(),
        bmiColors: std::mem::zeroed(),
    };
    bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bi.bmiHeader.biWidth = w as i32;
    bi.bmiHeader.biHeight = -(h as i32);
    bi.bmiHeader.biPlanes = 1;
    bi.bmiHeader.biBitCount = 32;
    bi.bmiHeader.biCompression = BI_RGB;
    bi.bmiHeader.biSizeImage = (w * 4) as u32 * h as u32;
    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
    let hbmp = CreateDIBSection(mdc, &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
    SelectObject(mdc, hbmp);
    ReleaseDC(std::ptr::null_mut(), sdc);
    Scratch {
        dc: mdc,
        hbmp,
        bits: bits as *mut u8,
        w,
        h,
        stride: w * 4,
    }
}

unsafe fn ensure_scratch(cell: &mut Option<Scratch>, w: usize, h: usize) {
    let need = match cell {
        Some(s) => s.w < w || s.h < h,
        None => true,
    };
    if need {
        *cell = Some(scratch(w.max(512), h.max(128)));
    }
}

unsafe fn font(size: f32, bold: bool) -> HFONT {
    let px = size.round().max(1.0) as i32;
    FONTS.with(|c| {
        let mut c = c.borrow_mut();
        let map = c.get_or_insert_with(HashMap::new);
        if let Some(f) = map.get(&(px, bold)) {
            return *f;
        }
        let face = w("Segoe UI");
        let f = CreateFontW(
            -px,
            0,
            0,
            0,
            if bold { FW_BOLD as i32 } else { FW_REGULAR as i32 },
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY,
            FF_DONTCARE as u32,
            face.as_ptr(),
        );
        map.insert((px, bold), f);
        f
    })
}

pub unsafe fn measure(s: &str, size: f32, bold: bool) -> (i32, i32) {
    if s.is_empty() {
        return (0, 0);
    }
    SCRATCH.with(|cell| {
        let mut cell = cell.borrow_mut();
        ensure_scratch(&mut cell, 8, 8);
        let sc = cell.as_ref().unwrap_unchecked();
        let f = font(size, bold);
        let old = SelectObject(sc.dc, f);
        let v: Vec<u16> = s.encode_utf16().collect();
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        DrawTextW(sc.dc, v.as_ptr(), v.len() as i32, &mut r, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(sc.dc, old);
        (r.right.max(0), r.bottom.max(0))
    })
}

pub unsafe fn fit(s: &str, max_w: i32, size: f32, bold: bool) -> String {
    if measure(s, size, bold).0 <= max_w {
        return s.to_string();
    }
    let mut t: String = s.chars().collect();
    while !t.is_empty() && measure(&format!("{}\u{2026}", t), size, bold).0 > max_w {
        t.pop();
    }
    format!("{}\u{2026}", t)
}

pub unsafe fn text_px(
    bits: *mut u8,
    cw: usize,
    ch: usize,
    s: &str,
    x: i32,
    y: i32,
    size: f32,
    bold: bool,
    color: u32,
    align: u32,
    clip: Option<(i32, i32, i32, i32)>,
) -> (i32, i32) {
    let (tw, th) = measure(s, size, bold);
    if tw <= 0 || th <= 0 {
        return (0, 0);
    }
    let dx = if align & DT_CENTER != 0 {
        x - tw / 2
    } else if align & DT_RIGHT != 0 {
        x - tw
    } else {
        x
    };
    let dy = if align & DT_VCENTER != 0 {
        y - th / 2
    } else if align & DT_BOTTOM != 0 {
        y - th
    } else {
        y
    };
    let mut x0 = dx.max(0);
    let mut y0 = dy.max(0);
    let mut x1 = (dx + tw).min(cw as i32);
    let mut y1 = (dy + th).min(ch as i32);
    if let Some((cx, cy, cww, chh)) = clip {
        x0 = x0.max(cx);
        y0 = y0.max(cy);
        x1 = x1.min(cx + cww);
        y1 = y1.min(cy + chh);
    }
    if x1 <= x0 || y1 <= y0 {
        return (tw, th);
    }
    let (r, g, b) = (((color >> 16) & 0xFF) as u32, ((color >> 8) & 0xFF) as u32, (color & 0xFF) as u32);
    SCRATCH.with(|cell| {
        let mut cell = cell.borrow_mut();
        ensure_scratch(&mut cell, tw as usize, th as usize);
        let sc = cell.as_ref().unwrap_unchecked();
        let sstride = sc.stride;
        for yy in 0..th as usize {
            let srow = yy * sstride;
            for xx in 0..tw as usize {
                let so = srow + xx * 4;
                *sc.bits.add(so) = 0;
                *sc.bits.add(so + 1) = 0;
                *sc.bits.add(so + 2) = 0;
                *sc.bits.add(so + 3) = 255;
            }
        }
        for yy in y0..y1 {
            let srow = (yy - dy) as usize * sstride;
            let crow = yy as usize * cw * 4;
            for xx in x0..x1 {
                let so = srow + (xx - dx) as usize * 4;
                let co = crow + xx as usize * 4;
                let a = *bits.add(co + 3) as u32;
                if a == 0 {
                    continue;
                }
                *sc.bits.add(so) = ((*bits.add(co) as u32 * 255 + a / 2) / a).min(255) as u8;
                *sc.bits.add(so + 1) = ((*bits.add(co + 1) as u32 * 255 + a / 2) / a).min(255) as u8;
                *sc.bits.add(so + 2) = ((*bits.add(co + 2) as u32 * 255 + a / 2) / a).min(255) as u8;
                *sc.bits.add(so + 3) = 255;
            }
        }
        let f = font(size, bold);
        let old = SelectObject(sc.dc, f);
        SetBkMode(sc.dc, TRANSPARENT as i32);
        SetTextColor(sc.dc, r | (g << 8) | (b << 16));
        let v: Vec<u16> = s.encode_utf16().collect();
        let mut rc = RECT { left: 0, top: 0, right: tw, bottom: th };
        DrawTextW(sc.dc, v.as_ptr(), v.len() as i32, &mut rc, DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX);
        SelectObject(sc.dc, old);
        for yy in y0..y1 {
            let srow = (yy - dy) as usize * sstride;
            let crow = yy as usize * cw * 4;
            for xx in x0..x1 {
                let so = srow + (xx - dx) as usize * 4;
                let co = crow + xx as usize * 4;
                let pa = *bits.add(co + 3) as u32;
                if pa == 0 {
                    continue;
                }
                *bits.add(co) = (*sc.bits.add(so) as u32 * pa / 255) as u8;
                *bits.add(co + 1) = (*sc.bits.add(so + 1) as u32 * pa / 255) as u8;
                *bits.add(co + 2) = (*sc.bits.add(so + 2) as u32 * pa / 255) as u8;
            }
        }
    });
    (tw, th)
}

pub unsafe fn text(c: &Canvas, s: &str, x: i32, y: i32, size: f32, bold: bool, color: u32, align: u32) -> (i32, i32) {
    text_px(c.bits, c.w as usize, c.h as usize, s, x, y, size, bold, color, align, None)
}

fn blur_h(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize) {
    let div = (2 * r + 1) as i32;
    let wm = w as i32 - 1;
    for y in 0..h {
        let row = &src[y * w * 4..(y + 1) * w * 4];
        let out = &mut dst[y * w * 4..(y + 1) * w * 4];
        let mut s = [0i32; 3];
        for k in -(r as i32)..=(r as i32) {
            let x = k.clamp(0, wm) as usize * 4;
            for c in 0..3 {
                s[c] += row[x + c] as i32;
            }
        }
        for x in 0..w {
            let o = x * 4;
            for c in 0..3 {
                out[o + c] = (s[c] / div) as u8;
            }
            out[o + 3] = 255;
            let xr = ((x as i32 - r as i32).clamp(0, wm) as usize) * 4;
            let xa = ((x as i32 + r as i32 + 1).clamp(0, wm) as usize) * 4;
            for c in 0..3 {
                s[c] += row[xa + c] as i32 - row[xr + c] as i32;
            }
        }
    }
}

fn blur_v(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize) {
    let div = (2 * r + 1) as i32;
    let hm = h as i32 - 1;
    for x in 0..w {
        let o = x * 4;
        let mut s = [0i32; 3];
        for k in -(r as i32)..=(r as i32) {
            let y = k.clamp(0, hm) as usize * w * 4;
            for c in 0..3 {
                s[c] += src[y + o + c] as i32;
            }
        }
        for y in 0..h {
            let d = y * w * 4 + o;
            for c in 0..3 {
                dst[d + c] = (s[c] / div) as u8;
            }
            dst[d + 3] = 255;
            let yr = ((y as i32 - r as i32).clamp(0, hm) as usize) * w * 4;
            let ya = ((y as i32 + r as i32 + 1).clamp(0, hm) as usize) * w * 4;
            for c in 0..3 {
                s[c] += src[ya + o + c] as i32 - src[yr + o + c] as i32;
            }
        }
    }
}

pub fn blur(src: &[u8], w: usize, h: usize, r: usize, passes: usize) -> Vec<u8> {
    let mut a = src.to_vec();
    if r == 0 || passes == 0 {
        for p in a.chunks_exact_mut(4) {
            p[3] = 255;
        }
        return a;
    }
    let mut b = vec![0u8; a.len()];
    for _ in 0..passes {
        blur_h(&a, &mut b, w, h, r);
        blur_v(&b, &mut a, w, h, r);
    }
    a
}

pub fn round_mask(buf: &mut [u8], w: usize, h: usize, radius: f32) {
    let hw = w as f32 / 2.0;
    let hh = h as f32 / 2.0;
    let r = radius.max(0.0).min(hw).min(hh);
    for y in 0..h {
        for x in 0..w {
            let px = (x as f32 + 0.5) - hw;
            let py = (y as f32 + 0.5) - hh;
            let qx = px.abs() - (hw - r);
            let qy = py.abs() - (hh - r);
            let d = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r;
            let a = (((0.5 - d).clamp(0.0, 1.0)) * 255.0 + 0.5) as u32;
            let i = (y * w + x) * 4;
            for c in 0..3 {
                buf[i + c] = ((buf[i + c] as u32 * a + 127) / 255) as u8;
            }
            buf[i + 3] = a as u8;
        }
    }
}

pub fn tint(buf: &mut [u8], color: u32, a: f32) {
    let a = a.clamp(0.0, 1.0);
    let (r, g, b) = (((color >> 16) & 0xFF) as f32, ((color >> 8) & 0xFF) as f32, (color & 0xFF) as f32);
    for p in buf.chunks_exact_mut(4) {
        p[0] = (p[0] as f32 * (1.0 - a) + b * a + 0.5) as u8;
        p[1] = (p[1] as f32 * (1.0 - a) + g * a + 0.5) as u8;
        p[2] = (p[2] as f32 * (1.0 - a) + r * a + 0.5) as u8;
        p[3] = 255;
    }
}

pub fn over(dst: &mut [u8], dw: usize, dh: usize, src: &[u8], sw: usize, sh: usize, ox: usize, oy: usize) {
    for y in 0..sh {
        let dy = oy + y;
        if dy >= dh {
            break;
        }
        for x in 0..sw {
            let dx = ox + x;
            if dx >= dw {
                break;
            }
            let si = (y * sw + x) * 4;
            let di = (dy * dw + dx) * 4;
            let a = src[si + 3] as u32;
            if a == 0 {
                continue;
            }
            let da = dst[di + 3] as u32;
            let inv = 255 - a;
            for c in 0..3 {
                dst[di + c] = (src[si + c] as u32 + (dst[di + c] as u32 * inv + 127) / 255).min(255) as u8;
            }
            dst[di + 3] = (a + (da * inv + 127) / 255).min(255) as u8;
        }
    }
}

pub unsafe fn acrylic(
    src: *const u8,
    sw: i32,
    sh: i32,
    rect: (i32, i32, i32, i32),
    radius: f32,
    color: u32,
    tint_a: f32,
    blur_r: usize,
) -> (Vec<u8>, usize, usize) {    let (rx, ry, rw, rh) = rect;
    let (rw, rh) = (rw.max(1) as usize, rh.max(1) as usize);
    let mut buf = vec![0u8; rw * rh * 4];
    for y in 0..rh {
        for x in 0..rw {
            let (sx, sy) = (rx + x as i32, ry + y as i32);
            let i = (y * rw + x) * 4;
            if sx >= 0 && sy >= 0 && sx < sw && sy < sh {
                let o = (sy as usize * sw as usize + sx as usize) * 4;
                buf[i..i + 4].copy_from_slice(std::slice::from_raw_parts(src.add(o), 4));
            } else {
                buf[i] = 0x0E;
                buf[i + 1] = 0x0E;
                buf[i + 2] = 0x12;
                buf[i + 3] = 255;
            }
        }
    }
    buf = blur(&buf, rw, rh, blur_r, 2);
    tint(&mut buf, color, tint_a);
    round_mask(&mut buf, rw, rh, radius);
    (buf, rw, rh)
}

pub unsafe fn com_release(p: *mut std::ffi::c_void) {
    if p.is_null() {
        return;
    }
    let vt = *(p as *mut *mut std::ffi::c_void);
    let fp = *(vt as *mut *mut std::ffi::c_void).add(2);
    let rel: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32 = std::mem::transmute(fp);
    rel(p);
}

fn coverage(x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let px = x + 0.5 - w / 2.0;
    let py = y + 0.5 - h / 2.0;
    let qx = px.abs() - (w / 2.0 - r);
    let qy = py.abs() - (h / 2.0 - r);
    let d = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r;
    (0.5 - d).clamp(0.0, 1.0)
}

pub unsafe fn copy_rounded(dst: *mut u8, dw: usize, src: *const u8, rect: (i32, i32, i32, i32), radius: f32) {
    let (x, y, w, h) = rect;
    if w <= 0 || h <= 0 {
        return;
    }
    let r = radius.min(w as f32 / 2.0).min(h as f32 / 2.0);
    let ri = r.ceil() as i32;
    for yy in 0..h {
        let row = ((y + yy) as usize * dw + x as usize) * 4;
        for xx in 0..w {
            let corner = (xx < ri || xx >= w - ri) && (yy < ri || yy >= h - ri);
            let o = row + xx as usize * 4;
            if !corner {
                std::ptr::copy_nonoverlapping(src.add(o), dst.add(o), 4);
                continue;
            }
            let cov = coverage(xx as f32, yy as f32, w as f32, h as f32, r);
            if cov >= 0.999 {
                std::ptr::copy_nonoverlapping(src.add(o), dst.add(o), 4);
            } else if cov > 0.001 {
                let c = (cov * 255.0) as u32;
                let inv = 255 - c;
                for k in 0..3 {
                    let s = *src.add(o + k) as u32;
                    let d = *dst.add(o + k) as u32;
                    *dst.add(o + k) = ((s * c + d * inv + 127) / 255) as u8;
                }
            }
        }
    }
}

pub unsafe fn mosaic(bits: *mut u8, w: usize, h: usize, rect: (i32, i32, i32, i32), block: i32, clip: (i32, i32, i32, i32)) {
    let (rx, ry, rw, rh) = rect;
    let b = block.max(2);
    let x0 = rx.max(clip.0).max(0);
    let y0 = ry.max(clip.1).max(0);
    let x1 = (rx + rw).min(clip.0 + clip.2).min(w as i32);
    let y1 = (ry + rh).min(clip.1 + clip.3).min(h as i32);
    let mut by = ry;
    while by < y1 {
        let mut bx = rx;
        while bx < x1 {
            let (ex, ey) = ((bx + b).min(rx + rw).min(x1), (by + b).min(ry + rh).min(y1));
            let (sx, sy) = (bx.max(x0), by.max(y0));
            if ex > sx && ey > sy {
                let (mut sb, mut sg, mut sr, mut n) = (0u64, 0u64, 0u64, 0u64);
                for yy in sy..ey {
                    let row = (yy as usize * w) * 4;
                    for xx in sx..ex {
                        let o = row + xx as usize * 4;
                        sb += *bits.add(o) as u64;
                        sg += *bits.add(o + 1) as u64;
                        sr += *bits.add(o + 2) as u64;
                        n += 1;
                    }
                }
                let (ab, ag, ar) = ((sb / n) as u8, (sg / n) as u8, (sr / n) as u8);
                for yy in sy..ey {
                    let row = (yy as usize * w) * 4;
                    for xx in sx..ex {
                        let o = row + xx as usize * 4;
                        *bits.add(o) = ab;
                        *bits.add(o + 1) = ag;
                        *bits.add(o + 2) = ar;
                    }
                }
            }
            bx += b;
        }
        by += b;
    }
}
