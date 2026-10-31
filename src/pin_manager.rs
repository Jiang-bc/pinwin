//! Pinned window state manager, floating Pin badge (`PinForm`), and WinEventHook engine.
//!
//! Ported and enhanced from:
//! - `PinWin/Controls/PinForm.cs`
//! - `PinWin/Controls/PinnedWindowListControl.cs`
//! - `PinWin/BusinessLayer/WinEventHook.cs`
//! - `PinWin/BusinessLayer/WinEventHookHandler.cs`

use std::cell::RefCell;
use std::mem::{size_of, zeroed};
use std::ptr::null;

use crate::winapi::ffi::*;
use crate::winapi::resources::{OwnedBitmap, OwnedHook, OwnedMenu};
use crate::winapi::window_state::WindowState;
use crate::winapi::{
    ApiTopMost, ApiWindowEffects, ApiWindowPos, ApiWindowTitle, WindowStyle, to_wide,
};

const PIN_BADGE_CLASS: &str = "PinWin_BadgeWindow_Class";

/// Notification message posted to the main window whenever pinned state changes.
pub const WM_PINWIN_LIST_CHANGED: UINT = WM_APP + 101;
/// Notification message posted when a Pin badge is clicked or requests unpinning.
pub const WM_PINWIN_BADGE_UNPIN: UINT = WM_APP + 102;
pub const WM_PINWIN_WINDOW_EVENT: UINT = WM_APP + 103;
pub const WM_PINWIN_UNPIN_ALL: UINT = WM_APP + 104;

/// Menu command IDs for the Pin badge right-click quick menu.
const IDM_BADGE_UNPIN: usize = 5001;
const IDM_BADGE_OPACITY_100: usize = 5002;
const IDM_BADGE_OPACITY_85: usize = 5003;
const IDM_BADGE_OPACITY_70: usize = 5004;
const IDM_BADGE_OPACITY_50: usize = 5005;
const IDM_BADGE_GHOST_TOGGLE: usize = 5006;

/// Represents a single pinned external window and its attached `PinBadge` overlay.
#[derive(Clone)]
pub struct PinnedWindowEntry {
    pub target_hwnd: HWND,
    pub badge_hwnd: HWND,
    pub title: String,
    pub opacity: u8,
    pub click_through: bool,
    pub last_badge_pos: POINT,
    original: WindowState,
}

impl PinnedWindowEntry {
    /// Formatted label displayed in `MainForm`'s `lstMain` listbox.
    pub fn display_label(&self) -> String {
        let base_title = if self.title.trim().is_empty() {
            format!("Window (0x{:X})", self.target_hwnd as usize)
        } else {
            self.title.clone()
        };

        let mut tags = Vec::new();
        if self.opacity < 255 {
            let pct = ((self.opacity as u32 * 100) + 127) / 255;
            tags.push(format!("{}%", pct));
        }
        if self.click_through {
            tags.push("Ghost".to_string());
        }

        if tags.is_empty() {
            base_title
        } else {
            format!("{} [{}]", base_title, tags.join(", "))
        }
    }
}

struct GlobalHookState {
    notify_hwnd: HWND,
    badge_icon: HICON,
    owned_icon: Option<crate::winapi::resources::OwnedIcon>,
    badge_width: i32,
    badge_height: i32,
    entries: Vec<PinnedWindowEntry>,
    hooks: Vec<OwnedHook>,
    class_registered: bool,
}

impl Default for GlobalHookState {
    fn default() -> Self {
        Self {
            notify_hwnd: std::ptr::null_mut(),
            badge_icon: std::ptr::null_mut(),
            owned_icon: None,
            badge_width: 43,
            badge_height: 42,
            entries: Vec::new(),
            hooks: Vec::new(),
            class_registered: false,
        }
    }
}

thread_local! {
    static STATE: RefCell<GlobalHookState> = RefCell::new(GlobalHookState::default());
}

/// High-performance event-driven manager for pinned windows and their Pin badges.
pub struct PinManager;

