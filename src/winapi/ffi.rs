//! Official Windows SDK bindings, plus small value helpers.
#![allow(non_snake_case, clippy::upper_case_acronyms, unused_imports)]

pub use windows_sys::Win32::Foundation::*;
pub use windows_sys::Win32::Graphics::Dwm::*;
pub use windows_sys::Win32::Graphics::Gdi::*;
pub use windows_sys::Win32::System::LibraryLoader::*;
pub use windows_sys::Win32::System::SystemServices::{SS_CENTER, SS_ICON, SS_LEFT};
pub use windows_sys::Win32::System::Threading::*;
pub use windows_sys::Win32::UI::Accessibility::*;
pub use windows_sys::Win32::UI::Controls::Dialogs::*;
pub use windows_sys::Win32::UI::Controls::*;
pub use windows_sys::Win32::UI::HiDpi::*;
pub use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
pub use windows_sys::Win32::UI::Shell::*;
pub use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub type LONG = i32;
pub type DWORD = u32;
pub type UINT = u32;
pub type BOOL = i32;
pub type COLORREF = u32;

pub trait RectExt {
    fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self;
    fn width(&self) -> i32;
    fn height(&self) -> i32;
}
impl RectExt for RECT {
    fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
    fn width(&self) -> i32 {
        self.right - self.left
    }
    fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

pub const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}
pub const fn loword(value: usize) -> u16 {
    value as u16
}
pub const fn hiword(value: usize) -> u16 {
    (value >> 16) as u16
}
pub const fn get_x_lparam(value: LPARAM) -> i32 {
    value as i16 as i32
}
pub const fn get_y_lparam(value: LPARAM) -> i32 {
    (value >> 16) as i16 as i32
}
pub const fn get_wheel_delta_wparam(value: WPARAM) -> i16 {
    hiword(value) as i16
}
