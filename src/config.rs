use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::win::*;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Bind {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub win: bool,
    pub vk: u32,
}

impl Bind {
    pub fn mods(&self) -> u32 {
        let mut m: u32 = 0;
        if self.ctrl {
            m |= MOD_CONTROL;
        }
        if self.shift {
            m |= MOD_SHIFT;
        }
        if self.alt {
            m |= MOD_ALT;
        }
        if self.win {
            m |= MOD_WIN;
        }
        m | MOD_NOREPEAT
    }

    pub fn parts(&self) -> Vec<String> {
        let mut p: Vec<String> = Vec::new();
        if self.ctrl {
            p.push("Ctrl".to_string());
        }
        if self.shift {
            p.push("Shift".to_string());
        }
        if self.alt {
            p.push("Alt".to_string());
        }
        if self.win {
            p.push("Win".to_string());
        }
        p.push(vk_name(self.vk));
        p
    }

    pub fn label(&self) -> String {
        self.parts().join(" + ")
    }
}

pub fn vk_name(vk: u32) -> String {
    match vk {
        8 => "Backspace".into(),
        9 => "Tab".into(),
        13 => "Enter".into(),
        16 => "Shift".into(),
        17 => "Ctrl".into(),
        18 => "Alt".into(),
        19 => "Pause".into(),
        20 => "Caps Lock".into(),
        27 => "Esc".into(),
        32 => "Space".into(),
        33 => "PgUp".into(),
        34 => "PgDn".into(),
        35 => "End".into(),
        36 => "Home".into(),
        37 => "Left".into(),
        38 => "Up".into(),
        39 => "Right".into(),
        40 => "Down".into(),
        44 => "PrtScn".into(),
        45 => "Ins".into(),
        46 => "Del".into(),
        91 | 92 => "Win".into(),
        93 => "Menu".into(),
        96..=105 => format!("Num {}", vk - 96),
        106 => "Num *".into(),
        107 => "Num +".into(),
        109 => "Num -".into(),
        110 => "Num .".into(),
        111 => "Num /".into(),
        112..=135 => format!("F{}", vk - 111),
        144 => "Num Lock".into(),
        145 => "Scroll".into(),
        48..=57 => char::from(vk as u8).to_string(),
        65..=90 => char::from(vk as u8).to_string(),
        186 => ";".into(),
        187 => "=".into(),
        188 => ",".into(),
        189 => "-".into(),
        190 => ".".into(),
        191 => "/".into(),
        192 => "`".into(),
        219 => "[".into(),
        220 => "\\".into(),
        221 => "]".into(),
        222 => "'".into(),
        _ => format!("Key 0x{:02X}", vk),
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub region: Bind,
    pub fullscreen: Bind,
    pub repeat: Bind,
    pub autostart: bool,
    pub sound: bool,
    pub ask_name: bool,
    pub jpg: bool,
    pub save_dir: Option<String>,
    pub loupe: bool,
    pub dim: u8,
    pub toolbar_top: bool,
    pub last: Option<[i32; 4]>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            region: Bind { ctrl: false, shift: false, alt: false, win: false, vk: 44 },
            fullscreen: Bind { ctrl: true, shift: true, alt: false, win: false, vk: 70 },
            repeat: Bind { ctrl: true, shift: true, alt: false, win: false, vk: 82 },
            autostart: false,
            sound: true,
            ask_name: true,
            jpg: false,
            save_dir: None,
            loupe: true,
            dim: 1,
            toolbar_top: false,
            last: None,
        }
    }
}

impl Config {
    pub fn ext(&self) -> &'static str {
        if self.jpg {
            "jpg"
        } else {
            "png"
        }
    }

    pub fn save_dir(&self) -> PathBuf {
        match &self.save_dir {
            Some(d) if !d.is_empty() => PathBuf::from(d),
            _ => {
                let base = std::env::var("USERPROFILE").unwrap_or_default();
                PathBuf::from(base).join("Pictures")
            }
        }
    }

    pub fn binds(&self) -> [Bind; 3] {
        [self.region, self.fullscreen, self.repeat]
    }
}

static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

pub fn dir() -> PathBuf {
    let base = std::env::var("APPDATA").unwrap_or_else(|_| std::env::var("USERPROFILE").unwrap_or_default());
    PathBuf::from(base).join("Kerf")
}

fn path() -> PathBuf {
    dir().join("config.json")
}

pub fn load() {
    let mut c = Config::default();
    if let Ok(s) = fs::read_to_string(path()) {
        if let Ok(v) = serde_json::from_str::<Config>(&s) {
            c = v;
        }
    }
    *CONFIG.lock().unwrap() = Some(c);
    save();
}

pub fn get() -> Config {
    CONFIG.lock().unwrap().clone().unwrap_or_default()
}

pub fn update(f: impl FnOnce(&mut Config)) {
    let mut g = CONFIG.lock().unwrap();
    if let Some(c) = g.as_mut() {
        f(c);
    }
    drop(g);
    save();
}

pub fn save() {
    if let Some(c) = CONFIG.lock().unwrap().as_ref() {
        if let Ok(s) = serde_json::to_string_pretty(c) {
            let _ = fs::create_dir_all(dir());
            let _ = fs::write(path(), s);
        }
    }
}

pub unsafe fn apply_autostart(on: bool) {
    let mut key: HKEY = std::ptr::null_mut();
    let sub = w("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    if RegCreateKeyExW(
        HKEY_CURRENT_USER,
        sub.as_ptr(),
        0,
        std::ptr::null(),
        0,
        KEY_WRITE,
        std::ptr::null(),
        &mut key,
        std::ptr::null_mut(),
    ) != 0
    {
        return;
    }
    if on {
        let mut buf = [0u16; 1024];
        let n = GetModuleFileNameW(std::ptr::null_mut(), buf.as_mut_ptr(), 1024) as usize;
        let p = format!("\"{}\"", String::from_utf16_lossy(&buf[..n]));
        let bytes: Vec<u8> = p.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        RegSetValueExW(key, w("Kerf").as_ptr(), 0, REG_SZ, bytes.as_ptr(), bytes.len() as u32);
    } else {
        RegDeleteValueW(key, w("Kerf").as_ptr());
    }
    RegCloseKey(key);
}
