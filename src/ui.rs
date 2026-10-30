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
use crate::pin_manager::{PinManager, WM_PINWIN_BADGE_UNPIN, WM_PINWIN_LIST_CHANGED};
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

/// Run the main PinWin GUI application loop.
pub fn run_gui_app(start_minimized: bool) -> i32 {
    let initial_settings = ApplicationSettings::load_from_file(None);
    MAIN_UI.with(|cell| {
        cell.borrow_mut().settings = initial_settings;
    });

    unsafe {
        let h_inst = GetModuleHandleW(null());
        let class_name = to_wide(MAIN_WINDOW_CLASS);
        let app_icon = OwnedIcon::from_owned(ApiResources::create_icon_from_ico_bytes(
            EMBEDDED_TRAY_ICO,
            true,
            32,
        ));

        let mut wc: WNDCLASSEXW = zeroed();
        wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(main_wnd_proc);
        wc.hInstance = h_inst;
        wc.hIcon = app_icon
            .as_ref()
            .map_or(std::ptr::null_mut(), OwnedIcon::as_raw);
        wc.hIconSm = wc.hIcon;
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        wc.hbrBackground = (COLOR_BTNFACE + 1) as HBRUSH;
        wc.lpszClassName = class_name.as_ptr();
        let _ = RegisterClassExW(&wc);

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let win_w = 500;
        let win_h = 375;
        let pos_x = ((screen_w - win_w) / 2).max(40);
        let pos_y = ((screen_h - win_h) / 2).max(40);

        let title = to_wide("PinWin (Rust Edition)");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN,
            pos_x,
            pos_y,
            win_w,
            win_h,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            h_inst,
            null(),
        );

        if hwnd.is_null() {
            return 1;
        }

        PinManager::init(hwnd);
        apply_current_settings(hwnd);
        let _ = SetTimer(hwnd, TIMER_POS_SYNC, 250, None);

        if start_minimized {
            let _ = ShowWindow(hwnd, SW_HIDE);
        } else {
            let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
            let _ = UpdateWindow(hwnd);
        }

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }

        PinManager::shutdown();
        remove_tray_icon(hwnd);
        MAIN_UI.with(|c| {
            let mut st = c.borrow_mut();
            st.owned_tray_icon = None;
            st.owned_fonts.clear();
        });
        let _ = UnregisterClassW(class_name.as_ptr(), h_inst);
        msg.wParam as i32
    }
}

unsafe fn create_main_controls(hwnd: HWND) {
    unsafe {
        let h_inst = GetModuleHandleW(null());

        let font_name = to_wide("Segoe UI");
        let font_reg = CreateFontW(
            -14,
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            font_name.as_ptr(),
        );
        let font_bold = CreateFontW(
            -15,
            0,
            0,
            0,
            600,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            font_name.as_ptr(),
        );

        let cls_btn = to_wide("BUTTON");
        let cls_lst = to_wide("LISTBOX");
        let cls_lbl = to_wide("STATIC");

        let btn_select = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Select Window").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_DEFPUSHBUTTON as u32),
            14,
            12,
            175,
            36,
            hwnd,
            IDC_BTN_SELECT_WINDOW as HMENU,
            h_inst,
            null(),
        );

        let btn_cursor = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Pin Under Cursor").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            197,
            12,
            140,
            36,
            hwnd,
            IDC_BTN_PIN_CURSOR as HMENU,
            h_inst,
            null(),
        );

        let btn_settings = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Settings").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            345,
            12,
            125,
            36,
            hwnd,
            IDC_BTN_SETTINGS as HMENU,
            h_inst,
            null(),
        );

        let lst_pinned = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            cls_lst.as_ptr(),
            to_wide("").as_ptr(),
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | WS_VSCROLL
                | (LBS_NOTIFY as u32)
                | (LBS_EXTENDEDSEL as u32)
                | (LBS_NOINTEGRALHEIGHT as u32),
            14,
            58,
            456,
            176,
            hwnd,
            IDC_LST_PINNED as HMENU,
            h_inst,
            null(),
        );

        let lbl_empty = CreateWindowExW(
            0,
            cls_lbl.as_ptr(),
            to_wide("A list of pinned windows will be shown here").as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_CENTER,
            24,
            134,
            436,
            24,
            hwnd,
            IDC_LBL_EMPTY_HINT as HMENU,
            h_inst,
            null(),
        );

        let btn_unpin_all = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Unpin All").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            14,
            242,
            100,
            30,
            hwnd,
            IDC_BTN_UNPIN_ALL as HMENU,
            h_inst,
            null(),
        );

        let btn_unpin_sel = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Unpin Selected").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            120,
            242,
            115,
            30,
            hwnd,
            IDC_BTN_UNPIN_SELECTED as HMENU,
            h_inst,
            null(),
        );

        let btn_op_100 = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("100%").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            243,
            242,
            46,
            30,
            hwnd,
            IDC_BTN_OPACITY_100 as HMENU,
            h_inst,
            null(),
        );

        let btn_op_80 = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("80%").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            293,
            242,
            46,
            30,
            hwnd,
            IDC_BTN_OPACITY_80 as HMENU,
            h_inst,
            null(),
        );

        let btn_op_60 = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("60%").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            343,
            242,
            46,
            30,
            hwnd,
            IDC_BTN_OPACITY_60 as HMENU,
            h_inst,
            null(),
        );

        let btn_ghost = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Ghost").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            393,
            242,
            77,
            30,
            hwnd,
            IDC_BTN_GHOST_TOGGLE as HMENU,
            h_inst,
            null(),
        );

        let grp_hint = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | (BS_GROUPBOX as u32),
            14,
            276,
            456,
            48,
            hwnd,
            std::ptr::null_mut(),
            h_inst,
            null(),
        );

        let lbl_hint = CreateWindowExW(
            0,
            cls_lbl.as_ptr(),
            to_wide("Use CTRL + F11 to trigger window selection when minimized to tray").as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            26,
            294,
            432,
            22,
            hwnd,
            IDC_LBL_SHORTCUT_HINT as HMENU,
            h_inst,
            null(),
        );

        for ctrl in [
            btn_cursor,
            btn_settings,
            lst_pinned,
            lbl_empty,
            btn_unpin_all,
            btn_unpin_sel,
            btn_op_100,
            btn_op_80,
            btn_op_60,
            btn_ghost,
            grp_hint,
            lbl_hint,
        ] {
            let _ = SendMessageW(ctrl, WM_SETFONT, font_reg as WPARAM, 1);
        }
        let _ = SendMessageW(btn_select, WM_SETFONT, font_bold as WPARAM, 1);

        MAIN_UI.with(|cell| {
            let mut st = cell.borrow_mut();
            st.hwnd = hwnd;
            st.btn_select_window = btn_select;
            st.btn_pin_cursor = btn_cursor;
            st.btn_settings = btn_settings;
            st.lst_pinned = lst_pinned;
            st.lbl_empty_hint = lbl_empty;
            st.btn_unpin_all = btn_unpin_all;
            st.btn_unpin_selected = btn_unpin_sel;
            st.btn_opacity_100 = btn_op_100;
            st.btn_opacity_80 = btn_op_80;
            st.btn_opacity_60 = btn_op_60;
            st.btn_ghost_toggle = btn_ghost;
            st.lbl_shortcut_hint = lbl_hint;
            st.owned_fonts = [font_reg, font_bold]
                .into_iter()
                .filter_map(|h| OwnedFont::from_owned(h))
                .collect();
            st.font_regular = font_reg;
            st.font_bold = font_bold;
        });

        refresh_pinned_listbox();
    }
}

