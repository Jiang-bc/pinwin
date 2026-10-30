//! Effects owned by one pin operation. A window property prevents restoring a reused HWND.
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{ApiTopMost, ffi::*};

static NEXT_TOKEN: AtomicUsize = AtomicUsize::new(1);
const PROPERTY: &[u16] = &[80, 105, 110, 87, 105, 110, 46, 79, 119, 110, 101, 114, 0];

#[derive(Clone)]
pub struct WindowState {
    token: usize,
    process: u32,
    thread: u32,
    style: u32,
    layered: Option<(u32, u8, u32)>,
}

impl WindowState {
    pub fn capture(hwnd: HWND) -> Option<Self> {
        unsafe {
            let mut process = 0;
            let thread = GetWindowThreadProcessId(hwnd, &mut process);
            if thread == 0 {
                return None;
            }
            let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            let (mut color, mut alpha, mut flags) = (0, 255, 0);
            let layered = if style & WS_EX_LAYERED != 0 {
                // Per-pixel layered windows cannot be faithfully restored using constant alpha.
                if GetLayeredWindowAttributes(hwnd, &mut color, &mut alpha, &mut flags) == FALSE {
                    return None;
                }
                Some((color, alpha, flags))
            } else {
                None
            };
            let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
            if SetPropW(hwnd, PROPERTY.as_ptr(), token as HANDLE) == FALSE {
                return None;
            }
            Some(Self {
                token,
                process,
                thread,
                style,
                layered,
            })
        }
    }

    pub fn matches(&self, hwnd: HWND) -> bool {
        unsafe {
            let mut process = 0;
            GetWindowThreadProcessId(hwnd, &mut process) == self.thread
                && process == self.process
                && GetPropW(hwnd, PROPERTY.as_ptr()) as usize == self.token
        }
    }

    pub fn was_click_through(&self) -> bool {
        self.style & WS_EX_TRANSPARENT != 0
    }

    pub fn restore(&self, hwnd: HWND) -> bool {
        if !self.matches(hwnd) {
            return false;
        }
        unsafe {
            let current = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
            let mask = WS_EX_LAYERED | WS_EX_TRANSPARENT;
            SetLastError(0);
            let result = SetWindowLongW(
                hwnd,
                GWL_EXSTYLE,
                ((current & !mask) | (self.style & mask)) as i32,
            );
            let mut ok = result != 0 || GetLastError() == 0;
            if let Some((color, alpha, flags)) = self.layered {
                ok &= SetLayeredWindowAttributes(hwnd, color, alpha, flags) != FALSE;
            }
            ok &= if self.style & WS_EX_TOPMOST != 0 {
                ApiTopMost::set(hwnd)
            } else {
                ApiTopMost::clear(hwnd)
            };
            RemovePropW(hwnd, PROPERTY.as_ptr());
            ok
        }
    }
}