impl PinManager {
    /// Initialize the Pin badge window class, embedded bitmap, and global `WinEventHook` listeners.
    pub fn init(notify_hwnd: HWND) {
        let (need_bitmap, need_class, need_hooks) = STATE.with(|c| {
            let mut st = c.borrow_mut();
            st.notify_hwnd = notify_hwnd;
            (
                st.badge_icon.is_null(),
                !st.class_registered,
                st.hooks.is_empty(),
            )
        });
        // Windows can synchronously call our procedures: never hold a RefCell borrow here.
        if need_bitmap {
            let icon =
                crate::winapi::resources::load_embedded_icon(crate::winapi::EMBEDDED_APP_ICO, 64)
                    .ok();
            STATE.with(|c| {
                let mut st = c.borrow_mut();
                st.badge_icon = icon.as_ref().map_or(std::ptr::null_mut(), |i| i.as_raw());
                st.owned_icon = icon;
                st.badge_width = 24;
                st.badge_height = 24;
            });
        }
        if need_class {
            register_badge_window_class();
            STATE.with(|c| c.borrow_mut().class_registered = true);
        }
        if need_hooks {
            let hooks = install_win_event_hooks()
                .into_iter()
                .filter_map(|h| unsafe { OwnedHook::from_owned(h) })
                .collect();
            STATE.with(|c| c.borrow_mut().hooks = hooks);
        }
    }

    /// Shut down all hooks and restore windows before releasing resources.
    pub fn shutdown() {
        Self::unpin_all();
        let (hooks, bitmap) = STATE.with(|c| {
            let mut st = c.borrow_mut();
            st.badge_icon = std::ptr::null_mut();
            (std::mem::take(&mut st.hooks), st.owned_icon.take())
        });
        drop(hooks);
        drop(bitmap);
    }

    /// Return a snapshot of all currently pinned windows.
    pub fn entries() -> Vec<PinnedWindowEntry> {
        STATE.with(|cell| cell.borrow().entries.clone())
    }

    /// Return the count of currently pinned windows.
    pub fn count() -> usize {
        STATE.with(|cell| cell.borrow().entries.len())
    }

    /// Check if `hwnd` is either one of our own Pin badge windows or already pinned.
    pub fn is_badge_window(hwnd: HWND) -> bool {
        STATE.with(|cell| cell.borrow().entries.iter().any(|e| e.badge_hwnd == hwnd))
    }

    pub fn is_pinned(target_hwnd: HWND) -> bool {
        STATE.with(|cell| {
            cell.borrow()
                .entries
                .iter()
                .any(|e| e.target_hwnd == target_hwnd)
        })
    }

    /// Pin a target window (or unpin if `toggle_if_pinned` is true and it is already pinned).
    /// Returns `Ok(true)` if newly pinned, `Ok(false)` if unpinned or already pinned, `Err` if invalid.
    pub fn pin_window(
        target_hwnd: HWND,
        default_opacity: u8,
        toggle_if_pinned: bool,
    ) -> Result<bool, &'static str> {
        if target_hwnd.is_null() || !ApiWindowPos::is_window(target_hwnd) {
            return Err("Target window handle is invalid.");
        }

        // Ignore desktop / shell window or our own main window / badge windows
        let notify_hwnd = STATE.with(|c| c.borrow().notify_hwnd);
        if target_hwnd == notify_hwnd || target_hwnd == unsafe { GetDesktopWindow() } {
            return Err("Cannot pin desktop or PinWin window itself.");
        }
        if Self::is_badge_window(target_hwnd) {
            return Err("Cannot pin a PinBadge window.");
        }

        if Self::is_pinned(target_hwnd) && !Self::identity_matches(target_hwnd) {
            Self::unpin_window(target_hwnd);
        }
        if Self::is_pinned(target_hwnd) {
            if toggle_if_pinned {
                Self::unpin_window(target_hwnd);
                return Ok(false);
            }
            // Refresh TopMost & badge position
            let _ = ApiTopMost::set(target_hwnd);
            Self::sync_window(target_hwnd);
            return Ok(false);
        }

        let title = ApiWindowTitle::from_handle(target_hwnd);
        if title.trim().is_empty() {
            return Err("Window has no title or cannot be pinned.");
        }

        let original = WindowState::capture(target_hwnd)
            .ok_or("Cannot safely capture window effects or acquire window ownership.")?;
        if !ApiTopMost::set(target_hwnd) {
            original.restore(target_hwnd);
            return Err(
                "Failed to set WS_EX_TOPMOST on target window (check elevation/permissions).",
            );
        }