unsafe fn apply_current_settings(hwnd: HWND) {
    unsafe {
        let settings = MAIN_UI.with(|c| c.borrow().settings.clone());

        ApiHotKey::clear(hwnd, HOTKEY_ID_PROMPT);
        ApiHotKey::clear(hwnd, HOTKEY_ID_UNDER_CURSOR);

        let mut warnings = Vec::new();
        if !settings.shortcut_pin_window_prompt.is_none()
            && !ApiHotKey::set(hwnd, settings.shortcut_pin_window_prompt, HOTKEY_ID_PROMPT)
        {
            warnings.push(format!(
                "Failed to register global shortcut '{}'. Another app may be using it.",
                settings.shortcut_pin_window_prompt
            ));
        }

        if !settings.shortcut_pin_window_under_cursor.is_none()
            && !ApiHotKey::set(
                hwnd,
                settings.shortcut_pin_window_under_cursor,
                HOTKEY_ID_UNDER_CURSOR,
            )
        {
            warnings.push(format!(
                "Failed to register global shortcut '{}'. Another app may be using it.",
                settings.shortcut_pin_window_under_cursor
            ));
        }

        // Update shortcut hint label
        let lbl_hint = MAIN_UI.with(|c| c.borrow().lbl_shortcut_hint);
        if !lbl_hint.is_null() {
            let hint_text = match settings.shortcut_pin_window_prompt.to_shortcut_string() {
                Some(s) => format!(
                    "Use {} to trigger window selection when minimized to tray",
                    s
                ),
                None => "Assign a global shortcut in Settings for quick tray pinning".to_string(),
            };
            let wide = to_wide(&hint_text);
            let _ = SetWindowTextW(lbl_hint, wide.as_ptr());
        }

        // Load and apply tray icon
        let new_icon =
            ApiResources::load_icon_or_embedded(&settings.tray_icon_path, EMBEDDED_TRAY_ICO, 16);
        update_tray_icon(hwnd, new_icon, None);
        MAIN_UI.with(|c| {
            let mut st = c.borrow_mut();
            st.tray_icon = new_icon;
            st.owned_tray_icon = OwnedIcon::from_owned(new_icon);
        });

        if !warnings.is_empty() {
            let msg = warnings.join("\r\n");
            let msg_w = to_wide(&msg);
            let cap_w = to_wide("PinWin Shortcut Warning");
            let _ = MessageBoxW(hwnd, msg_w.as_ptr(), cap_w.as_ptr(), MB_OK | MB_ICONWARNING);
        }
    }
}

