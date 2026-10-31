//! Native Win32 GUI (`MainForm`, `TrayAppForm`, `SettingsForm`, `IconFileSelectorControl`).
//!
//! Ported and enhanced from:
//! - `PinWin/MainForm.cs` & `MainForm.Designer.cs`
//! - `PinWin/Controls/TrayAppForm.cs`
//! - `PinWin/Controls/PinnedWindowListControl.cs`
//! - `PinWin/SettingsForm.cs` & `SettingsForm.Designer.cs`
//! - `PinWin/Controls/IconFileSelectorControl.cs`

use std::cell::RefCell;
use std::mem::{size_of, zeroed};
use std::path::Path;
use std::ptr::null;

use crate::keys::{
    KeyCombo, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, VK_BACK, VK_CONTROL, VK_DELETE, VK_ESCAPE,
    VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, is_modifier_only_vk,
};
use crate::overlay::DesktopOverlay;
use crate::pin_manager::{
    PinManager, WM_PINWIN_BADGE_UNPIN, WM_PINWIN_LIST_CHANGED, WM_PINWIN_WINDOW_EVENT,
};
use crate::relative_path::resolve_asset_path;
use crate::settings::{ApplicationSettings, DEFAULT_CAPTURE_ICON_PATH, DEFAULT_TRAY_ICON_PATH};
use crate::winapi::ffi::*;
use crate::winapi::resources::{OwnedFont, OwnedIcon, OwnedMenu};
use crate::winapi::{
    ApiForm, ApiHotKey, ApiResources, ApiWindowPos, EMBEDDED_TARGET_CURSOR_ICO, EMBEDDED_TRAY_ICO,
    from_wide, select_file_from_disk, to_wide,
};

const MAIN_WINDOW_CLASS: &str = "PinWin_MainForm_Class";
const SETTINGS_WINDOW_CLASS: &str = "PinWin_SettingsForm_Class";
const HOTKEY_EDIT_PROP_PROMPT: usize = 1;
const HOTKEY_EDIT_PROP_CURSOR: usize = 2;

const WM_TRAYICON: UINT = WM_APP + 1;
const TIMER_POS_SYNC: usize = 1;

// HotKey registration IDs
const HOTKEY_ID_PROMPT: i32 = 1;
const HOTKEY_ID_UNDER_CURSOR: i32 = 2;

// MainForm Control IDs
const IDC_BTN_SELECT_WINDOW: usize = 1001;
const IDC_BTN_PIN_CURSOR: usize = 1002;
const IDC_BTN_SETTINGS: usize = 1003;
const IDC_LST_PINNED: usize = 1004;
const IDC_LBL_EMPTY_HINT: usize = 1005;
const IDC_BTN_UNPIN_ALL: usize = 1006;
const IDC_BTN_UNPIN_SELECTED: usize = 1007;
const IDC_BTN_OPACITY_100: usize = 1008;
const IDC_BTN_OPACITY_80: usize = 1009;
const IDC_BTN_OPACITY_60: usize = 1010;
const IDC_BTN_GHOST_TOGGLE: usize = 1011;
const IDC_LBL_SHORTCUT_HINT: usize = 1012;

// Tray Context Menu IDs
const IDM_TRAY_OPEN: usize = 2001;
const IDM_TRAY_SELECT_WINDOW: usize = 2002;
const IDM_TRAY_PIN_UNDER_CURSOR: usize = 2003;
const IDM_TRAY_UNPIN_ALL: usize = 2004;
const IDM_TRAY_RESTORE_GHOST: usize = 2005;
const IDM_TRAY_SETTINGS: usize = 2006;
const IDM_TRAY_EXIT: usize = 2007;

// SettingsDialog Control IDs
const IDC_SET_PROMPT_CLEAR: usize = 3001;
const IDC_SET_CURSOR_CLEAR: usize = 3002;
const IDC_SET_TRAY_CUSTOM_CHK: usize = 3003;
const IDC_SET_TRAY_PATH_EDIT: usize = 3004;
const IDC_SET_TRAY_BROWSE_BTN: usize = 3005;
const IDC_SET_TRAY_STATUS_LBL: usize = 3006;
const IDC_SET_TRAY_ICON_PIC: usize = 3007;
const IDC_SET_CAP_CUSTOM_CHK: usize = 3008;
const IDC_SET_CAP_PATH_EDIT: usize = 3009;
const IDC_SET_CAP_BROWSE_BTN: usize = 3010;
const IDC_SET_CAP_STATUS_LBL: usize = 3011;
const IDC_SET_CAP_ICON_PIC: usize = 3012;
const IDC_SET_HIGHLIGHT_CHK: usize = 3013;
const IDC_SET_SAVE_BTN: usize = 3014;
const IDC_SET_CANCEL_BTN: usize = 3015;

