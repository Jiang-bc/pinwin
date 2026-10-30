//! Safe, ergonomic Rust wrappers over Win32 APIs.
//!
//! Ported and enhanced from `PinWin/WinApi/*.cs`, `PinWin/BusinessLayer/WindowStyle.cs`,
//! `PinWin/BusinessLayer/SingleInstanceManager.cs`, and `PinWin/BusinessLayer/ScreenCapture.cs`.

// SDK handle pointers are opaque tokens validated by Windows; these wrappers never dereference them.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod ffi;
pub mod resources;
pub mod window_state;

use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use crate::algorithms::HierarchyLookup;
use crate::keys::KeyCombo;
use crate::relative_path::{choose_optimal_path, resolve_asset_path};
use ffi::*;

/// Embedded original PinWin visual assets (`Pinned.bmp`, `Tray.ico`, `TargetWindowIcon.ico`, `PinWin.ico`).
pub static EMBEDDED_PINNED_BMP: &[u8] = include_bytes!("../../PinWin/Images/Pinned.bmp");
pub static EMBEDDED_TRAY_ICO: &[u8] = include_bytes!("../../PinWin/Images/Tray.ico");
pub static EMBEDDED_TARGET_CURSOR_ICO: &[u8] =
    include_bytes!("../../PinWin/Images/TargetWindowIcon.ico");

/// Convert a Rust `&str` to a null-terminated UTF-16 `Vec<u16>`.
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Convert a null-terminated or fixed-length UTF-16 slice to a Rust `String`.
pub fn from_wide(slice: &[u16]) -> String {
    let len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    String::from_utf16_lossy(&slice[..len])
}

/// Configure the current process for Per-Monitor V2 DPI awareness (with fallback to `SetProcessDPIAware`).
/// Fixes original PinWin Known Issue #17 on multi-monitor setups with mixed DPI scaling.
pub fn enable_high_dpi_awareness() {
    unsafe {
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == FALSE {
            let _ = SetProcessDPIAware();
        }
    }
}

/// Single-instance mutex guard (`PinWin/BusinessLayer/SingleInstanceManager.cs`).
pub struct SingleInstanceManager {
    mutex_handle: HANDLE,
    has_ownership: bool,
}

impl SingleInstanceManager {
    pub fn new(mutex_name: &str) -> Self {
        let wide_name = to_wide(mutex_name);
        let handle = unsafe { CreateMutexW(null(), FALSE, wide_name.as_ptr()) };
        Self {
            mutex_handle: handle,
            has_ownership: false,
        }
    }

    /// Attempt to acquire single-instance lock within `timeout_ms`.
    pub fn is_first_instance(&mut self, timeout_ms: u32) -> bool {
        if self.mutex_handle.is_null() {
            return false;
        }
        let res = unsafe { WaitForSingleObject(self.mutex_handle, timeout_ms) };
        self.has_ownership = res == WAIT_OBJECT_0 || res == WAIT_ABANDONED;
        self.has_ownership
    }
}

impl Drop for SingleInstanceManager {
    fn drop(&mut self) {
        if !self.mutex_handle.is_null() {
            unsafe {
                if self.has_ownership {
                    let _ = ReleaseMutex(self.mutex_handle);
                }
                let _ = CloseHandle(self.mutex_handle);
            }
            self.mutex_handle = std::ptr::null_mut();
        }
    }
}

/// TopMost manipulation (`PinWin/WinApi/ApiTopMost.cs`) plus opacity & click-through extensions.
pub struct ApiTopMost;

impl ApiTopMost {
    /// Set external window as TopMost (`HWND_TOPMOST`).
    pub fn set(hwnd: HWND) -> bool {
        if hwnd.is_null() || !ApiWindowPos::is_window(hwnd) {
            return false;
        }
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            ) != FALSE
        }
    }

    /// Clear TopMost status for an external window (`HWND_NOTOPMOST`).
    pub fn clear(hwnd: HWND) -> bool {
        if hwnd.is_null() || !ApiWindowPos::is_window(hwnd) {
            return false;
        }
        unsafe {
            SetWindowPos(
                hwnd,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            ) != FALSE
        }
    }

    /// Check whether `hwnd` currently has `WS_EX_TOPMOST`.
    pub fn is_topmost(hwnd: HWND) -> bool {
        if hwnd.is_null() || !ApiWindowPos::is_window(hwnd) {
            return false;
        }
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 };
        (ex_style & WS_EX_TOPMOST) != 0
    }
}

