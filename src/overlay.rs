//! Full-screen interactive window selection overlay (`DesktopOverlayForm` & `OverlayForm`).
//!
//! Ported and enhanced from:
//! - `PinWin/DesktopOverlayForm.cs`
//! - `PinWin/Controls/OverlayForm.cs`
//! - `PinWin/BusinessLayer/ScreenCapture.cs`
//!
//! Enhancements over original C# implementation:
//! - Zero-flicker double-buffered GDI rendering (`WM_ERASEBKGND` returns 1).
//! - Live target window bounding-box highlight and title preview HUD banner while moving cursor.
//! - Multi-monitor virtual desktop coordinate support (`SM_XVIRTUALSCREEN..SM_CYVIRTUALSCREEN`).

use std::cell::RefCell;
use std::mem::{size_of, zeroed};
use std::ptr::null;

use crate::winapi::ffi::*;
use crate::winapi::resources::{OwnedBitmap, OwnedCursor};
use crate::winapi::{
    ApiForm, ApiResources, ApiWindowPos, ApiWindowTitle, EMBEDDED_TARGET_CURSOR_ICO,
    capture_desktop_bitmap, to_wide,
};

const OVERLAY_CLASS_NAME: &str = "PinWin_DesktopOverlay_Class";

struct OverlaySession {
    desktop_bounds: RECT,
    screenshot_bmp: HBITMAP,
    owned_screenshot: Option<OwnedBitmap>,
    owned_cursor: Option<OwnedCursor>,
    target_cursor: HCURSOR,
    cursor_pt: POINT,
    hover_hwnd: HWND,
    hover_rect: RECT,
    hover_title: String,
    highlight_on_hover: bool,
    selected_point: Option<POINT>,
    finished: bool,
}

impl Default for OverlaySession {
    fn default() -> Self {
        Self {
            desktop_bounds: RECT::default(),
            screenshot_bmp: std::ptr::null_mut(),
            owned_screenshot: None,
            owned_cursor: None,
            target_cursor: std::ptr::null_mut(),
            cursor_pt: POINT::default(),
            hover_hwnd: std::ptr::null_mut(),
            hover_rect: RECT::default(),
            hover_title: String::new(),
            highlight_on_hover: true,
            selected_point: None,
            finished: false,
        }
    }
}

thread_local! {
    static OVERLAY_STATE: RefCell<OverlaySession> = RefCell::new(OverlaySession::default());
    static OVERLAY_CLASS_REGISTERED: RefCell<bool> = const { RefCell::new(false) };
}

pub struct DesktopOverlay;

impl DesktopOverlay {
    /// Open the full-screen window selection overlay and return `Some(POINT)` when the user
    /// left-clicks a target window, or `None` if cancelled via `ESC` or Right-click.
    pub fn select_point(custom_cursor_path: &str, highlight_on_hover: bool) -> Option<POINT> {
        let bounds = ApiWindowPos::get_desktop_bounds();
        let screenshot_bmp = capture_desktop_bitmap(bounds);
        let cursor =
            ApiResources::load_cursor_or_embedded(custom_cursor_path, EMBEDDED_TARGET_CURSOR_ICO);

        let mut init_pt = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut init_pt);
        }

        OVERLAY_STATE.with(|cell| {
            let mut st = cell.borrow_mut();
            st.desktop_bounds = bounds;
            st.owned_screenshot = unsafe { OwnedBitmap::from_owned(screenshot_bmp) };
            st.owned_cursor = unsafe { OwnedCursor::from_owned(cursor) };
            st.screenshot_bmp = screenshot_bmp;
            st.target_cursor = cursor;
            st.cursor_pt = init_pt;
            st.hover_hwnd = std::ptr::null_mut();
            st.hover_rect = RECT::default();
            st.hover_title.clear();
            st.highlight_on_hover = highlight_on_hover;
            st.selected_point = None;
            st.finished = false;
        });

        ensure_overlay_class_registered();

        let hwnd = unsafe {
            let h_inst = GetModuleHandleW(null());
            let class_name = to_wide(OVERLAY_CLASS_NAME);
            let title = to_wide("PinWin Window Selector");
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_VISIBLE,
                bounds.left,
                bounds.top,
                bounds.width(),
                bounds.height(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                h_inst,
                null(),
            )
        };

        if hwnd.is_null() {
            cleanup_overlay_resources();
            return None;
        }

        // Compute initial hover target before showing
        update_hover_target(hwnd, init_pt);

        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
            let _ = SetFocus(hwnd);
            let _ = UpdateWindow(hwnd);

            let mut msg: MSG = zeroed();
            while !OVERLAY_STATE.with(|c| c.borrow().finished) {
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

            if ApiWindowPos::is_window(hwnd) {
                let _ = DestroyWindow(hwnd);
            }
        }

        let result = OVERLAY_STATE.with(|c| c.borrow().selected_point);
        cleanup_overlay_resources();
        result
    }
}

