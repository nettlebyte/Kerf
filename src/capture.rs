use crate::gfx as gp;
use crate::shapes::Shape;
use crate::win::*;

pub struct Shot {
    pub c: gp::Canvas,
    pub vx: i32,
    pub vy: i32,
}

impl Shot {
    pub fn w(&self) -> i32 {
        self.c.w
    }

    pub fn h(&self) -> i32 {
        self.c.h
    }

    pub unsafe fn grab() -> Shot {
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        Shot::rect(vx, vy, GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN))
    }

    pub unsafe fn rect(x: i32, y: i32, w: i32, h: i32) -> Shot {
        let c = gp::Canvas::new(w, h);
        let sdc = GetDC(std::ptr::null_mut());
        BitBlt(c.dc, 0, 0, c.w, c.h, sdc, x, y, SRCCOPY | CAPTUREBLT);
        ReleaseDC(std::ptr::null_mut(), sdc);
        for p in c.px().chunks_exact_mut(4) {
            p[3] = 255;
        }
        Shot { c, vx: x, vy: y }
    }
}

pub struct Crop {
    pub img: *mut GpImage,
    pub g: *mut GpGraphics,
    pub buf: Vec<u8>,
    pub w: i32,
    pub h: i32,
}

impl Drop for Crop {
    fn drop(&mut self) {
        unsafe {
            if !self.g.is_null() {
                GdipDeleteGraphics(self.g);
            }
            if !self.img.is_null() {
                GdipDisposeImage(self.img);
            }
        }
    }
}

pub unsafe fn crop(shot: &Shot, r: (i32, i32, i32, i32), shapes: &[Shape]) -> Crop {
    let (rx, ry, rw, rh) = (r.0, r.1, r.2.max(1), r.3.max(1));
    let (sw, sh) = (shot.w(), shot.h());
    let stride = rw as usize * 4;
    let mut buf: Vec<u8> = Vec::with_capacity(stride * rh as usize);
    for y in ry..ry + rh {
        if y < 0 || y >= sh {
            buf.extend(std::iter::repeat(0u8).take(stride));
            continue;
        }
        let mut row = vec![0u8; stride];
        let xa = rx.max(0);
        let xb = (rx + rw).min(sw);
        if xb > xa {
            let src = shot.c.bits.offset(((y * sw + xa) * 4) as isize);
            let dst = (xa - rx) as usize * 4;
            row[dst..dst + ((xb - xa) * 4) as usize]
                .copy_from_slice(std::slice::from_raw_parts(src, ((xb - xa) * 4) as usize));
        }
        buf.extend_from_slice(&row);
    }
    let mut bmp: *mut GpBitmap = std::ptr::null_mut();
    GdipCreateBitmapFromScan0(rw, rh, stride as i32, PXF_32ARGB, buf.as_ptr(), &mut bmp);
    let img = bmp as *mut GpImage;
    let mut g: *mut GpGraphics = std::ptr::null_mut();
    if !img.is_null() {
        GdipGetImageGraphicsContext(img, &mut g);
        if !g.is_null() {
            gp::setup(g);
            GdipTranslateWorldTransform(g, (shot.vx - rx) as f32, (shot.vy - ry) as f32, MatrixOrderPrepend);
            let origin = ((shot.vx - rx) as f32, (shot.vy - ry) as f32);
            for s in shapes {
                crate::shapes::draw(buf.as_mut_ptr(), rw as usize, rh as usize, g, origin, None, s);
            }
            GdipResetWorldTransform(g);
        }
    }
    Crop { img, g, buf, w: rw, h: rh }
}

pub unsafe fn dib_bytes(c: &Crop) -> Vec<u8> {
    let hdr = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: c.w,
        biHeight: c.h,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        biSizeImage: (c.w * c.h * 4) as u32,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    let mut out: Vec<u8> = Vec::with_capacity(40 + c.buf.len());
    out.extend_from_slice(std::slice::from_raw_parts(&hdr as *const _ as *const u8, 40));
    let stride = c.w as usize * 4;
    for y in (0..c.h).rev() {
        let s = y as usize * stride;
        out.extend_from_slice(&c.buf[s..s + stride]);
    }
    out
}