/// Window opacity and click-through (ghost mode) helper.
pub struct ApiWindowEffects;

impl ApiWindowEffects {
    /// Set window opacity (`alpha` in `30..=255`).
    /// When `alpha == 255` and click-through is not enabled, removes `WS_EX_LAYERED` for zero overhead.
    pub fn set_opacity(hwnd: HWND, alpha: u8) -> bool {
        if hwnd.is_null() || !ApiWindowPos::is_window(hwnd) {
            return false;
        }
        let clamped = alpha.clamp(30, 255);
        unsafe {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            if clamped == 255 && (ex_style & WS_EX_TRANSPARENT) == 0 {
                if (ex_style & WS_EX_LAYERED) != 0 {
                    SetWindowLongW(hwnd, GWL_EXSTYLE, (ex_style & !WS_EX_LAYERED) as i32);
                }
                return true;
            }
            if (ex_style & WS_EX_LAYERED) == 0 {
                SetWindowLongW(hwnd, GWL_EXSTYLE, (ex_style | WS_EX_LAYERED) as i32);
            }
            if SetLayeredWindowAttributes(hwnd, 0, clamped, LWA_ALPHA) != FALSE {
                return true;
            }
            // Fallback for headless/non-DWM display drivers (ERROR_GEN_FAILURE = 31 on repeat calls):
            // Reset the WS_EX_LAYERED bit and re-apply SetLayeredWindowAttributes.
            SetWindowLongW(hwnd, GWL_EXSTYLE, (ex_style & !WS_EX_LAYERED) as i32);
            SetWindowLongW(hwnd, GWL_EXSTYLE, (ex_style | WS_EX_LAYERED) as i32);
            SetLayeredWindowAttributes(hwnd, 0, clamped, LWA_ALPHA) != FALSE
        }
    }

    /// Enable or disable Click-Through (Ghost Mode) via `WS_EX_TRANSPARENT | WS_EX_LAYERED`.
    pub fn set_click_through(hwnd: HWND, enabled: bool, current_alpha: u8) -> bool {
        if hwnd.is_null() || !ApiWindowPos::is_window(hwnd) {
            return false;
        }
        let clamped = current_alpha.clamp(30, 255);
        unsafe {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            let new_style = if enabled {
                ex_style | WS_EX_LAYERED | WS_EX_TRANSPARENT
            } else if clamped == 255 {
                ex_style & !(WS_EX_TRANSPARENT | WS_EX_LAYERED)
            } else {
                ex_style & !WS_EX_TRANSPARENT
            };
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_style as i32);
            if (new_style & WS_EX_LAYERED) != 0
                && SetLayeredWindowAttributes(hwnd, 0, clamped, LWA_ALPHA) == FALSE
            {
                SetWindowLongW(hwnd, GWL_EXSTYLE, (new_style & !WS_EX_LAYERED) as i32);
                SetWindowLongW(hwnd, GWL_EXSTYLE, new_style as i32);
                let _ = SetLayeredWindowAttributes(hwnd, 0, clamped, LWA_ALPHA);
            }
            true
        }
    }

    pub fn is_click_through(hwnd: HWND) -> bool {
        if hwnd.is_null() || !ApiWindowPos::is_window(hwnd) {
            return false;
        }
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 };
        (ex_style & WS_EX_TRANSPARENT) != 0
    }
}

/// Window position & DWM bounds (`PinWin/WinApi/ApiWindowPos.cs`).
pub struct ApiWindowPos;