        let opacity = default_opacity.clamp(30, 255);
        if opacity < 255 && !ApiWindowEffects::set_opacity(target_hwnd, opacity) {
            original.restore(target_hwnd);
            return Err("Failed to apply window opacity; original effects restored.");
        }

        let badge_pos = WindowStyle::get_pin_badge_location(target_hwnd);
        let badge_hwnd = create_badge_window(target_hwnd, badge_pos);
        if badge_hwnd.is_null() {
            original.restore(target_hwnd);
            return Err("Failed to create pin badge; original effects restored.");
        }
        let click_through = original.was_click_through();

        STATE.with(|cell| {
            let mut st = cell.borrow_mut();
            st.entries.push(PinnedWindowEntry {
                target_hwnd,
                badge_hwnd,
                title,
                opacity,
                click_through,
                last_badge_pos: badge_pos,
                original,
            });
        });

        Self::sync_window(target_hwnd);
        notify_list_changed();
        Ok(true)
    }

    /// Unpin a specific window, restoring its normal z-order, opacity, and click-through state.
    pub fn unpin_window(target_hwnd: HWND) -> bool {
        let removed = STATE.with(|cell| {
            let mut st = cell.borrow_mut();
            if let Some(idx) = st.entries.iter().position(|e| e.target_hwnd == target_hwnd) {
                Some(st.entries.remove(idx))
            } else {
                None
            }
        });

        if let Some(entry) = removed {
            if !entry.badge_hwnd.is_null() && ApiWindowPos::is_window(entry.badge_hwnd) {
                unsafe {
                    let _ = DestroyWindow(entry.badge_hwnd);
                }
            }
            entry.original.restore(entry.target_hwnd);
            notify_list_changed();
            true
        } else {
            false
        }
    }

    /// Unpin all currently pinned windows.
    pub fn unpin_all() {
        let targets: Vec<HWND> = STATE.with(|cell| {
            cell.borrow()
                .entries
                .iter()
                .map(|e| e.target_hwnd)
                .collect()
        });
        for hwnd in targets {
            Self::unpin_window(hwnd);
        }
    }

    /// Adjust opacity (`30..=255`) for a pinned window.
    pub fn set_window_opacity(target_hwnd: HWND, opacity: u8) -> bool {
        let clamped = opacity.clamp(30, 255);
        if !Self::identity_matches(target_hwnd) {
            Self::unpin_window(target_hwnd);
            return false;
        }
        if !ApiWindowEffects::set_opacity(target_hwnd, clamped) {
            return false;
        }
        STATE.with(|cell| {
            let mut st = cell.borrow_mut();
            if let Some(entry) = st.entries.iter_mut().find(|e| e.target_hwnd == target_hwnd) {
                entry.opacity = clamped;
            }
        });
        notify_list_changed();
        true
    }

    /// Toggle or set click-through (Ghost Mode) for a pinned window.
    pub fn set_window_click_through(target_hwnd: HWND, enabled: bool) -> bool {
        if !Self::identity_matches(target_hwnd) {
            Self::unpin_window(target_hwnd);
            return false;
        }
        let cur_opacity = STATE.with(|cell| {
            cell.borrow()
                .entries
                .iter()
                .find(|e| e.target_hwnd == target_hwnd)
                .map(|e| e.opacity)
        });
        let Some(opacity) = cur_opacity else {
            return false;
        };
        // Ghost Mode only toggles mouse click-through; keep opacity untouched.
        let effective_alpha = opacity;
        if !ApiWindowEffects::set_click_through(target_hwnd, enabled, effective_alpha) {
            return false;
        }
        STATE.with(|cell| {
            let mut st = cell.borrow_mut();
            if let Some(entry) = st.entries.iter_mut().find(|e| e.target_hwnd == target_hwnd) {
                entry.click_through = enabled;
                entry.opacity = effective_alpha;
            }
        });
        notify_list_changed();
        true
    }

    /// Restore normal clickability on all pinned windows that have Ghost Mode enabled.
    pub fn restore_all_click_through() {
        let ghosted: Vec<HWND> = STATE.with(|cell| {
            cell.borrow()
                .entries
                .iter()
                .filter(|e| e.click_through)
                .map(|e| e.target_hwnd)
                .collect()
        });
        for hwnd in ghosted {
            Self::set_window_click_through(hwnd, false);
        }
    }

    /// Synchronize a single pinned window's Pin badge position and TopMost state.
    pub fn sync_window(target_hwnd: HWND) {
        if !Self::identity_matches(target_hwnd) {
            Self::unpin_window(target_hwnd);
            return;
        }

        let snapshot = STATE.with(|cell| {
            let st = cell.borrow();
            st.entries
                .iter()
                .find(|e| e.target_hwnd == target_hwnd)
                .map(|e| {
                    (
                        e.badge_hwnd,
                        e.title.clone(),
                        st.badge_width,
                        st.badge_height,
                    )
                })
        });

        let Some((badge_hwnd, old_title, _, _)) = snapshot else {
            return;
        };
        let dpi = ApiWindowPos::dpi(target_hwnd);
        let badge_w = (24 * dpi as i32 + 48) / 96;
        let badge_h = badge_w;

        if ApiWindowPos::is_iconic(target_hwnd) || !ApiWindowPos::is_visible(target_hwnd) {
            if !badge_hwnd.is_null() {
                unsafe {
                    let _ = ShowWindow(badge_hwnd, SW_HIDE);
                }
            }
            return;
        }

        if !ApiTopMost::is_topmost(target_hwnd) {
            let _ = ApiTopMost::set(target_hwnd);
        }

        let new_pos = WindowStyle::get_pin_badge_location(target_hwnd);
        if !badge_hwnd.is_null() && ApiWindowPos::is_window(badge_hwnd) {
            unsafe {
                let _ = SetWindowPos(
                    badge_hwnd,
                    HWND_TOPMOST,
                    new_pos.x,
                    new_pos.y,
                    badge_w,
                    badge_h,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }

        let live_title = ApiWindowTitle::from_handle(target_hwnd);
        let title_changed = !live_title.is_empty() && live_title != old_title;

        STATE.with(|cell| {
            let mut st = cell.borrow_mut();
            if let Some(entry) = st.entries.iter_mut().find(|e| e.target_hwnd == target_hwnd) {
                entry.last_badge_pos = new_pos;
                if title_changed {
                    entry.title = live_title;
                }
            }
        });

        if title_changed {
            notify_list_changed();
        }
    }

    fn identity_matches(hwnd: HWND) -> bool {
        let snapshot = STATE.with(|c| {
            c.borrow()
                .entries
                .iter()
                .find(|e| e.target_hwnd == hwnd)
                .map(|e| e.original.clone())
        });
        snapshot.is_some_and(|state| state.matches(hwnd))
    }

    /// Periodic lightweight health check for all pinned windows (runs every 250ms instead of 10ms + Sleep(1)).
    pub fn sync_all() {
        let targets: Vec<HWND> = STATE.with(|cell| {
            cell.borrow()
                .entries
                .iter()
                .map(|e| e.target_hwnd)
                .collect()
        });
        for hwnd in targets {
            Self::sync_window(hwnd);
        }
    }
}

fn notify_list_changed() {
    let notify_hwnd = STATE.with(|c| c.borrow().notify_hwnd);
    if !notify_hwnd.is_null() && ApiWindowPos::is_window(notify_hwnd) {
        unsafe {
            let _ = PostMessageW(notify_hwnd, WM_PINWIN_LIST_CHANGED, 0, 0);
        }
    }
}

fn register_badge_window_class() {
    unsafe {
        let h_inst = GetModuleHandleW(null());
        let class_name = to_wide(PIN_BADGE_CLASS);
        let mut wc: WNDCLASSEXW = zeroed();
        wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(badge_wnd_proc);
        wc.hInstance = h_inst;
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_HAND);
        wc.lpszClassName = class_name.as_ptr();
        let _ = RegisterClassExW(&wc);
    }
}