unsafe fn update_tray_icon(hwnd: HWND, hicon: HICON, balloon_text: Option<&str>) {
    unsafe {
        let mut nid: NOTIFYICONDATAW = zeroed();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = hicon;

        let pinned_count = PinManager::count();
        let tip = if pinned_count == 0 {
            "PinWin - Always on Top Window Manager".to_string()
        } else {
            format!("PinWin - {} window(s) pinned", pinned_count)
        };
        let tip_wide = to_wide(&tip);
        for (dst, &src) in nid.szTip.iter_mut().zip(tip_wide.iter()) {
            *dst = src;
        }

        if let Some(info) = balloon_text {
            nid.uFlags |= NIF_INFO;
            nid.dwInfoFlags = NIIF_INFO;
            nid.Anonymous.uTimeout = 1500;
            let title_w = to_wide("PinWin");
            for (dst, &src) in nid.szInfoTitle.iter_mut().zip(title_w.iter()) {
                *dst = src;
            }
            let info_w = to_wide(info);
            for (dst, &src) in nid.szInfo.iter_mut().zip(info_w.iter()) {
                *dst = src;
            }
        }

        if Shell_NotifyIconW(NIM_MODIFY, &nid) == FALSE {
            let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        }
    }
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    unsafe {
        let mut nid: NOTIFYICONDATAW = zeroed();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

unsafe fn refresh_pinned_listbox() {
    unsafe {
        let entries = PinManager::entries();
        let (
            hwnd,
            lst_hwnd,
            lbl_empty,
            btn_all,
            btn_sel,
            btn_100,
            btn_80,
            btn_60,
            btn_ghost,
            tray_icon,
        ) = MAIN_UI.with(|cell| {
            let st = cell.borrow();
            (
                st.hwnd,
                st.lst_pinned,
                st.lbl_empty_hint,
                st.btn_unpin_all,
                st.btn_unpin_selected,
                st.btn_opacity_100,
                st.btn_opacity_80,
                st.btn_opacity_60,
                st.btn_ghost_toggle,
                st.tray_icon,
            )
        });

        if lst_hwnd.is_null() {
            return;
        }

        let previously_selected = get_selected_target_hwnds();
        let _ = SendMessageW(lst_hwnd, LB_RESETCONTENT, 0, 0);
        let mut hwnds = Vec::with_capacity(entries.len());

        for entry in &entries {
            let label = to_wide(&entry.display_label());
            let idx = SendMessageW(lst_hwnd, LB_ADDSTRING, 0, label.as_ptr() as LPARAM);
            if idx >= 0 {
                let _ = SendMessageW(
                    lst_hwnd,
                    LB_SETITEMDATA,
                    idx as WPARAM,
                    entry.target_hwnd as isize,
                );
            }
            hwnds.push(entry.target_hwnd);
        }

        // Restore only the rows the user had selected before the refresh.
        for (idx, &target_hwnd) in hwnds.iter().enumerate() {
            if previously_selected.contains(&target_hwnd) {
                let _ = SendMessageW(lst_hwnd, LB_SETSEL, TRUE as WPARAM, idx as LPARAM);
            }
        }

        MAIN_UI.with(|c| c.borrow_mut().pinned_hwnds_in_list = hwnds);

        let has_items = !entries.is_empty();
        let _ = ShowWindow(lbl_empty, if has_items { SW_HIDE } else { SW_SHOW });
        let _ = EnableWindow(btn_all, if has_items { TRUE } else { FALSE });

        update_selection_action_buttons(
            lst_hwnd, btn_sel, btn_100, btn_80, btn_60, btn_ghost, has_items,
        );

        if !hwnd.is_null() && !tray_icon.is_null() {
            update_tray_icon(hwnd, tray_icon, None);
        }
    }
}

unsafe fn update_selection_action_buttons(
    lst_hwnd: HWND,
    btn_sel: HWND,
    btn_100: HWND,
    btn_80: HWND,
    btn_60: HWND,
    btn_ghost: HWND,
    has_items: bool,
) {
    unsafe {
        let sel_count = if has_items {
            SendMessageW(lst_hwnd, LB_GETSELCOUNT, 0, 0)
        } else {
            0
        };
        let enable_sel = if sel_count > 0 { TRUE } else { FALSE };
        for btn in [btn_sel, btn_100, btn_80, btn_60, btn_ghost] {
            let _ = EnableWindow(btn, enable_sel);
        }
    }
}

unsafe fn get_selected_target_hwnds() -> Vec<HWND> {
    unsafe {
        let (lst_hwnd, hwnds) = MAIN_UI.with(|cell| {
            let st = cell.borrow();
            (st.lst_pinned, st.pinned_hwnds_in_list.clone())
        });
        if lst_hwnd.is_null() || hwnds.is_empty() {
            return Vec::new();
        }

        let mut selected = Vec::new();
        for (idx, &target_hwnd) in hwnds.iter().enumerate() {
            let is_sel = SendMessageW(lst_hwnd, LB_GETSEL, idx, 0);
            if is_sel > 0 {
                selected.push(target_hwnd);
            }
        }
        selected
    }
}

unsafe fn trigger_select_window_overlay(hwnd: HWND) {
    unsafe {
        let was_visible = ApiWindowPos::is_visible(hwnd);
        if was_visible {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }

        let (capture_icon, highlight, default_opacity) = MAIN_UI.with(|cell| {
            let st = cell.borrow();
            (
                st.settings.capture_icon_path.clone(),
                st.settings.highlight_target_on_hover,
                st.settings.default_opacity,
            )
        });

        if let Some(point) = DesktopOverlay::select_point(&capture_icon, highlight) {
            let target_hwnd = ApiForm::select(point);
            if !target_hwnd.is_null()
                && target_hwnd != hwnd
                && let Err(err) = PinManager::pin_window(target_hwnd, default_opacity, false)
            {
                let msg_w = to_wide(err);
                let cap_w = to_wide("PinWin");
                let _ = MessageBoxW(hwnd, msg_w.as_ptr(), cap_w.as_ptr(), MB_OK | MB_ICONWARNING);
            }
        }

        if was_visible {
            let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

unsafe fn trigger_pin_window_under_cursor(hwnd: HWND) {
    unsafe {
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt) != FALSE {
            let target_hwnd = ApiForm::select(pt);
            if !target_hwnd.is_null() && target_hwnd != hwnd {
                let default_opacity = MAIN_UI.with(|c| c.borrow().settings.default_opacity);
                // Toggle if already pinned when using Pin Under Cursor shortcut
                let _ = PinManager::pin_window(target_hwnd, default_opacity, true);
            }
        }
    }
}

unsafe fn show_tray_context_menu(hwnd: HWND) {
    unsafe {
        let Some(owned_menu) = OwnedMenu::from_owned(CreatePopupMenu()) else {
            return;
        };
        let menu = owned_menu.as_raw();
        if menu.is_null() {
            return;
        }

        let (prompt_sc, cursor_sc) = MAIN_UI.with(|cell| {
            let st = cell.borrow();
            (
                st.settings.shortcut_pin_window_prompt.to_shortcut_string(),
                st.settings
                    .shortcut_pin_window_under_cursor
                    .to_shortcut_string(),
            )
        });

        let select_label = match prompt_sc {
            Some(s) => format!("Select Window to Pin\t{}", s),
            None => "Select Window to Pin".to_string(),
        };
        let cursor_label = match cursor_sc {
            Some(s) => format!("Pin Window Under Cursor\t{}", s),
            None => "Pin Window Under Cursor".to_string(),
        };

        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_TRAY_OPEN,
            to_wide("Open PinWin").as_ptr(),
        );
        let _ = SetMenuDefaultItem(menu, IDM_TRAY_OPEN as u32, FALSE as u32);
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, null());
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_TRAY_SELECT_WINDOW,
            to_wide(&select_label).as_ptr(),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_TRAY_PIN_UNDER_CURSOR,
            to_wide(&cursor_label).as_ptr(),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_TRAY_UNPIN_ALL,
            to_wide("Unpin All Windows").as_ptr(),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_TRAY_RESTORE_GHOST,
            to_wide("Restore Click-Through Windows").as_ptr(),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, null());
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_TRAY_SETTINGS,
            to_wide("Settings...").as_ptr(),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, null());
        let _ = AppendMenuW(menu, MF_STRING, IDM_TRAY_EXIT, to_wide("Exit").as_ptr());

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
            pt.x,
            pt.y,
            0,
            hwnd,
            null(),
        ) as usize;

        handle_menu_or_button_command(hwnd, cmd);
    }
}