impl ApiWindowPos {
    /// Get real visible window bounds using `DWMWA_EXTENDED_FRAME_BOUNDS` with fallback to `GetWindowRect`.
    pub fn get(hwnd: HWND) -> RECT {
        if hwnd.is_null() {
            return RECT::default();
        }
        let mut rect = RECT::default();
        unsafe {
            let hr = DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS.try_into().unwrap(),
                &mut rect as *mut RECT as *mut c_void,
                size_of::<RECT>() as u32,
            );
            if hr >= 0 && rect.width() > 0 && rect.height() > 0 {
                return rect;
            }
            let _ = GetWindowRect(hwnd, &mut rect);
        }
        rect
    }

    pub fn is_window(hwnd: HWND) -> bool {
        !hwnd.is_null() && unsafe { IsWindow(hwnd) != FALSE }
    }

    pub fn is_iconic(hwnd: HWND) -> bool {
        !hwnd.is_null() && unsafe { IsIconic(hwnd) != FALSE }
    }

    pub fn is_visible(hwnd: HWND) -> bool {
        if hwnd.is_null() || !Self::is_window(hwnd) {
            return false;
        }
        unsafe {
            IsWindowVisible(hwnd) != FALSE
                || (GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_VISIBLE) != 0
        }
    }

    pub fn is_cloaked(hwnd: HWND) -> bool {
        if hwnd.is_null() {
            return false;
        }
        let mut cloaked: u32 = 0;
        let hr = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED.try_into().unwrap(),
                &mut cloaked as *mut u32 as *mut c_void,
                size_of::<u32>() as u32,
            )
        };
        // Only treat DWM_CLOAKED_APP (0x1) as cloaked; DWM_CLOAKED_SHELL (0x2) is set on all windows in non-console/SSH sessions.
        hr >= 0 && (cloaked & 1) != 0
    }

    /// Get virtual desktop bounds across all monitors (`PinWin/BusinessLayer/ScreenCapture.cs`).
    pub fn get_desktop_bounds() -> RECT {
        unsafe {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            if w > 0 && h > 0 {
                RECT::new(x, y, x + w, y + h)
            } else {
                let sw = GetSystemMetrics(SM_CXSCREEN);
                let sh = GetSystemMetrics(SM_CYSCREEN);
                RECT::new(0, 0, sw.max(800), sh.max(600))
            }
        }
    }
}

/// System metrics & caption button box width (`PinWin/WinApi/ApiSystemMetrics.cs` & `WindowStyle.cs`).
pub struct WindowStyle {
    pub handle: HWND,
    window_style_raw: u32,
}

impl WindowStyle {
    pub fn new(handle: HWND) -> Self {
        let style = if !handle.is_null() {
            unsafe { GetWindowLongW(handle, GWL_STYLE) as u32 }
        } else {
            0
        };
        Self {
            handle,
            window_style_raw: style,
        }
    }

    pub const fn maximize_box(&self) -> bool {
        (self.window_style_raw & WS_MAXIMIZEBOX) != 0
    }

    pub const fn minimize_box(&self) -> bool {
        (self.window_style_raw & WS_MINIMIZEBOX) != 0
    }

    /// Compute total width of the window's caption button area (Minimize, Maximize, Close).
    pub fn get_control_box_width(&self) -> i32 {
        if !self.handle.is_null() {
            let mut rc = RECT::default();
            let hr = unsafe {
                DwmGetWindowAttribute(
                    self.handle,
                    DWMWA_CAPTION_BUTTON_BOUNDS.try_into().unwrap(),
                    &mut rc as *mut RECT as *mut c_void,
                    size_of::<RECT>() as u32,
                )
            };
            if hr == 0 && rc.width() > 0 {
                return rc.width();
            }
        }

        let button_count = if self.maximize_box() || self.minimize_box() {
            3
        } else {
            1
        };
        let avg_width = unsafe { GetSystemMetrics(SM_CXSIZE) }.max(28);
        avg_width * button_count
    }

    /// Compute absolute screen position for the Pin badge on `handle`'s title bar.
    /// Matches `PinForm.GetRelativeLocation(IntPtr handle)` in original PinWin, with bounds clamping.
    pub fn get_pin_badge_location(handle: HWND) -> POINT {
        let style = Self::new(handle);
        let right_margin = style.get_control_box_width() + 28;
        let rect = ApiWindowPos::get(handle);
        let min_x = rect.left + 8;
        let x = (rect.right - right_margin).max(min_x);
        let y = rect.top + 5;
        POINT { x, y }
    }
}

/// Non-blocking window title lookup (`PinWin/WinApi/ApiWindowTitle.cs`).
///
/// **Optimization**: Replaces blocking `SendMessage(WM_GETTEXT)` with `SendMessageTimeoutW`
/// (`SMTO_ABORTIFHUNG`, 200ms) so hung 3rd-party processes can never freeze PinWin.
pub struct ApiWindowTitle;

