pub use windows_sys::core::GUID;

pub use windows_sys::Win32::Foundation::{
    ERROR_ALREADY_EXISTS, GlobalFree, HGLOBAL, HWND, LPARAM, LRESULT, POINT, RECT,
    SIZE, TRUE, WPARAM, GetLastError,
};

pub use windows_sys::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BLENDFUNCTION, BITMAPINFO, BITMAPINFOHEADER, CAPTUREBLT,
    CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DT_BOTTOM, DT_CALCRECT, DT_CENTER, DT_LEFT,
    DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_TOP, DT_VCENTER, DIB_RGB_COLORS, FF_DONTCARE,
    FW_BOLD, FW_REGULAR, GetMonitorInfoW, HBITMAP, HDC, HFONT, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, OUT_DEFAULT_PRECIS, SRCCOPY, ScreenToClient, BitBlt, CreateCompatibleDC,
    CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, DrawTextW, GetDC, MonitorFromPoint,
    ReleaseDC, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};

pub use windows_sys::Win32::Graphics::GdiPlus::{
    CombineModeReplace, DashStyleCustom, EncoderParameter, EncoderParameters,
    EncoderParameterValueTypeLong, EncoderQuality, FillModeAlternate, FlushIntentionFlush,
    GdipAddPathArc, GdipAddPathEllipse, GdipAddPathPath, GdipAddPathRectangle, GdipCreateBitmapFromScan0, GdipCreateFromHDC,
    GdipCreateLineBrushFromRect, GdipCreatePath, GdipCreatePen1, GdipCreateSolidFill,
    GdipDeleteBrush, GdipDeleteGraphics, GdipDeletePath, GdipDeletePen, GdipDisposeImage,
    GdipDrawArc, GdipDrawEllipse, GdipDrawImageRectRectI, GdipDrawLine, GdipDrawLines,
    GdipDrawPath, GdipDrawRectangle, GdipFillEllipse, GdipFillPath, GdipFillRectangle, GdipFlush,
    GdipGetImageGraphicsContext, GdipResetClip, GdipResetWorldTransform, GdipSaveImageToFile,
    GdipSaveImageToStream, GdipSetClipPath, GdipSetInterpolationMode,
    GdipSetPenDashArray, GdipSetPenDashStyle, GdipSetPenEndCap, GdipSetPenStartCap,
    GdipSetPixelOffsetMode, GdipSetSmoothingMode, GdiplusStartup, GdiplusStartupInput,
    GdipTranslateWorldTransform, GpBitmap, GpBrush, GpGraphics, GpImage, GpLineGradient, GpPath,
    GpPen, GpSolidFill, InterpolationModeDefault, InterpolationModeNearestNeighbor,
    LinearGradientModeVertical, LineCapFlat, LineCapRound, MatrixOrderPrepend, PixelOffsetModeHalf,
    PointF, RectF, SmoothingModeAntiAlias, UnitPixel, WrapModeTile,
};

pub use windows_sys::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};

pub use windows_sys::Win32::UI::Controls::Dialogs::{
    GetSaveFileNameW, OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OPENFILENAMEW,
};

pub use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;

pub use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

pub use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey,
    ReleaseCapture, SetCapture, SetFocus, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
    UnregisterHotKey, VIRTUAL_KEY, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_LEFT,
    VK_LWIN, VK_MENU, VK_RETURN, VK_RIGHT, VK_RWIN, VK_SHIFT, VK_TAB, VK_UP,
};

pub use windows_sys::Win32::UI::Shell::{
    BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD,
    NIM_DELETE, NOTIFYICONDATAW, SHBrowseForFolderW, SHGetPathFromIDListW, Shell_NotifyIconW,
};

pub use windows_sys::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CreateIconIndirect, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, EnumWindows, FindWindowW, GetClassNameW, GetCursorPos,
    GetMessageW, GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, GWL_EXSTYLE, GWLP_USERDATA, HICON, HWND_NOTOPMOST, HWND_TOP, HWND_TOPMOST, ICONINFO,
    IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_IBEAM, IDC_SIZEALL, IDC_SIZENESW, IDC_SIZENWSE, IDC_SIZENS,
    IDC_SIZEWE, IsIconic, IsWindowVisible, KillTimer, LoadCursorW, MA_ACTIVATE, MSG, PostMessageW,
    PostQuitMessage, RegisterClassW, SW_HIDE, SW_SHOW, SW_SHOWNOACTIVATE,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SetCursor, SetForegroundWindow, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, ULW_ALPHA, UpdateLayeredWindow,
    WA_INACTIVE, WM_ACTIVATE, WM_APP, WM_CHAR, WM_CONTEXTMENU, WM_DESTROY, WM_HOTKEY, WM_KEYDOWN,
    WM_KILLFOCUS, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE,
    WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR,
    WM_SYSKEYDOWN, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

pub use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree};
pub use windows_sys::Win32::System::Com::StructuredStorage::{
    CreateStreamOnHGlobal, GetHGlobalFromStream,
};
pub use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
pub use windows_sys::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleW, GetProcAddress, LoadLibraryA,
};
pub use windows_sys::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
pub use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteValueW,
    RegSetValueExW,
};
pub use windows_sys::Win32::System::Threading::{CreateMutexExW, GetCurrentProcessId};

pub const WM_MOUSELEAVE: u32 = 0x02A3;
pub const PXF_32ARGB: i32 = 0x26200A;
pub const CLEARTYPE_QUALITY: u32 = 5;

pub fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn lp_x(l: LPARAM) -> i32 {
    ((l as i32) & 0xFFFF) as u16 as i16 as i32
}

pub fn lp_y(l: LPARAM) -> i32 {
    (((l as i32) >> 16) & 0xFFFF) as u16 as i16 as i32
}

pub fn vk_down(vk: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) & 0x8000u16 as i16 != 0 }
}

pub unsafe fn track_leave(hwnd: HWND) {
    let mut t = TRACKMOUSEEVENT {
        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dwFlags: TME_LEAVE,
        hwndTrack: hwnd,
        dwHoverTime: 0,
    };
    TrackMouseEvent(&mut t);
}