unsafe fn handle_menu_or_button_command(hwnd: HWND, cmd_id: usize) {
    unsafe {
        match cmd_id {
            IDC_BTN_SELECT_WINDOW | IDM_TRAY_SELECT_WINDOW => {
                trigger_select_window_overlay(hwnd);
            }
            IDC_BTN_PIN_CURSOR | IDM_TRAY_PIN_UNDER_CURSOR => {
                trigger_pin_window_under_cursor(hwnd);
            }
            IDC_BTN_SETTINGS | IDM_TRAY_SETTINGS => {
                open_settings_dialog(hwnd);
            }
            IDC_BTN_UNPIN_ALL | IDM_TRAY_UNPIN_ALL => {
                PinManager::unpin_all();
            }
            IDM_TRAY_RESTORE_GHOST => {
                PinManager::restore_all_click_through();
            }
            IDC_BTN_UNPIN_SELECTED => {
                for target in get_selected_target_hwnds() {
                    PinManager::unpin_window(target);
                }
            }
            IDC_BTN_OPACITY_100 => {
                for target in get_selected_target_hwnds() {
                    PinManager::set_window_opacity(target, 255);
                }
            }
            IDC_BTN_OPACITY_80 => {
                for target in get_selected_target_hwnds() {
                    PinManager::set_window_opacity(target, 204);
                }
            }
            IDC_BTN_OPACITY_60 => {
                for target in get_selected_target_hwnds() {
                    PinManager::set_window_opacity(target, 153);
                }
            }
            IDC_BTN_GHOST_TOGGLE => {
                let entries = PinManager::entries();
                for target in get_selected_target_hwnds() {
                    let cur_ghost = entries
                        .iter()
                        .find(|e| e.target_hwnd == target)
                        .map(|e| e.click_through)
                        .unwrap_or(false);
                    PinManager::set_window_click_through(target, !cur_ghost);
                }
            }
            IDM_TRAY_OPEN => {
                let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
                let _ = SetForegroundWindow(hwnd);
            }
            IDM_TRAY_EXIT => {
                let _ = DestroyWindow(hwnd);
            }
            _ => {}
        }
    }
}