impl ApiWindowTitle {
    pub fn from_handle(handle: HWND) -> String {
        if handle.is_null() || !ApiWindowPos::is_window(handle) {
            return String::new();
        }

        unsafe {
            let mut len_result: usize = 0;
            let ok = SendMessageTimeoutW(
                handle,
                WM_GETTEXTLENGTH,
                0,
                0,
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                200,
                &mut len_result,
            );
            if ok != 0 && len_result > 0 && len_result < 4096 {
                let cap = len_result + 1;
                let mut buf = vec![0u16; cap];
                let mut copied: usize = 0;
                let ok2 = SendMessageTimeoutW(
                    handle,
                    WM_GETTEXT,
                    cap,
                    buf.as_mut_ptr() as LPARAM,
                    SMTO_ABORTIFHUNG | SMTO_BLOCK,
                    200,
                    &mut copied,
                );
                if ok2 != 0 {
                    let s = from_wide(&buf);
                    if !s.is_empty() {
                        return s;
                    }
                }
            }

            // Fallback to GetWindowTextW (reads window title from window manager cache without IPC)
            let len = GetWindowTextLengthW(handle);
            if len > 0 && len < 4096 {
                let mut buf = vec![0u16; (len + 1) as usize];
                let n = GetWindowTextW(handle, buf.as_mut_ptr(), len + 1);
                if n > 0 {
                    return from_wide(&buf[..n as usize]);
                }
            }
        }
        String::new()
    }

    pub fn class_name(handle: HWND) -> String {
        if handle.is_null() {
            return String::new();
        }
        let mut buf = [0u16; 256];
        let n = unsafe { GetClassNameW(handle, buf.as_mut_ptr(), buf.len() as i32) };
        if n > 0 {
            from_wide(&buf[..n as usize])
        } else {
            String::new()
        }
    }
}

/// Parent/Owner window lookup (`PinWin/WinApi/ApiOwnerFormLookup.cs` & `ApiForm.cs`).
/// Implements `HierarchyLookup<HWND>` using Win32 `GetParent`.
pub struct ApiOwnerFormLookup;

impl HierarchyLookup<HWND> for ApiOwnerFormLookup {
    fn get_parent_element(&self, child: HWND) -> HWND {
        if child.is_null() {
            return std::ptr::null_mut();
        }
        unsafe { GetParent(child) }
    }

    fn stop_condition(&self, parent: HWND) -> bool {
        parent.is_null()
    }
}

/// Locate the top-level window at screen `point` (`PinWin/WinApi/ApiForm.cs`).
pub struct ApiForm;

impl ApiForm {
    pub fn select_excluding(point: POINT, excluded: HWND) -> HWND {
        unsafe {
            let mut hwnd = GetTopWindow(std::ptr::null_mut());
            while !hwnd.is_null() {
                if hwnd != excluded && IsWindowVisible(hwnd) != FALSE && IsIconic(hwnd) == FALSE {
                    let rect = ApiWindowPos::get(hwnd);
                    if point.x >= rect.left
                        && point.x < rect.right
                        && point.y >= rect.top
                        && point.y < rect.bottom
                    {
                        return hwnd;
                    }
                }
                hwnd = GetWindow(hwnd, GW_HWNDNEXT);
            }
        }
        std::ptr::null_mut()
    }

    pub fn select(point: POINT) -> HWND {
        let found = unsafe { WindowFromPoint(point) };
        if found.is_null() {
            return std::ptr::null_mut();
        }
        let lookup = ApiOwnerFormLookup;
        let root = lookup.find_parent(found);
        if !root.is_null() {
            root
        } else {
            unsafe { GetAncestor(found, GA_ROOT) }
        }
    }
}

/// Global HotKey registration (`PinWin/WinApi/ApiHotKey.cs`).
pub struct ApiHotKey;

impl ApiHotKey {
    pub fn set(hwnd: HWND, combo: KeyCombo, key_id: i32) -> bool {
        if hwnd.is_null() || combo.is_none() {
            return false;
        }
        unsafe { RegisterHotKey(hwnd, key_id, combo.modifiers, combo.vk) != FALSE }
    }

    pub fn clear(hwnd: HWND, key_id: i32) {
        if !hwnd.is_null() {
            unsafe {
                let _ = UnregisterHotKey(hwnd, key_id);
            }
        }
    }
}

/// Load icon / cursor / bitmap resources from embedded bytes or custom file path.
pub struct ApiResources;

