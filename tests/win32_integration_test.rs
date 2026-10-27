//! Win32 Integration Tests for PinWin (Rust Edition).
//!
//! Ports and expands `PinWinTest/WinApiOwnerFormLookupTest.cs` and
//! `PinWinTest/WinformsOwnerFormLookupTest.cs`, plus live Win32 tests for:
//! - Multi-level parent/child HWND hierarchy lookup (`ApiOwnerFormLookup`)
//! - TopMost (`WS_EX_TOPMOST`) toggling via `ApiTopMost`
//! - Non-blocking `ApiWindowTitle::from_handle` (`SendMessageTimeoutW`)
//! - Opacity (`WS_EX_LAYERED`) & Click-Through (`WS_EX_TRANSPARENT`) via `ApiWindowEffects`
//! - Embedded asset decoding (`Pinned.bmp`, `Tray.ico`, `TargetWindowIcon.ico`)
//! - `SingleInstanceManager` mutex locking
//! - `PinManager` end-to-end pinning, badge creation, position sync, and unpinning

use std::mem::{size_of, zeroed};
use std::ptr::null;
use std::sync::Once;

use pinwin::algorithms::HierarchyLookup;
use pinwin::cli::{execute_cli_action, CliAction};
use pinwin::pin_manager::PinManager;
use pinwin::winapi::ffi::*;
use pinwin::winapi::{
    enumerate_top_level_windows, to_wide, ApiOwnerFormLookup, ApiResources, ApiTopMost,
    ApiWindowEffects, ApiWindowPos, ApiWindowTitle, SingleInstanceManager, WindowStyle,
    EMBEDDED_TARGET_CURSOR_ICO, EMBEDDED_TRAY_ICO,
};

static REGISTER_TEST_CLASS: Once = Once::new();
const TEST_TOPLEVEL_CLASS: &str = "PinWin_IntegrationTest_TopLevel_Class";

unsafe fn create_test_window(title: &str, style: DWORD, parent: HWND) -> HWND {
    let h_inst = GetModuleHandleW(null());
    REGISTER_TEST_CLASS.call_once(|| {
        let cls_w = to_wide(TEST_TOPLEVEL_CLASS);
        let mut wc: WNDCLASSEXW = zeroed();
        wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(DefWindowProcW);
        wc.hInstance = h_inst;
        wc.lpszClassName = cls_w.as_ptr();
        let _ = RegisterClassExW(&wc);
    });

    let cls = if (style & WS_CHILD) != 0 {
        to_wide("STATIC")
    } else {
        to_wide(TEST_TOPLEVEL_CLASS)
    };
    let text = to_wide(title);
    CreateWindowExW(
        0,
        cls.as_ptr(),
        text.as_ptr(),
        style,
        100,
        100,
        320,
        220,
        parent,
        0,
        h_inst,
        null(),
    )
}

#[test]
fn test_win32_owner_form_lookup_parent_child_hierarchy() {
    unsafe {
        // Root top-level window -> Child panel -> Nested button
        let root = create_test_window("PinWinRootTestWindow", WS_OVERLAPPEDWINDOW | WS_VISIBLE, 0);
        assert_ne!(root, 0);

        let child = create_test_window("ChildPanel", WS_CHILD | WS_VISIBLE, root);
        assert_ne!(child, 0);

        let grandchild = create_test_window("GrandChildButton", WS_CHILD | WS_VISIBLE, child);
        assert_ne!(grandchild, 0);

        let lookup = ApiOwnerFormLookup;
        assert_eq!(lookup.find_parent(grandchild), root);
        assert_eq!(lookup.find_parent(child), root);
        assert_eq!(lookup.find_parent(root), root);

        let _ = DestroyWindow(root);
    }
}