unsafe extern "system" fn main_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_CREATE => {
                create_main_controls(hwnd);
                0
            }
            WM_SIZE => {
                if wparam == SIZE_MINIMIZED as usize {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                    let (already_shown, tray_icon) = MAIN_UI.with(|cell| {
                        let mut st = cell.borrow_mut();
                        let was = st.balloon_shown;
                        st.balloon_shown = true;
                        (was, st.tray_icon)
                    });
                    if !already_shown && !tray_icon.is_null() {
                        update_tray_icon(hwnd, tray_icon, Some("Running minimized to tray"));
                    }
                    return 0;
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_COMMAND => {
                let id = u32::from(loword(wparam)) as usize;
                let code = u32::from(hiword(wparam));
                if id == IDC_LST_PINNED && (code == LBN_SELCHANGE || code == LBN_DBLCLK) {
                    MAIN_UI.with(|cell| {
                        let st = cell.borrow();
                        let has_items = !st.pinned_hwnds_in_list.is_empty();
                        update_selection_action_buttons(
                            st.lst_pinned,
                            st.btn_unpin_selected,
                            st.btn_opacity_100,
                            st.btn_opacity_80,
                            st.btn_opacity_60,
                            st.btn_ghost_toggle,
                            has_items,
                        );
                    });
                    return 0;
                }
                if code == BN_CLICKED {
                    handle_menu_or_button_command(hwnd, id);
                    return 0;
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_HOTKEY => {
                match wparam as i32 {
                    HOTKEY_ID_PROMPT => trigger_select_window_overlay(hwnd),
                    HOTKEY_ID_UNDER_CURSOR => trigger_pin_window_under_cursor(hwnd),
                    _ => {}
                }
                0
            }
            WM_TIMER => {
                if wparam == TIMER_POS_SYNC {
                    PinManager::sync_all();
                }
                0
            }
            WM_PINWIN_LIST_CHANGED => {
                refresh_pinned_listbox();
                0
            }
            WM_PINWIN_BADGE_UNPIN => {
                let target_hwnd = wparam as HWND;
                PinManager::unpin_window(target_hwnd);
                0
            }
            WM_TRAYICON => {
                let mouse_msg = u32::from(loword(lparam as usize));
                match mouse_msg {
                    WM_LBUTTONUP => {
                        // Left-clicking tray icon triggers window selection (matching MainForm.TrayIcon_Click)
                        trigger_select_window_overlay(hwnd);
                    }
                    WM_LBUTTONDBLCLK => {
                        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
                        let _ = SetForegroundWindow(hwnd);
                    }
                    WM_RBUTTONUP => {
                        show_tray_context_menu(hwnd);
                    }
                    _ => {}
                }
                0
            }
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                let _ = KillTimer(hwnd, TIMER_POS_SYNC);
                ApiHotKey::clear(hwnd, HOTKEY_ID_PROMPT);
                ApiHotKey::clear(hwnd, HOTKEY_ID_UNDER_CURSOR);
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

// ============================================================================
// Settings Dialog (`SettingsForm.cs` + `IconFileSelectorControl.cs`)
// ============================================================================

unsafe fn open_settings_dialog(owner_hwnd: HWND) {
    unsafe {
        let current = MAIN_UI.with(|c| c.borrow().settings.clone());
        SETTINGS_UI.with(|cell| {
            let mut st = cell.borrow_mut();
            st.draft = current;
            st.saved = false;
            st.closed = false;
        });

        let h_inst = GetModuleHandleW(null());
        let class_name = to_wide(SETTINGS_WINDOW_CLASS);
        let mut wc: WNDCLASSEXW = zeroed();
        wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(settings_wnd_proc);
        wc.hInstance = h_inst;
        wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        wc.hbrBackground = (COLOR_BTNFACE + 1) as HBRUSH;
        wc.lpszClassName = class_name.as_ptr();
        let _ = RegisterClassExW(&wc);

        let owner_rc = ApiWindowPos::get(owner_hwnd);
        let dlg_w = 500;
        let dlg_h = 400;
        let pos_x = (owner_rc.left + (owner_rc.width() - dlg_w) / 2).max(40);
        let pos_y = (owner_rc.top + (owner_rc.height() - dlg_h) / 2).max(40);

        let _ = EnableWindow(owner_hwnd, FALSE);

        let dlg_hwnd = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            class_name.as_ptr(),
            to_wide("PinWin Settings").as_ptr(),
            WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_VISIBLE | WS_CLIPCHILDREN,
            pos_x,
            pos_y,
            dlg_w,
            dlg_h,
            owner_hwnd,
            std::ptr::null_mut(),
            h_inst,
            null(),
        );

        if dlg_hwnd.is_null() {
            let _ = EnableWindow(owner_hwnd, TRUE);
            return;
        }

        let mut msg: MSG = zeroed();
        while !SETTINGS_UI.with(|c| c.borrow().closed) {
            let res = GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0);
            if res <= 0 {
                if res == 0 {
                    PostQuitMessage(msg.wParam as i32);
                }
                break;
            }
            let _ = TranslateMessage(&msg);
            let _ = DispatchMessageW(&msg);
        }

        let _ = EnableWindow(owner_hwnd, TRUE);
        let _ = SetForegroundWindow(owner_hwnd);
        if ApiWindowPos::is_window(dlg_hwnd) {
            let _ = DestroyWindow(dlg_hwnd);
        }

        SETTINGS_UI.with(|c| c.borrow_mut().preview_icons.clear());
        let (saved, new_settings) = SETTINGS_UI.with(|cell| {
            let st = cell.borrow();
            (st.saved, st.draft.clone())
        });

        if saved {
            MAIN_UI.with(|c| c.borrow_mut().settings = new_settings.clone());
            apply_current_settings(owner_hwnd);
            if !new_settings.save_to_file(None) {
                let msg_w = to_wide("Failed to save settings to %APPDATA%\\PinWin\\settings.json.");
                let cap_w = to_wide("PinWin Settings Error");
                let _ = MessageBoxW(
                    owner_hwnd,
                    msg_w.as_ptr(),
                    cap_w.as_ptr(),
                    MB_OK | MB_ICONERROR,
                );
            }
        }
    }
}

unsafe fn create_settings_controls(hwnd: HWND) {
    unsafe {
        let h_inst = GetModuleHandleW(null());
        let font = MAIN_UI.with(|c| c.borrow().font_regular);

        let cls_btn = to_wide("BUTTON");
        let cls_lbl = to_wide("STATIC");
        let cls_edt = to_wide("EDIT");

        // --- Group 1: Global Keyboard Shortcuts ---
        let grp_keys = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Global Keyboard Shortcuts (Click box and press keys)").as_ptr(),
            WS_CHILD | WS_VISIBLE | (BS_GROUPBOX as u32),
            14,
            10,
            456,
            96,
            hwnd,
            std::ptr::null_mut(),
            h_inst,
            null(),
        );

        let lbl_prompt = CreateWindowExW(
            0,
            cls_lbl.as_ptr(),
            to_wide("Select & Pin Window:").as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            26,
            36,
            155,
            22,
            hwnd,
            std::ptr::null_mut(),
            h_inst,
            null(),
        );

        let edt_prompt = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            cls_edt.as_ptr(),
            to_wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (ES_AUTOHSCROLL as u32) | (ES_READONLY as u32),
            185,
            33,
            195,
            24,
            hwnd,
            std::ptr::null_mut(),
            h_inst,
            null(),
        );

        let btn_prompt_clr = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Clear").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            388,
            32,
            68,
            25,
            hwnd,
            IDC_SET_PROMPT_CLEAR as HMENU,
            h_inst,
            null(),
        );

        let lbl_cursor = CreateWindowExW(
            0,
            cls_lbl.as_ptr(),
            to_wide("Pin Under Cursor:").as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            26,
            68,
            155,
            22,
            hwnd,
            std::ptr::null_mut(),
            h_inst,
            null(),
        );

        let edt_cursor = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            cls_edt.as_ptr(),
            to_wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (ES_AUTOHSCROLL as u32) | (ES_READONLY as u32),
            185,
            65,
            195,
            24,
            hwnd,
            std::ptr::null_mut(),
            h_inst,
            null(),
        );

        let btn_cursor_clr = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Clear").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            388,
            64,
            68,
            25,
            hwnd,
            IDC_SET_CURSOR_CLEAR as HMENU,
            h_inst,
            null(),
        );

        // Subclass hotkey edit controls to capture key combinations live
        SetWindowLongPtrW(edt_prompt, GWLP_USERDATA, HOTKEY_EDIT_PROP_PROMPT as isize);
        SetWindowLongPtrW(edt_cursor, GWLP_USERDATA, HOTKEY_EDIT_PROP_CURSOR as isize);
        let orig_proc = SetWindowLongPtrW(
            edt_prompt,
            GWL_WNDPROC,
            hotkey_edit_subclass_proc as *const () as isize,
        );
        let _ = SetWindowLongPtrW(
            edt_cursor,
            GWL_WNDPROC,
            hotkey_edit_subclass_proc as *const () as isize,
        );

        // --- Group 2: Tray Icon Selector ---
        let grp_tray = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Tray Icon").as_ptr(),
            WS_CHILD | WS_VISIBLE | (BS_GROUPBOX as u32),
            14,
            114,
            456,
            82,
            hwnd,
            std::ptr::null_mut(),
            h_inst,
            null(),
        );

        let chk_tray = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Use Custom").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_AUTOCHECKBOX as u32),
            26,
            138,
            96,
            22,
            hwnd,
            IDC_SET_TRAY_CUSTOM_CHK as HMENU,
            h_inst,
            null(),
        );

        let edt_tray = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            cls_edt.as_ptr(),
            to_wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (ES_AUTOHSCROLL as u32),
            126,
            137,
            205,
            24,
            hwnd,
            IDC_SET_TRAY_PATH_EDIT as HMENU,
            h_inst,
            null(),
        );

        let btn_tray_brw = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Browse...").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            337,
            136,
            75,
            25,
            hwnd,
            IDC_SET_TRAY_BROWSE_BTN as HMENU,
            h_inst,
            null(),
        );

        let pic_tray = CreateWindowExW(
            0,
            cls_lbl.as_ptr(),
            to_wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_ICON,
            422,
            134,
            32,
            32,
            hwnd,
            IDC_SET_TRAY_ICON_PIC as HMENU,
            h_inst,
            null(),
        );

        let lbl_tray_st = CreateWindowExW(
            0,
            cls_lbl.as_ptr(),
            to_wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            126,
            166,
            290,
            20,
            hwnd,
            IDC_SET_TRAY_STATUS_LBL as HMENU,
            h_inst,
            null(),
        );

        // --- Group 3: Capture Cursor Icon Selector ---
        let grp_cap = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Window Capture Cursor").as_ptr(),
            WS_CHILD | WS_VISIBLE | (BS_GROUPBOX as u32),
            14,
            202,
            456,
            82,
            hwnd,
            std::ptr::null_mut(),
            h_inst,
            null(),
        );

        let chk_cap = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Use Custom").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_AUTOCHECKBOX as u32),
            26,
            226,
            96,
            22,
            hwnd,
            IDC_SET_CAP_CUSTOM_CHK as HMENU,
            h_inst,
            null(),
        );

        let edt_cap = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            cls_edt.as_ptr(),
            to_wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (ES_AUTOHSCROLL as u32),
            126,
            225,
            205,
            24,
            hwnd,
            IDC_SET_CAP_PATH_EDIT as HMENU,
            h_inst,
            null(),
        );

        let btn_cap_brw = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Browse...").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            337,
            224,
            75,
            25,
            hwnd,
            IDC_SET_CAP_BROWSE_BTN as HMENU,
            h_inst,
            null(),
        );

        let pic_cap = CreateWindowExW(
            0,
            cls_lbl.as_ptr(),
            to_wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_ICON,
            422,
            222,
            32,
            32,
            hwnd,
            IDC_SET_CAP_ICON_PIC as HMENU,
            h_inst,
            null(),
        );

        let lbl_cap_st = CreateWindowExW(
            0,
            cls_lbl.as_ptr(),
            to_wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            126,
            254,
            290,
            20,
            hwnd,
            IDC_SET_CAP_STATUS_LBL as HMENU,
            h_inst,
            null(),
        );

        // --- Highlight checkbox & Save / Don't Save buttons ---
        let chk_highlight = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Highlight target window border during selection").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_AUTOCHECKBOX as u32),
            18,
            294,
            350,
            22,
            hwnd,
            IDC_SET_HIGHLIGHT_CHK as HMENU,
            h_inst,
            null(),
        );

        let btn_save = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Save").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_DEFPUSHBUTTON as u32),
            262,
            324,
            98,
            30,
            hwnd,
            IDC_SET_SAVE_BTN as HMENU,
            h_inst,
            null(),
        );

        let btn_cancel = CreateWindowExW(
            0,
            cls_btn.as_ptr(),
            to_wide("Don't Save").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_PUSHBUTTON as u32),
            370,
            324,
            100,
            30,
            hwnd,
            IDC_SET_CANCEL_BTN as HMENU,
            h_inst,
            null(),
        );

        for ctrl in [
            grp_keys,
            lbl_prompt,
            edt_prompt,
            btn_prompt_clr,
            lbl_cursor,
            edt_cursor,
            btn_cursor_clr,
            grp_tray,
            chk_tray,
            edt_tray,
            btn_tray_brw,
            lbl_tray_st,
            grp_cap,
            chk_cap,
            edt_cap,
            btn_cap_brw,
            lbl_cap_st,
            chk_highlight,
            btn_save,
            btn_cancel,
        ] {
            let _ = SendMessageW(ctrl, WM_SETFONT, font as WPARAM, 1);
        }

        SETTINGS_UI.with(|cell| {
            let mut st = cell.borrow_mut();
            st.hwnd = hwnd;
            st.edt_prompt = edt_prompt;
            st.edt_cursor = edt_cursor;
            st.orig_edit_proc = orig_proc;
            st.chk_tray_custom = chk_tray;
            st.edt_tray_path = edt_tray;
            st.btn_tray_browse = btn_tray_brw;
            st.lbl_tray_status = lbl_tray_st;
            st.pic_tray_icon = pic_tray;
            st.chk_cap_custom = chk_cap;
            st.edt_cap_path = edt_cap;
            st.btn_cap_browse = btn_cap_brw;
            st.lbl_cap_status = lbl_cap_st;
            st.pic_cap_icon = pic_cap;
            st.chk_highlight = chk_highlight;
        });

        populate_settings_controls_from_draft();
    }
}

