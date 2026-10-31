use super::*;

pub fn run_gui_app(start_minimized: bool) -> i32 {
    let initial_settings = ApplicationSettings::load_from_file(None);
    MAIN_UI.with(|cell| {
        cell.borrow_mut().settings = initial_settings;
    });

    unsafe {
        let h_inst = GetModuleHandleW(null());
        let class_name = to_wide(MAIN_WINDOW_CLASS);
        let app_icon = OwnedIcon::from_owned(ApiResources::create_icon_from_ico_bytes(
            crate::winapi::EMBEDDED_APP_ICO,
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

pub(super) unsafe fn create_main_controls(hwnd: HWND) {
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

pub(super) unsafe fn apply_current_settings(hwnd: HWND) {
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

pub(super) unsafe fn refresh_pinned_listbox() {
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
        for idx in selection_indices(&hwnds, &previously_selected) {
            let _ = SendMessageW(lst_hwnd, LB_SETSEL, TRUE as WPARAM, idx as LPARAM);
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

pub(super) unsafe fn update_selection_action_buttons(
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

pub(super) unsafe fn get_selected_target_hwnds() -> Vec<HWND> {
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

pub(super) unsafe fn trigger_select_window_overlay(hwnd: HWND) {
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

pub(super) unsafe fn trigger_pin_window_under_cursor(hwnd: HWND) {
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

pub(super) unsafe fn handle_menu_or_button_command(hwnd: HWND, cmd_id: usize) {
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

pub(super) unsafe extern "system" fn main_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let taskbar_created = RegisterWindowMessageW(to_wide("TaskbarCreated").as_ptr());
        if taskbar_created != 0 && msg == taskbar_created {
            let icon = MAIN_UI.with(|c| c.borrow().tray_icon);
            update_tray_icon(hwnd, icon, None);
            return 0;
        }
        match msg {
            WM_DPICHANGED => {
                update_dpi_layout(hwnd, wparam, lparam);
                0
            }
            WM_CREATE => {
                create_main_controls(hwnd);
                initial_dpi_layout(hwnd);
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
            crate::pin_manager::WM_PINWIN_UNPIN_ALL => {
                let count = PinManager::count();
                PinManager::unpin_all();
                count as LRESULT
            }
            WM_PINWIN_WINDOW_EVENT => {
                PinManager::sync_window(wparam as HWND);
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