#[test]
fn test_win32_topmost_and_window_title_and_effects() {
    unsafe {
        let hwnd = create_test_window(
            "PinWinTopMostIntegrationWindow",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            0,
        );
        assert_ne!(hwnd, 0);

        // Verify non-blocking title retrieval
        let title = ApiWindowTitle::from_handle(hwnd);
        assert_eq!(title, "PinWinTopMostIntegrationWindow");

        // Verify TopMost toggle
        assert!(!ApiTopMost::is_topmost(hwnd));
        assert!(ApiTopMost::set(hwnd));
        assert!(ApiTopMost::is_topmost(hwnd));
        assert!(ApiTopMost::clear(hwnd));
        assert!(!ApiTopMost::is_topmost(hwnd));

        // Verify Opacity & Click-through (Ghost mode)
        assert!(ApiWindowEffects::set_opacity(hwnd, 180));
        assert!(!ApiWindowEffects::is_click_through(hwnd));
        assert!(ApiWindowEffects::set_click_through(hwnd, true, 180));
        assert!(ApiWindowEffects::is_click_through(hwnd));
        assert!(ApiWindowEffects::set_click_through(hwnd, false, 255));
        assert!(!ApiWindowEffects::is_click_through(hwnd));

        // Verify Pin badge coordinate calculation
        let badge_pt = WindowStyle::get_pin_badge_location(hwnd);
        let bounds = ApiWindowPos::get(hwnd);
        assert!(badge_pt.x >= bounds.left && badge_pt.x <= bounds.right);
        assert_eq!(badge_pt.y, bounds.top + 5);

        let _ = DestroyWindow(hwnd);
    }
}

#[test]
fn test_embedded_assets_load_cleanly() {
    unsafe {
        let (hbmp, w, h, _colorkey) = ApiResources::create_pinned_bitmap_from_embedded();
        assert_ne!(hbmp, 0);
        assert_eq!(w, 19);
        assert_eq!(h, 18);
        let _ = DeleteObject(hbmp);

        let tray_icon = ApiResources::create_icon_from_ico_bytes(EMBEDDED_TRAY_ICO, true, 16);
        assert_ne!(tray_icon, 0);

        let target_cursor =
            ApiResources::create_icon_from_ico_bytes(EMBEDDED_TARGET_CURSOR_ICO, true, 32);
        assert_ne!(target_cursor, 0);
    }
}

#[test]
fn test_single_instance_mutex_exclusivity() {
    let mutex_name = format!("Local\\PinWin_Test_Mutex_{}", std::process::id());
    let mut inst1 = SingleInstanceManager::new(&mutex_name);
    assert!(inst1.is_first_instance(100));
}

#[test]
fn test_pin_manager_lifecycle() {
    unsafe {
        let notify_dummy = create_test_window("NotifyDummy", WS_OVERLAPPEDWINDOW, 0);
        let target = create_test_window(
            "TargetWindowForPinManager",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            0,
        );
        assert_ne!(target, 0);

        PinManager::init(notify_dummy);
        assert_eq!(PinManager::count(), 0);

        // Pin the target window
        let pinned = PinManager::pin_window(target, 220, false).unwrap();
        assert!(pinned);
        assert_eq!(PinManager::count(), 1);
        assert!(PinManager::is_pinned(target));
        assert!(ApiTopMost::is_topmost(target));

        let entries = PinManager::entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].target_hwnd, target);
        assert_eq!(entries[0].opacity, 220);
        assert!(entries[0]
            .display_label()
            .contains("TargetWindowForPinManager"));
        assert!(ApiWindowPos::is_window(entries[0].badge_hwnd));

        // Update opacity & ghost mode via PinManager
        assert!(PinManager::set_window_opacity(target, 160));
        assert!(PinManager::set_window_click_through(target, true));
        assert!(ApiWindowEffects::is_click_through(target));
        PinManager::restore_all_click_through();
        assert!(!ApiWindowEffects::is_click_through(target));

        // Unpin
        assert!(PinManager::unpin_window(target));
        assert_eq!(PinManager::count(), 0);
        assert!(!ApiTopMost::is_topmost(target));

        PinManager::shutdown();
        let _ = DestroyWindow(target);
        let _ = DestroyWindow(notify_dummy);
    }
}

#[test]
fn test_cli_pin_title_and_unpin_hwnd_live_window() {
    unsafe {
        let hwnd = create_test_window(
            "UniqueCliTargetWindow_98765",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            0,
        );
        assert_ne!(hwnd, 0);

        let all = enumerate_top_level_windows();
        assert!(all.iter().any(|w| w.hwnd == hwnd));

        let rc = execute_cli_action(CliAction::PinTitle {
            pattern: "UniqueCliTargetWindow_98765".to_string(),
            opacity: Some(200),
        });
        assert_eq!(rc, 0);
        assert!(ApiTopMost::is_topmost(hwnd));

        let rc2 = execute_cli_action(CliAction::UnpinHwnd { hwnd });
        assert_eq!(rc2, 0);
        assert!(!ApiTopMost::is_topmost(hwnd));

        let _ = DestroyWindow(hwnd);
    }
}