unsafe fn populate_settings_controls_from_draft() {
    unsafe {
        let st_snapshot = SETTINGS_UI.with(|cell| {
            let st = cell.borrow();
            (
                st.edt_prompt,
                st.edt_cursor,
                st.chk_tray_custom,
                st.edt_tray_path,
                st.chk_cap_custom,
                st.edt_cap_path,
                st.chk_highlight,
                st.draft.clone(),
            )
        });

        let (edt_prompt, edt_cursor, chk_tray, edt_tray, chk_cap, edt_cap, chk_highlight, draft) =
            st_snapshot;

        let prompt_str = draft
            .shortcut_pin_window_prompt
            .to_shortcut_string()
            .unwrap_or_default();
        let cursor_str = draft
            .shortcut_pin_window_under_cursor
            .to_shortcut_string()
            .unwrap_or_default();

        let _ = SetWindowTextW(edt_prompt, to_wide(&prompt_str).as_ptr());
        let _ = SetWindowTextW(edt_cursor, to_wide(&cursor_str).as_ptr());

        let tray_custom = !draft.tray_icon_path.trim().is_empty();
        let _ = SendMessageW(
            chk_tray,
            BM_SETCHECK,
            if tray_custom {
                BST_CHECKED.try_into().unwrap()
            } else {
                BST_UNCHECKED.try_into().unwrap()
            },
            0,
        );
        let tray_display = if tray_custom {
            draft.tray_icon_path.as_str()
        } else {
            DEFAULT_TRAY_ICON_PATH
        };
        let _ = SetWindowTextW(edt_tray, to_wide(tray_display).as_ptr());

        let cap_custom = !draft.capture_icon_path.trim().is_empty();
        let _ = SendMessageW(
            chk_cap,
            BM_SETCHECK,
            if cap_custom {
                BST_CHECKED.try_into().unwrap()
            } else {
                BST_UNCHECKED.try_into().unwrap()
            },
            0,
        );
        let cap_display = if cap_custom {
            draft.capture_icon_path.as_str()
        } else {
            DEFAULT_CAPTURE_ICON_PATH
        };
        let _ = SetWindowTextW(edt_cap, to_wide(cap_display).as_ptr());

        let _ = SendMessageW(
            chk_highlight,
            BM_SETCHECK,
            if draft.highlight_target_on_hover {
                BST_CHECKED.try_into().unwrap()
            } else {
                BST_UNCHECKED.try_into().unwrap()
            },
            0,
        );

        refresh_icon_selector_state(true);
        refresh_icon_selector_state(false);
    }
}

