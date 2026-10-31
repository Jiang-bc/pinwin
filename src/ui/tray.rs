use super::*;

pub(super) unsafe fn update_tray_icon(hwnd: HWND, hicon: HICON, balloon_text: Option<&str>) {
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

pub(super) unsafe fn remove_tray_icon(hwnd: HWND) {
    unsafe {
        let mut nid: NOTIFYICONDATAW = zeroed();
        nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

pub(super) unsafe fn show_tray_context_menu(hwnd: HWND) {
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