#[derive(Default)]
struct MainUiState {
    hwnd: HWND,
    btn_select_window: HWND,
    btn_pin_cursor: HWND,
    btn_settings: HWND,
    lst_pinned: HWND,
    lbl_empty_hint: HWND,
    btn_unpin_all: HWND,
    btn_unpin_selected: HWND,
    btn_opacity_100: HWND,
    btn_opacity_80: HWND,
    btn_opacity_60: HWND,
    btn_ghost_toggle: HWND,
    lbl_shortcut_hint: HWND,
    font_regular: HFONT,
    owned_fonts: Vec<OwnedFont>,
    owned_tray_icon: Option<OwnedIcon>,
    font_bold: HFONT,
    tray_icon: HICON,
    settings: ApplicationSettings,
    balloon_shown: bool,
    pinned_hwnds_in_list: Vec<HWND>,
}

#[derive(Default)]
struct SettingsUiState {
    hwnd: HWND,
    edt_prompt: HWND,
    edt_cursor: HWND,
    orig_edit_proc: isize,
    chk_tray_custom: HWND,
    edt_tray_path: HWND,
    btn_tray_browse: HWND,
    lbl_tray_status: HWND,
    pic_tray_icon: HWND,
    chk_cap_custom: HWND,
    edt_cap_path: HWND,
    btn_cap_browse: HWND,
    lbl_cap_status: HWND,
    pic_cap_icon: HWND,
    chk_highlight: HWND,
    draft: ApplicationSettings,
    saved: bool,
    preview_icons: Vec<OwnedIcon>,
    closed: bool,
}

thread_local! {
    static MAIN_UI: RefCell<MainUiState> = RefCell::new(MainUiState::default());
    static SETTINGS_UI: RefCell<SettingsUiState> = RefCell::new(SettingsUiState::default());
}

/// Map an explicit selection onto the refreshed item order.
pub fn selection_indices<T: PartialEq>(items: &[T], selected: &[T]) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| selected.contains(item).then_some(i))
        .collect()
}

mod main_window;
mod settings;
mod tray;
pub use main_window::run_gui_app;
use main_window::*;
use settings::*;
use tray::*;

unsafe fn scale_controls(hwnd: HWND, old_dpi: u32, new_dpi: u32) {
    unsafe {
        let mut child = GetWindow(hwnd, GW_CHILD);
        while !child.is_null() {
            let mut rect: RECT = zeroed();
            GetWindowRect(child, &mut rect);
            MapWindowPoints(
                std::ptr::null_mut(),
                hwnd,
                (&mut rect as *mut RECT).cast(),
                2,
            );
            let scale = |v: i32| (v as i64 * new_dpi as i64 / old_dpi.max(1) as i64) as i32;
            SetWindowPos(
                child,
                std::ptr::null_mut(),
                scale(rect.left),
                scale(rect.top),
                scale(rect.width()),
                scale(rect.height()),
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            let font = SendMessageW(child, WM_GETFONT, 0, 0) as HFONT;
            let mut description: LOGFONTW = zeroed();
            if !font.is_null()
                && GetObjectW(
                    font,
                    size_of::<LOGFONTW>() as i32,
                    (&mut description as *mut LOGFONTW).cast(),
                ) != 0
            {
                description.lfHeight = scale(description.lfHeight);
                if let Some(owned) = OwnedFont::from_owned(CreateFontIndirectW(&description)) {
                    SendMessageW(child, WM_SETFONT, owned.as_raw() as usize, TRUE as isize);
                    MAIN_UI.with(|c| c.borrow_mut().owned_fonts.push(owned));
                }
            }
            child = GetWindow(child, GW_HWNDNEXT);
        }
        SetPropW(
            hwnd,
            to_wide("PinWin.Dpi").as_ptr(),
            new_dpi as usize as HANDLE,
        );
    }
}

unsafe fn initial_dpi_layout(hwnd: HWND) {
    unsafe {
        let dpi = ApiWindowPos::dpi(hwnd);
        scale_controls(hwnd, 96, dpi);
        let mut rect: RECT = zeroed();
        GetWindowRect(hwnd, &mut rect);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            rect.width() * dpi as i32 / 96,
            rect.height() * dpi as i32 / 96,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

unsafe fn update_dpi_layout(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) {
    unsafe {
        let old = GetPropW(hwnd, to_wide("PinWin.Dpi").as_ptr()) as usize as u32;
        let dpi = hiword(wparam) as u32;
        scale_controls(hwnd, old.max(96), dpi);
        let suggested = &*(lparam as *const RECT);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            suggested.left,
            suggested.top,
            suggested.width(),
            suggested.height(),
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}