unsafe fn refresh_icon_selector_state(is_tray: bool) {
    unsafe {
        let (chk_hwnd, edt_hwnd, btn_hwnd, lbl_hwnd, pic_hwnd) = SETTINGS_UI.with(|cell| {
            let st = cell.borrow();
            if is_tray {
                (
                    st.chk_tray_custom,
                    st.edt_tray_path,
                    st.btn_tray_browse,
                    st.lbl_tray_status,
                    st.pic_tray_icon,
                )
            } else {
                (
                    st.chk_cap_custom,
                    st.edt_cap_path,
                    st.btn_cap_browse,
                    st.lbl_cap_status,
                    st.pic_cap_icon,
                )
            }
        });

        let checked = SendMessageW(chk_hwnd, BM_GETCHECK, 0, 0) as u32 == BST_CHECKED;
        let _ = EnableWindow(edt_hwnd, if checked { TRUE } else { FALSE });
        let _ = EnableWindow(btn_hwnd, if checked { TRUE } else { FALSE });

        let embedded = if is_tray {
            EMBEDDED_TRAY_ICO
        } else {
            EMBEDDED_TARGET_CURSOR_ICO
        };

        if !checked {
            let default_path = if is_tray {
                DEFAULT_TRAY_ICON_PATH
            } else {
                DEFAULT_CAPTURE_ICON_PATH
            };
            let _ = SetWindowTextW(edt_hwnd, to_wide(default_path).as_ptr());
            let _ = SetWindowTextW(lbl_hwnd, to_wide("Using built-in icon").as_ptr());
            let hicon = ApiResources::create_icon_from_ico_bytes(embedded, true, 32);
            replace_preview_icon(pic_hwnd, hicon);
            SETTINGS_UI.with(|cell| {
                let mut st = cell.borrow_mut();
                if is_tray {
                    st.draft.tray_icon_path.clear();
                } else {
                    st.draft.capture_icon_path.clear();
                }
            });
            return;
        }

        let path_str = read_window_text(edt_hwnd);
        let trimmed = path_str.trim();
        if trimmed.is_empty() {
            let _ = SetWindowTextW(lbl_hwnd, to_wide("Empty file path").as_ptr());
            let hicon = ApiResources::create_icon_from_ico_bytes(embedded, true, 32);
            replace_preview_icon(pic_hwnd, hicon);
            return;
        }

        let resolved =
            resolve_asset_path(trimmed).unwrap_or_else(|| Path::new(trimmed).to_path_buf());
        if !resolved.is_file() {
            let _ = SetWindowTextW(lbl_hwnd, to_wide("File does not exist").as_ptr());
            return;
        }

        let hicon = ApiResources::load_icon_or_embedded(trimmed, embedded, 32);
        if !hicon.is_null() {
            let _ = SetWindowTextW(lbl_hwnd, to_wide("Valid custom icon").as_ptr());
            replace_preview_icon(pic_hwnd, hicon);
            SETTINGS_UI.with(|cell| {
                let mut st = cell.borrow_mut();
                if is_tray {
                    st.draft.tray_icon_path = trimmed.to_string();
                } else {
                    st.draft.capture_icon_path = trimmed.to_string();
                }
            });
        } else {
            let _ = SetWindowTextW(lbl_hwnd, to_wide("Invalid icon file format").as_ptr());
        }
    }
}

unsafe fn replace_preview_icon(hwnd: HWND, raw: HICON) {
    unsafe {
        let previous = SendMessageW(hwnd, STM_SETICON, raw as WPARAM, 0) as HICON;
        SETTINGS_UI.with(|c| {
            let mut st = c.borrow_mut();
            st.preview_icons.retain(|icon| icon.as_raw() != previous);
            if let Some(icon) = OwnedIcon::from_owned(raw) {
                st.preview_icons.push(icon);
            }
        });
    }
}

unsafe fn read_window_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len + 1) as usize];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
        if n > 0 {
            from_wide(&buf[..n as usize])
        } else {
            String::new()
        }
    }
}