fn create_badge_window(target_hwnd: HWND, pos: POINT) -> HWND {
    let dpi = ApiWindowPos::dpi(target_hwnd);
    let w = (24 * dpi as i32 + 48) / 96;
    let h = w;

    unsafe {
        let h_inst = GetModuleHandleW(null());
        let class_name = to_wide(PIN_BADGE_CLASS);
        let title = to_wide("PinBadge");
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_VISIBLE,
            pos.x,
            pos.y,
            w,
            h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            h_inst,
            null(),
        );
        if !hwnd.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, target_hwnd as isize);
            render_badge(hwnd, w);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = UpdateWindow(hwnd);
        }
        hwnd
    }
}

unsafe extern "system" fn badge_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_MOUSEACTIVATE => MA_NOACTIVATE.try_into().unwrap(),
            WM_LBUTTONUP => {
                let target_hwnd = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as HWND;
                if !target_hwnd.is_null() {
                    let notify_hwnd = STATE.with(|c| c.borrow().notify_hwnd);
                    if !notify_hwnd.is_null() {
                        let _ = PostMessageW(
                            notify_hwnd,
                            WM_PINWIN_BADGE_UNPIN,
                            target_hwnd as usize,
                            0,
                        );
                    } else {
                        PinManager::unpin_window(target_hwnd);
                    }
                }
                0
            }
            WM_RBUTTONUP => {
                let target_hwnd = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as HWND;
                if !target_hwnd.is_null() {
                    show_badge_context_menu(hwnd, target_hwnd);
                }
                0
            }
            WM_MOUSEWHEEL => {
                let target_hwnd = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as HWND;
                if !target_hwnd.is_null() {
                    let delta = get_wheel_delta_wparam(wparam);
                    let cur_opacity = STATE.with(|cell| {
                        cell.borrow()
                            .entries
                            .iter()
                            .find(|e| e.target_hwnd == target_hwnd)
                            .map(|e| e.opacity)
                            .unwrap_or(255)
                    });
                    let step: i32 = if delta > 0 { 15 } else { -15 };
                    let next_opacity = (cur_opacity as i32 + step).clamp(50, 255) as u8;
                    PinManager::set_window_opacity(target_hwnd, next_opacity);
                }
                0
            }
            WM_DPICHANGED => {
                let size = (24 * hiword(wparam) as i32 + 48) / 96;
                render_badge(hwnd, size);
                0
            }
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = zeroed();
                let _hdc = BeginPaint(hwnd, &mut ps);
                EndPaint(hwnd, &ps);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