fn cleanup_overlay_resources() {
    OVERLAY_STATE.with(|cell| {
        let mut st = cell.borrow_mut();
        st.owned_screenshot = None;
        st.owned_cursor = None;
        st.screenshot_bmp = std::ptr::null_mut();
        st.target_cursor = std::ptr::null_mut();
    });
}

fn ensure_overlay_class_registered() {
    OVERLAY_CLASS_REGISTERED.with(|reg| {
        if !*reg.borrow() {
            unsafe {
                let h_inst = GetModuleHandleW(null());
                let class_name = to_wide(OVERLAY_CLASS_NAME);
                let mut wc: WNDCLASSEXW = zeroed();
                wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
                wc.lpfnWndProc = Some(overlay_wnd_proc);
                wc.hInstance = h_inst;
                wc.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_CROSS);
                wc.lpszClassName = class_name.as_ptr();
                let _ = RegisterClassExW(&wc);
            }
            *reg.borrow_mut() = true;
        }
    });
}

fn update_hover_target(overlay_hwnd: HWND, screen_pt: POINT) {
    let highlight = OVERLAY_STATE.with(|c| c.borrow().highlight_on_hover);
    if !highlight {
        OVERLAY_STATE.with(|c| c.borrow_mut().cursor_pt = screen_pt);
        return;
    }

    // Temporarily hide or skip overlay to inspect underlying window if needed
    let raw_target = ApiForm::select(screen_pt);
    if !raw_target.is_null() && raw_target != overlay_hwnd {
        let rect = ApiWindowPos::get(raw_target);
        let title = ApiWindowTitle::from_handle(raw_target);
        OVERLAY_STATE.with(|cell| {
            let mut st = cell.borrow_mut();
            st.cursor_pt = screen_pt;
            st.hover_hwnd = raw_target;
            st.hover_rect = rect;
            st.hover_title = title;
        });
    } else {
        OVERLAY_STATE.with(|cell| {
            let mut st = cell.borrow_mut();
            st.cursor_pt = screen_pt;
        });
    }
}

unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_ERASEBKGND => 1, // Prevent background erase flicker
            WM_SETCURSOR => {
                let cursor = OVERLAY_STATE.with(|c| c.borrow().target_cursor);
                if !cursor.is_null() {
                    SetCursor(cursor);
                    return 1;
                }
                DefWindowProcW(hwnd, msg, wparam, _lparam)
            }
            WM_MOUSEMOVE => {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                update_hover_target(hwnd, pt);
                let _ = InvalidateRect(hwnd, null(), FALSE);
                0
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                if wparam as u32 == crate::keys::VK_ESCAPE {
                    OVERLAY_STATE.with(|cell| {
                        let mut st = cell.borrow_mut();
                        st.selected_point = None;
                        st.finished = true;
                    });
                }
                0
            }
            WM_RBUTTONUP | WM_MBUTTONUP => {
                OVERLAY_STATE.with(|cell| {
                    let mut st = cell.borrow_mut();
                    st.selected_point = None;
                    st.finished = true;
                });
                0
            }
            WM_LBUTTONUP => {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                let _ = ShowWindow(hwnd, SW_HIDE);
                OVERLAY_STATE.with(|cell| {
                    let mut st = cell.borrow_mut();
                    st.selected_point = Some(pt);
                    st.finished = true;
                });
                0
            }
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);

                let (bounds, screenshot_bmp, cursor_pt, hover_rect, hover_title, highlight) =
                    OVERLAY_STATE.with(|cell| {
                        let st = cell.borrow();
                        (
                            st.desktop_bounds,
                            st.screenshot_bmp,
                            st.cursor_pt,
                            st.hover_rect,
                            st.hover_title.clone(),
                            st.highlight_on_hover,
                        )
                    });

                let w = bounds.width();
                let h = bounds.height();

                // Double-buffer offscreen DC
                let back_dc = CreateCompatibleDC(hdc);
                let back_bmp = CreateCompatibleBitmap(hdc, w, h);
                let old_back = SelectObject(back_dc, back_bmp);

                if !screenshot_bmp.is_null() {
                    let src_dc = CreateCompatibleDC(hdc);
                    let old_src = SelectObject(src_dc, screenshot_bmp);
                    let _ = BitBlt(back_dc, 0, 0, w, h, src_dc, 0, 0, SRCCOPY);
                    SelectObject(src_dc, old_src);
                    DeleteDC(src_dc);
                }

                // Draw highlight border around initial hovered window if enabled
                if highlight && hover_rect.width() > 0 && hover_rect.height() > 0 {
                    let rel_rect = RECT::new(
                        hover_rect.left - bounds.left,
                        hover_rect.top - bounds.top,
                        hover_rect.right - bounds.left,
                        hover_rect.bottom - bounds.top,
                    );
                    let border_brush = CreateSolidBrush(rgb(0, 174, 239));
                    let mut r = rel_rect;
                    for _ in 0..3 {
                        let _ = FrameRect(back_dc, &r, border_brush);
                        r.left += 1;
                        r.top += 1;
                        r.right -= 1;
                        r.bottom -= 1;
                    }
                    let _ = DeleteObject(border_brush);
                }

                // Draw HUD instruction & coordinate banner at top-center AND near cursor
                let hud_bg = CreateSolidBrush(rgb(24, 28, 36));
                let hud_border = CreateSolidBrush(rgb(0, 174, 239));
                let mut banner_rc = RECT::new((w / 2) - 270, 18, (w / 2) + 270, 56);
                let _ = FillRect(back_dc, &banner_rc, hud_bg);
                let _ = FrameRect(back_dc, &banner_rc, hud_border);

                let font = GetStockObject(DEFAULT_GUI_FONT);
                let old_font = SelectObject(back_dc, font);
                SetBkMode(back_dc, TRANSPARENT.try_into().unwrap());
                SetTextColor(back_dc, rgb(255, 255, 255));

                let banner_text = format!(
                    "PinWin Target Mode — Left-Click Window to Pin  |  ESC / Right-Click to Cancel  ({}, {})",
                    cursor_pt.x, cursor_pt.y
                );
                let banner_wide = to_wide(&banner_text);
                let _ = DrawTextW(
                    back_dc,
                    banner_wide.as_ptr(),
                    -1,
                    &mut banner_rc,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );

                // Also draw live coordinate badge next to cursor (matching OverlayForm.OnPaint)
                let rel_x = (cursor_pt.x - bounds.left + 18).clamp(8, (w - 220).max(8));
                let rel_y = (cursor_pt.y - bounds.top + 18).clamp(8, (h - 36).max(8));
                let mut tip_rc = RECT::new(rel_x, rel_y, rel_x + 210, rel_y + 26);
                let _ = FillRect(back_dc, &tip_rc, hud_bg);
                let _ = FrameRect(back_dc, &tip_rc, hud_border);
                let tip_str = if hover_title.is_empty() {
                    format!(" X: {}, Y: {}", cursor_pt.x, cursor_pt.y)
                } else {
                    format!(" ({}, {}) {}", cursor_pt.x, cursor_pt.y, hover_title)
                };
                let tip_wide = to_wide(&tip_str);
                let _ = DrawTextW(
                    back_dc,
                    tip_wide.as_ptr(),
                    -1,
                    &mut tip_rc,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                );

                SelectObject(back_dc, old_font);
                let _ = DeleteObject(hud_bg);
                let _ = DeleteObject(hud_border);

                let _ = BitBlt(hdc, 0, 0, w, h, back_dc, 0, 0, SRCCOPY);
                SelectObject(back_dc, old_back);
                let _ = DeleteObject(back_bmp);
                let _ = DeleteDC(back_dc);

                EndPaint(hwnd, &ps);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, _lparam),
        }
    }
}