impl ApiResources {
    /// Load an `HICON` from a custom file path (if non-empty and valid) or from embedded `.ico` bytes.
    pub fn load_icon_or_embedded(custom_path: &str, embedded_ico: &[u8], size: i32) -> HICON {
        if let Some(resolved) = resolve_asset_path(custom_path) {
            let wide = to_wide(&resolved.to_string_lossy());
            let h = unsafe {
                LoadImageW(
                    std::ptr::null_mut(),
                    wide.as_ptr(),
                    IMAGE_ICON,
                    size,
                    size,
                    LR_LOADFROMFILE | if size == 0 { LR_DEFAULTSIZE } else { 0 },
                )
            };
            if !h.is_null() {
                return h;
            }
        }
        Self::create_icon_from_ico_bytes(embedded_ico, true, size)
    }

    /// Load a cursor from `custom_path` (`ApiCursor.LoadFromPath`) or from embedded `.ico` bytes.
    pub fn load_cursor_or_embedded(custom_path: &str, embedded_ico: &[u8]) -> HCURSOR {
        if let Some(resolved) = resolve_asset_path(custom_path) {
            let wide = to_wide(&resolved.to_string_lossy());
            let hc = unsafe { LoadCursorFromFileW(wide.as_ptr()) };
            if !hc.is_null() {
                return hc;
            }
        }
        unsafe {
            let Some(icon) = resources::OwnedIcon::from_owned(Self::create_icon_from_ico_bytes(
                embedded_ico,
                true,
                32,
            )) else {
                return CopyIcon(LoadCursorW(null_mut(), IDC_CROSS));
            };
            let mut info: ICONINFO = zeroed();
            if GetIconInfo(icon.as_raw(), &mut info) == FALSE {
                return null_mut();
            }
            let _mask = resources::OwnedBitmap::from_owned(info.hbmMask);
            let _color = resources::OwnedBitmap::from_owned(info.hbmColor);
            info.fIcon = FALSE;
            info.xHotspot = 16;
            info.yHotspot = 16;
            CreateIconIndirect(&info)
        }
    }

    /// Validate an ICO directory before handing its selected frame to Windows.
    /// Invalid data returns a null handle; successful handles are always owned.
    pub fn create_icon_from_ico_bytes(bytes: &[u8], is_icon: bool, desired_size: i32) -> HICON {
        if !is_icon || bytes.len() < 6 || bytes[..4] != [0, 0, 1, 0] {
            return null_mut();
        }
        let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        let directory_end = 6 + count * 16;
        if count == 0 || directory_end > bytes.len() {
            return null_mut();
        }
        let mut frames = Vec::with_capacity(count);
        for entry in bytes[6..directory_end].as_chunks::<16>().0 {
            let length = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
            let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
            let Some(end) = offset.checked_add(length) else {
                return null_mut();
            };
            if length == 0 || offset < directory_end || end > bytes.len() {
                return null_mut();
            }
            let width = if entry[0] == 0 { 256 } else { entry[0] as i32 };
            frames.push(((width - desired_size).abs(), offset, end));
        }
        let (_, offset, end) = frames.into_iter().min_by_key(|frame| frame.0).unwrap();
        unsafe {
            CreateIconFromResourceEx(
                bytes[offset..end].as_ptr(),
                (end - offset) as u32,
                TRUE,
                0x0003_0000,
                desired_size,
                desired_size,
                0,
            )
        }
    }

    /// Decode the embedded 24-bit `Pinned.bmp` into a GDI `HBITMAP` and return `(hbitmap, width, height, colorkey)`.
    pub fn create_pinned_bitmap_from_embedded() -> (HBITMAP, i32, i32, COLORREF) {
        let bytes = EMBEDDED_PINNED_BMP;
        if bytes.len() < 54 || &bytes[0..2] != b"BM" {
            return (std::ptr::null_mut(), 43, 42, rgb(255, 0, 255));
        }
        let data_offset = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
        let width = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
        let height = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]).abs();
        let bpp = u16::from_le_bytes([bytes[28], bytes[29]]);

        if width <= 0 || height <= 0 || data_offset >= bytes.len() {
            return (std::ptr::null_mut(), 43, 42, rgb(255, 0, 255));
        }

        unsafe {
            let screen_dc = GetDC(std::ptr::null_mut());
            let mut bmi: BITMAPINFO = zeroed();
            bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width;
            bmi.bmiHeader.biHeight = height; // bottom-up DIB matches standard BMP
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = bpp;
            bmi.bmiHeader.biCompression = BI_RGB;

            let mut bits_ptr: *mut c_void = null_mut();
            let hbmp = CreateDIBSection(
                screen_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits_ptr,
                std::ptr::null_mut(),
                0,
            );
            if !hbmp.is_null() && !bits_ptr.is_null() {
                let pixel_data = &bytes[data_offset..];
                let stride = ((width as usize * (bpp as usize / 8) + 3) & !3) * height as usize;
                let copy_len = pixel_data.len().min(stride);
                std::ptr::copy_nonoverlapping(pixel_data.as_ptr(), bits_ptr as *mut u8, copy_len);
            }

            // Determine top-left pixel color for MakeTransparent() compatibility
            let mem_dc = CreateCompatibleDC(screen_dc);
            let old = SelectObject(mem_dc, hbmp);
            let colorkey = GetPixel(mem_dc, 0, 0);
            SelectObject(mem_dc, old);
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);

            (hbmp, width, height, colorkey)
        }
    }
}