unsafe fn show_badge_context_menu(badge_hwnd: HWND, target_hwnd: HWND) {
    unsafe {
        let (opacity, click_through) = STATE.with(|cell| {
            cell.borrow()
                .entries
                .iter()
                .find(|e| e.target_hwnd == target_hwnd)
                .map(|e| (e.opacity, e.click_through))
                .unwrap_or((255, false))
        });

        let Some(owned_menu) = OwnedMenu::from_owned(CreatePopupMenu()) else {
            return;
        };
        let menu = owned_menu.as_raw();
        if menu.is_null() {
            return;
        }

        let lbl_unpin = to_wide("Unpin Window (Left-Click)");
        let lbl_100 = to_wide("Opacity: 100% (Opaque)");
        let lbl_85 = to_wide("Opacity: 85%");
        let lbl_70 = to_wide("Opacity: 70%");
        let lbl_50 = to_wide("Opacity: 50%");
        let lbl_ghost = to_wide("Click-Through (Ghost Mode)");

        let _ = AppendMenuW(menu, MF_STRING, IDM_BADGE_UNPIN, lbl_unpin.as_ptr());
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, null());
        let _ = AppendMenuW(
            menu,
            MF_STRING | if opacity >= 250 { MF_CHECKED } else { 0 },
            IDM_BADGE_OPACITY_100,
            lbl_100.as_ptr(),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING
                | if (210..250).contains(&opacity) {
                    MF_CHECKED
                } else {
                    0
                },
            IDM_BADGE_OPACITY_85,
            lbl_85.as_ptr(),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING
                | if (165..210).contains(&opacity) {
                    MF_CHECKED
                } else {
                    0
                },
            IDM_BADGE_OPACITY_70,
            lbl_70.as_ptr(),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING | if opacity < 165 { MF_CHECKED } else { 0 },
            IDM_BADGE_OPACITY_50,
            lbl_50.as_ptr(),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, null());
        let _ = AppendMenuW(
            menu,
            MF_STRING | if click_through { MF_CHECKED } else { 0 },
            IDM_BADGE_GHOST_TOGGLE,
            lbl_ghost.as_ptr(),
        );

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(badge_hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
            pt.x,
            pt.y,
            0,
            badge_hwnd,
            null(),
        ) as usize;

        match cmd {
            IDM_BADGE_UNPIN => {
                PinManager::unpin_window(target_hwnd);
            }
            IDM_BADGE_OPACITY_100 => {
                PinManager::set_window_opacity(target_hwnd, 255);
            }
            IDM_BADGE_OPACITY_85 => {
                PinManager::set_window_opacity(target_hwnd, 217);
            }
            IDM_BADGE_OPACITY_70 => {
                PinManager::set_window_opacity(target_hwnd, 178);
            }
            IDM_BADGE_OPACITY_50 => {
                PinManager::set_window_opacity(target_hwnd, 128);
            }
            IDM_BADGE_GHOST_TOGGLE => {
                PinManager::set_window_click_through(target_hwnd, !click_through);
            }
            _ => {}
        }
    }
}