unsafe extern "system" fn hotkey_edit_subclass_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                let vk = wparam as u32;
                let prop = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as usize;

                if vk == VK_BACK || vk == VK_DELETE || vk == VK_ESCAPE {
                    let _ = SetWindowTextW(hwnd, to_wide("").as_ptr());
                    SETTINGS_UI.with(|cell| {
                        let mut st = cell.borrow_mut();
                        if prop == HOTKEY_EDIT_PROP_PROMPT {
                            st.draft.shortcut_pin_window_prompt = KeyCombo::NONE;
                        } else if prop == HOTKEY_EDIT_PROP_CURSOR {
                            st.draft.shortcut_pin_window_under_cursor = KeyCombo::NONE;
                        }
                    });
                    return 0;
                }

                if !is_modifier_only_vk(vk) {
                    let mut mods = 0u32;
                    if (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0 {
                        mods |= MOD_SHIFT;
                    }
                    if (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0 {
                        mods |= MOD_CONTROL;
                    }
                    if (GetAsyncKeyState(VK_MENU as i32) as u16 & 0x8000) != 0 {
                        mods |= MOD_ALT;
                    }
                    if (GetAsyncKeyState(VK_LWIN as i32) as u16 & 0x8000) != 0
                        || (GetAsyncKeyState(VK_RWIN as i32) as u16 & 0x8000) != 0
                    {
                        mods |= MOD_WIN;
                    }

                    let combo = KeyCombo::new(mods, vk);
                    if let Some(text) = combo.to_shortcut_string() {
                        let _ = SetWindowTextW(hwnd, to_wide(&text).as_ptr());
                        SETTINGS_UI.with(|cell| {
                            let mut st = cell.borrow_mut();
                            if prop == HOTKEY_EDIT_PROP_PROMPT {
                                st.draft.shortcut_pin_window_prompt = combo;
                            } else if prop == HOTKEY_EDIT_PROP_CURSOR {
                                st.draft.shortcut_pin_window_under_cursor = combo;
                            }
                        });
                    }
                }
                0
            }
            WM_CHAR => 0,
            _ => {
                let orig = SETTINGS_UI.with(|c| c.borrow().orig_edit_proc);
                if orig != 0 {
                    let f: unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT =
                        std::mem::transmute(orig);
                    f(hwnd, msg, wparam, lparam)
                } else {
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
            }
        }
    }
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_CREATE => {
                create_settings_controls(hwnd);
                0
            }
            WM_COMMAND => {
                let id = u32::from(loword(wparam)) as usize;
                let code = u32::from(hiword(wparam));

                match id {
                    IDC_SET_PROMPT_CLEAR if code == BN_CLICKED => {
                        let edt = SETTINGS_UI.with(|cell| {
                            let mut st = cell.borrow_mut();
                            st.draft.shortcut_pin_window_prompt = KeyCombo::NONE;
                            st.edt_prompt
                        });
                        let _ = SetWindowTextW(edt, to_wide("").as_ptr());
                        0
                    }
                    IDC_SET_CURSOR_CLEAR if code == BN_CLICKED => {
                        let edt = SETTINGS_UI.with(|cell| {
                            let mut st = cell.borrow_mut();
                            st.draft.shortcut_pin_window_under_cursor = KeyCombo::NONE;
                            st.edt_cursor
                        });
                        let _ = SetWindowTextW(edt, to_wide("").as_ptr());
                        0
                    }
                    IDC_SET_TRAY_CUSTOM_CHK if code == BN_CLICKED => {
                        refresh_icon_selector_state(true);
                        0
                    }
                    IDC_SET_CAP_CUSTOM_CHK if code == BN_CLICKED => {
                        refresh_icon_selector_state(false);
                        0
                    }
                    IDC_SET_TRAY_PATH_EDIT if code == EN_CHANGE => {
                        refresh_icon_selector_state(true);
                        0
                    }
                    IDC_SET_CAP_PATH_EDIT if code == EN_CHANGE => {
                        refresh_icon_selector_state(false);
                        0
                    }
                    IDC_SET_TRAY_BROWSE_BTN if code == BN_CLICKED => {
                        if let Some(chosen) = select_file_from_disk(hwnd, "Select Custom Tray Icon")
                        {
                            let edt = SETTINGS_UI.with(|c| c.borrow().edt_tray_path);
                            let _ = SetWindowTextW(edt, to_wide(&chosen).as_ptr());
                            refresh_icon_selector_state(true);
                        }
                        0
                    }
                    IDC_SET_CAP_BROWSE_BTN if code == BN_CLICKED => {
                        if let Some(chosen) =
                            select_file_from_disk(hwnd, "Select Custom Capture Cursor Icon")
                        {
                            let edt = SETTINGS_UI.with(|c| c.borrow().edt_cap_path);
                            let _ = SetWindowTextW(edt, to_wide(&chosen).as_ptr());
                            refresh_icon_selector_state(false);
                        }
                        0
                    }
                    IDC_SET_HIGHLIGHT_CHK if code == BN_CLICKED => {
                        let chk = SETTINGS_UI.with(|c| c.borrow().chk_highlight);
                        let on = SendMessageW(chk, BM_GETCHECK, 0, 0) as u32 == BST_CHECKED;
                        SETTINGS_UI.with(|c| c.borrow_mut().draft.highlight_target_on_hover = on);
                        0
                    }
                    IDC_SET_SAVE_BTN if code == BN_CLICKED => {
                        SETTINGS_UI.with(|cell| {
                            let mut st = cell.borrow_mut();
                            st.saved = true;
                            st.closed = true;
                        });
                        0
                    }
                    IDC_SET_CANCEL_BTN if code == BN_CLICKED => {
                        SETTINGS_UI.with(|cell| {
                            let mut st = cell.borrow_mut();
                            st.saved = false;
                            st.closed = true;
                        });
                        0
                    }
                    _ => DefWindowProcW(hwnd, msg, wparam, lparam),
                }
            }
            WM_CLOSE => {
                SETTINGS_UI.with(|cell| {
                    let mut st = cell.borrow_mut();
                    st.saved = false;
                    st.closed = true;
                });
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