/// Native Win32 OpenFileDialog (`PinWin/BusinessLayer/FileFromDisk.cs`).
pub fn select_file_from_disk(owner: HWND, title: &str) -> Option<String> {
    unsafe {
        let mut file_buf = [0u16; 520];
        let filter_str =
            "Icon & Cursor Files (*.ico;*.cur;*.bmp)\0*.ico;*.cur;*.bmp\0All Files (*.*)\0*.*\0\0";
        let filter_wide: Vec<u16> = filter_str.encode_utf16().collect();
        let title_wide = to_wide(title);
        let cwd = std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let cwd_wide = to_wide(&cwd);

        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = owner;
        ofn.lpstrFilter = filter_wide.as_ptr();
        ofn.lpstrFile = file_buf.as_mut_ptr();
        ofn.nMaxFile = file_buf.len() as u32;
        ofn.lpstrInitialDir = if cwd.is_empty() {
            null()
        } else {
            cwd_wide.as_ptr()
        };
        ofn.lpstrTitle = title_wide.as_ptr();
        ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;

        if GetOpenFileNameW(&mut ofn) != FALSE {
            let selected = from_wide(&file_buf);
            if !selected.is_empty() {
                return Some(choose_optimal_path(&cwd, &selected));
            }
        }
    }
    None
}

/// Capture the entire virtual desktop into an offscreen `HBITMAP` (`ApiScreenCapture.CreateBitmap`).
pub fn capture_desktop_bitmap(bounds: RECT) -> HBITMAP {
    let w = bounds.width();
    let h = bounds.height();
    if w <= 0 || h <= 0 {
        return std::ptr::null_mut();
    }
    unsafe {
        let h_desk = GetDesktopWindow();
        let h_src = GetWindowDC(h_desk);
        let h_dest = CreateCompatibleDC(h_src);
        let h_bmp = CreateCompatibleBitmap(h_src, w, h);
        let h_old = SelectObject(h_dest, h_bmp);
        let _ = BitBlt(
            h_dest,
            0,
            0,
            w,
            h,
            h_src,
            bounds.left,
            bounds.top,
            SRCCOPY | CAPTUREBLT,
        );
        SelectObject(h_dest, h_old);
        DeleteDC(h_dest);
        ReleaseDC(h_desk, h_src);
        h_bmp
    }
}

/// Information about a top-level visible window on the system.
#[derive(Clone)]
pub struct WindowInfo {
    pub hwnd: HWND,
    pub title: String,
    pub class_name: String,
    pub is_topmost: bool,
    pub is_minimized: bool,
    pub bounds: RECT,
}

/// Enumerate all visible, titled top-level windows (useful for CLI `--list` / `--pin-title` and tests).
pub fn enumerate_top_level_windows() -> Vec<WindowInfo> {
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let list = &mut *(lparam as *mut Vec<WindowInfo>);
            if ApiWindowPos::is_visible(hwnd) && !ApiWindowPos::is_cloaked(hwnd) {
                let title = ApiWindowTitle::from_handle(hwnd);
                if !title.trim().is_empty() {
                    list.push(WindowInfo {
                        hwnd,
                        title,
                        class_name: ApiWindowTitle::class_name(hwnd),
                        is_topmost: ApiTopMost::is_topmost(hwnd),
                        is_minimized: ApiWindowPos::is_iconic(hwnd),
                        bounds: ApiWindowPos::get(hwnd),
                    });
                }
            }
            TRUE
        }
    }

    let mut list: Vec<WindowInfo> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(enum_proc), &mut list as *mut Vec<WindowInfo> as LPARAM);
    }
    list
}