fn clsid(jpg: bool) -> GUID {
    if jpg {
        GUID { data1: 0x557CF401, data2: 0x1A04, data3: 0x11D3, data4: [0x9A, 0x73, 0x00, 0x00, 0xF8, 0x1E, 0xF3, 0x2E] }
    } else {
        GUID { data1: 0x557CF406, data2: 0x1A04, data3: 0x11D3, data4: [0x9A, 0x73, 0x00, 0x00, 0xF8, 0x1E, 0xF3, 0x2E] }
    }
}

unsafe fn encoder(jpg: bool, q: &mut u32) -> *const EncoderParameters {
    if !jpg {
        return std::ptr::null();
    }
    let ep = Box::new(EncoderParameters {
        Count: 1,
        Parameter: [EncoderParameter {
            Guid: EncoderQuality,
            NumberOfValues: 1,
            Type: EncoderParameterValueTypeLong as u32,
            Value: q as *mut u32 as *mut std::ffi::c_void,
        }],
    });
    Box::into_raw(ep)
}

unsafe fn free_encoder(jpg: bool, ep: *const EncoderParameters) {
    if jpg && !ep.is_null() {
        drop(Box::from_raw(ep as *mut EncoderParameters));
    }
}

pub unsafe fn encode(c: &Crop, jpg: bool) -> Option<Vec<u8>> {
    if c.img.is_null() {
        return None;
    }
    let mut stm: *mut std::ffi::c_void = std::ptr::null_mut();
    if CreateStreamOnHGlobal(std::ptr::null_mut(), 0, &mut stm) != 0 {
        return None;
    }
    let cls = clsid(jpg);
    let mut q = 93u32;
    let ep = encoder(jpg, &mut q);
    let st = GdipSaveImageToStream(c.img, stm, &cls, ep);
    free_encoder(jpg, ep);
    if st != 0 {
        gp::com_release(stm);
        return None;
    }
    let mut hg: HGLOBAL = std::ptr::null_mut();
    if GetHGlobalFromStream(stm, &mut hg) != 0 {
        gp::com_release(stm);
        return None;
    }
    let size = GlobalSize(hg);
    let p = GlobalLock(hg);
    let mut out: Vec<u8> = Vec::new();
    if !p.is_null() {
        out.extend_from_slice(std::slice::from_raw_parts(p as *const u8, size));
        GlobalUnlock(hg);
    }
    gp::com_release(stm);
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

pub unsafe fn save(c: &Crop, path: &str, jpg: bool) -> bool {
    if c.img.is_null() {
        return false;
    }
    let p = crate::win::w(path);
    let cls = clsid(jpg);
    let mut q = 93u32;
    let ep = encoder(jpg, &mut q);
    let ok = GdipSaveImageToFile(c.img, p.as_ptr(), &cls, ep) == 0;
    free_encoder(jpg, ep);
    ok
}

pub unsafe fn write_png(bits: *const u8, wd: i32, ht: i32, path: &str) -> bool {
    let mut bmp: *mut GpBitmap = std::ptr::null_mut();
    GdipCreateBitmapFromScan0(wd, ht, wd * 4, PXF_32ARGB, bits, &mut bmp);
    if bmp.is_null() {
        return false;
    }
    let cls = clsid(false);
    let p = w(path);
    let ok = GdipSaveImageToFile(bmp as *mut GpImage, p.as_ptr(), &cls, std::ptr::null()) == 0;
    GdipDisposeImage(bmp as *mut GpImage);
    ok
}

unsafe fn to_hglobal(bytes: &[u8]) -> Option<HGLOBAL> {
    let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
    if h.is_null() {
        return None;
    }
    let p = GlobalLock(h);
    if p.is_null() {
        GlobalFree(h);
        return None;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), p as *mut u8, bytes.len());
    GlobalUnlock(h);
    Some(h)
}

pub unsafe fn clipboard_put(dib: &[u8], png: &[u8]) -> bool {
    if OpenClipboard(std::ptr::null_mut()) == 0 {
        return false;
    }
    EmptyClipboard();
    let mut ok = false;
    if let Some(h) = to_hglobal(dib) {
        if !SetClipboardData(8, h).is_null() {
            ok = true;
        } else {
            GlobalFree(h);
        }
    }
    let fmt = RegisterClipboardFormatW(w("PNG").as_ptr());
    if fmt != 0 && !png.is_empty() {
        if let Some(h) = to_hglobal(png) {
            if SetClipboardData(fmt, h).is_null() {
                GlobalFree(h);
            }
        }
    }
    CloseClipboard();
    ok
}

pub fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86400);
    let sod = secs.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        y,
        m,
        d,
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}