fn install_win_event_hooks() -> Vec<HWINEVENTHOOK> {
    let mut hooks = Vec::new();
    let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
    unsafe {
        // Hook location change, destroy, show/hide, name change (0x8001..0x800C)
        let h1 = SetWinEventHook(
            EVENT_OBJECT_DESTROY,
            EVENT_OBJECT_NAMECHANGE,
            std::ptr::null_mut(),
            Some(win_event_proc),
            0,
            0,
            flags,
        );
        if !h1.is_null() {
            hooks.push(h1);
        }
        // Hook system move/size end and minimize start/end
        let h2 = SetWinEventHook(
            EVENT_SYSTEM_MOVESIZEEND,
            EVENT_SYSTEM_MINIMIZEEND,
            std::ptr::null_mut(),
            Some(win_event_proc),
            0,
            0,
            flags,
        );
        if !h2.is_null() {
            hooks.push(h2);
        }
    }
    hooks
}

unsafe extern "system" fn win_event_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    event: DWORD,
    hwnd: HWND,
    id_object: LONG,
    id_child: LONG,
    _id_event_thread: DWORD,
    _dwms_event_time: DWORD,
) {
    if hwnd.is_null() || id_object != OBJID_WINDOW || id_child != 0 {
        return;
    }
    let notify = STATE.with(|c| c.try_borrow().ok().map(|s| s.notify_hwnd));
    if let Some(notify) = notify
        && !notify.is_null()
    {
        unsafe {
            PostMessageW(
                notify,
                WM_PINWIN_WINDOW_EVENT,
                hwnd as usize,
                event as isize,
            );
        }
    }
}

fn render_badge(hwnd: HWND, size: i32) {
    // DrawIconEx writes the icon's premultiplied alpha into a clear 32-bit DIB.
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let Some(dc) = crate::winapi::resources::OwnedDc::from_owned(CreateCompatibleDC(screen))
        else {
            ReleaseDC(std::ptr::null_mut(), screen);
            return;
        };
        let mut info: BITMAPINFO = zeroed();
        info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = size;
        info.bmiHeader.biHeight = -size;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        let mut bits = std::ptr::null_mut();
        let raw = CreateDIBSection(
            screen,
            &info,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        if let Some(bitmap) = OwnedBitmap::from_owned(raw)
            && !bits.is_null()
        {
            std::ptr::write_bytes(bits.cast::<u8>(), 0, (size * size * 4) as usize);
            let old = SelectObject(dc.as_raw(), bitmap.as_raw());
            let icon = STATE.with(|c| c.borrow().badge_icon);
            DrawIconEx(
                dc.as_raw(),
                0,
                0,
                icon,
                size,
                size,
                0,
                std::ptr::null_mut(),
                DI_NORMAL,
            );
            let dimensions = SIZE { cx: size, cy: size };
            let source = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            UpdateLayeredWindow(
                hwnd,
                screen,
                std::ptr::null(),
                &dimensions,
                dc.as_raw(),
                &source,
                0,
                &blend,
                ULW_ALPHA,
            );
            SelectObject(dc.as_raw(), old);
        }
        ReleaseDC(std::ptr::null_mut(), screen);
    }
}
