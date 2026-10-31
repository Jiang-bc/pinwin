use super::*;

pub(super) unsafe fn open_settings_dialog(owner_hwnd: HWND) {
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
        DPI_FONTS.with(|c| c.borrow_mut().remove(&(dlg_hwnd as usize)));
    }
}

pub(super) unsafe fn create_settings_controls(hwnd: HWND) {
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

pub(super) unsafe fn populate_settings_controls_from_draft() {
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

pub(super) unsafe fn refresh_icon_selector_state(is_tray: bool) {
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
            if read_window_text(edt_hwnd) != default_path {
                let _ = SetWindowTextW(edt_hwnd, to_wide(default_path).as_ptr());
            }
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

pub(super) unsafe fn replace_preview_icon(hwnd: HWND, raw: HICON) {
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

pub(super) unsafe fn read_window_text(hwnd: HWND) -> String {
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

pub(super) unsafe extern "system" fn hotkey_edit_subclass_proc(
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

pub(super) unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_DPICHANGED => {
                update_dpi_layout(hwnd, wparam, lparam);
                0
            }
            WM_CREATE => {
                create_settings_controls(hwnd);
                initial_dpi_layout(hwnd);
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
                        let draft = SETTINGS_UI.with(|c| c.borrow().draft.clone());
                        let owner = MAIN_UI.with(|c| c.borrow().hwnd);
                        let path = ApplicationSettings::get_settings_file_path(
                            crate::settings::SETTINGS_FILENAME,
                            true,
                        );
                        match persist_settings(owner, &draft, &path) {
                            Ok(()) => {
                                apply_current_settings(owner);
                                SETTINGS_UI.with(|c| {
                                    let mut st = c.borrow_mut();
                                    st.saved = true;
                                    st.closed = true;
                                });
                            }
                            Err(error) => {
                                let message = to_wide(&format!(
                                    "Could not save settings to {}.\r\n{}",
                                    path.display(),
                                    error
                                ));
                                MessageBoxW(
                                    hwnd,
                                    message.as_ptr(),
                                    to_wide("PinWin Settings").as_ptr(),
                                    MB_OK | MB_ICONERROR,
                                );
                            }
                        }
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

fn register_shortcuts(hwnd: HWND, settings: &ApplicationSettings) -> bool {
    ApiHotKey::clear(hwnd, HOTKEY_ID_PROMPT);
    ApiHotKey::clear(hwnd, HOTKEY_ID_UNDER_CURSOR);
    let a = settings.shortcut_pin_window_prompt;
    let b = settings.shortcut_pin_window_under_cursor;
    (a.is_none() || ApiHotKey::set(hwnd, a, HOTKEY_ID_PROMPT))
        && (b.is_none() || ApiHotKey::set(hwnd, b, HOTKEY_ID_UNDER_CURSOR))
}

fn persist_settings(hwnd: HWND, draft: &ApplicationSettings, path: &Path) -> Result<(), String> {
    let previous = MAIN_UI.with(|c| c.borrow().settings.clone());
    let a = draft.shortcut_pin_window_prompt;
    let b = draft.shortcut_pin_window_under_cursor;
    if !a.is_none() && a == b {
        return Err("Choose different shortcuts for the two actions.".into());
    }
    if !hwnd.is_null() && !register_shortcuts(hwnd, draft) {
        register_shortcuts(hwnd, &previous);
        return Err("A shortcut is already in use. The previous shortcuts were restored.".into());
    }
    if let Err(error) = draft.save_to_explicit_path(path) {
        if !hwnd.is_null() {
            register_shortcuts(hwnd, &previous);
        }
        return Err(error.to_string());
    }
    MAIN_UI.with(|c| c.borrow_mut().settings = draft.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_save_does_not_replace_active_settings() {
        let previous = MAIN_UI.with(|c| c.borrow().settings.clone());
        let mut draft = previous.clone();
        draft.default_opacity = 100;
        // Replacing a directory with a settings file must fail even when parents are created.
        let path = std::env::temp_dir();
        assert!(persist_settings(std::ptr::null_mut(), &draft, &path).is_err());
        assert_eq!(
            MAIN_UI.with(|c| c.borrow().settings.default_opacity),
            previous.default_opacity
        );
    }
}
