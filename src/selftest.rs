use crate::capture;
use crate::gfx as gp;
use crate::menu;
use crate::settings;
use crate::toast::{self, Kind};
use crate::ui;
use crate::win::*;

pub unsafe fn run(dir: &str) {
    let shot = capture::Shot::grab();
    capture::write_png(shot.c.bits, shot.w(), shot.h(), &format!("{}\\capture.png", dir));
    let core = crate::CORE.load(std::sync::atomic::Ordering::Relaxed) as HWND;
    let tb = crate::toolbar::create(core);
    crate::toolbar::set_state(tb, 1, 3, 1, 2, 1);
    crate::toolbar::set_size(tb, 1440, 810);
    crate::toolbar::place(tb, 240, 200, shot.c.bits, shot.w(), shot.h(), shot.vx, shot.vy);
    if let Some((px, w, h)) = crate::toolbar::snapshot(tb) {
        capture::write_png(px.as_ptr(), w, h, &format!("{}\\toolbar.png", dir));
    }
    crate::toolbar::hide(tb);
    DestroyWindow(tb);
    settings::open();
    let sh = settings::hwnd();
    if !sh.is_null() {
        std::thread::sleep(std::time::Duration::from_millis(400));
        if let Some((px, w, h)) = settings::snapshot(sh) {
            capture::write_png(px.as_ptr(), w, h, &format!("{}\\settings.png", dir));
        }
        DestroyWindow(sh);
    }
    menu::show(core);
    let mh = menu::hwnd();
    if !mh.is_null() {
        std::thread::sleep(std::time::Duration::from_millis(300));
        if let Some((px, w, h)) = menu::snapshot(mh) {
            capture::write_png(px.as_ptr(), w, h, &format!("{}\\menu.png", dir));
        }
        DestroyWindow(mh);
    }
    toast::show("Copied to clipboard", Kind::Ok, shot.w() / 2, shot.h() / 2);
    std::thread::sleep(std::time::Duration::from_millis(300));
    if let Some((px, w, h)) = toast::snapshot() {
        capture::write_png(px.as_ptr(), w, h, &format!("{}\\toast.png", dir));
    }
    let icon = gp::Canvas::new(ui::si(256.0), ui::si(256.0));
    if let Some(g) = gp::Gfx::from_dc(icon.dc) {
        gp::setup(g.raw);
        crate::icons::logo(g.raw, 0.0, 0.0, icon.w as f32, true);
    }
    capture::write_png(icon.bits, icon.w, icon.h, &format!("{}\\icon.png", dir));
}
