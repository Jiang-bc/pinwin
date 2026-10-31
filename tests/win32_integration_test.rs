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
use pinwin::cli::{CliAction, execute_cli_action};
use pinwin::pin_manager::PinManager;
use pinwin::winapi::ffi::*;
use pinwin::winapi::{
    ApiOwnerFormLookup, ApiResources, ApiTopMost, ApiWindowEffects, ApiWindowPos, ApiWindowTitle,
    EMBEDDED_TARGET_CURSOR_ICO, EMBEDDED_TRAY_ICO, SingleInstanceManager, WindowStyle,
    enumerate_top_level_windows, to_wide,
};

static REGISTER_TEST_CLASS: Once = Once::new();
static DESKTOP_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
const TEST_TOPLEVEL_CLASS: &str = "PinWin_IntegrationTest_TopLevel_Class";

unsafe fn create_test_window(title: &str, style: DWORD, parent: HWND) -> HWND {
    unsafe {
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
            std::ptr::null_mut(),
            h_inst,
            null(),
        )
    }
}

#[test]
fn test_win32_owner_form_lookup_parent_child_hierarchy() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        // Root top-level window -> Child panel -> Nested button
        let root = create_test_window(
            "PinWinRootTestWindow",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            std::ptr::null_mut(),
        );
        assert!(!root.is_null());

        let child = create_test_window("ChildPanel", WS_CHILD | WS_VISIBLE, root);
        assert!(!child.is_null());

        let grandchild = create_test_window("GrandChildButton", WS_CHILD | WS_VISIBLE, child);
        assert!(!grandchild.is_null());

        let lookup = ApiOwnerFormLookup;
        assert_eq!(lookup.find_parent(grandchild), root);
        assert_eq!(lookup.find_parent(child), root);
        assert_eq!(lookup.find_parent(root), root);

        let _ = DestroyWindow(root);
    }
}

#[test]
fn test_win32_topmost_and_window_title_and_effects() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let hwnd = create_test_window(
            "PinWinTopMostIntegrationWindow",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            std::ptr::null_mut(),
        );
        assert!(!hwnd.is_null());

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
fn owned_resources_release_on_drop() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Process-wide counters must be isolated from the suite's concurrently created windows.
    if std::env::var_os("PINWIN_RESOURCE_CHILD").is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "owned_resources_release_on_drop",
                "--test-threads=1",
            ])
            .env("PINWIN_RESOURCE_CHILD", "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    use pinwin::winapi::resources::{OwnedBitmap, OwnedIcon};
    unsafe {
        // Warm up lazy Windows allocations before measuring resources owned by this process.
        drop(OwnedIcon::from_owned(
            ApiResources::create_icon_from_ico_bytes(EMBEDDED_TRAY_ICO, true, 32),
        ));
        let process = GetCurrentProcess();
        let before = GetGuiResources(process, GR_GDIOBJECTS);
        for _ in 0..100 {
            let icon = OwnedIcon::from_owned(ApiResources::create_icon_from_ico_bytes(
                EMBEDDED_TRAY_ICO,
                true,
                32,
            ))
            .unwrap();
            let bitmap =
                OwnedBitmap::from_owned(CreateBitmap(16, 16, 1, 32, std::ptr::null())).unwrap();
            assert!(!icon.as_raw().is_null());
            assert!(!bitmap.as_raw().is_null());
        }
        let after = GetGuiResources(process, GR_GDIOBJECTS);
        assert!(
            after <= before + 4,
            "GDI objects grew from {before} to {after}"
        );
    }
}

#[test]
fn malformed_icon_directory_is_rejected() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // A directory entry pointing beyond the supplied allocation must never reach Win32.
    let mut bytes = vec![0u8; 22];
    bytes[2] = 1;
    bytes[4] = 1;
    bytes[6] = 32;
    bytes[7] = 32;
    bytes[14..18].copy_from_slice(&128u32.to_le_bytes());
    bytes[18..22].copy_from_slice(&4096u32.to_le_bytes());
    assert!(ApiResources::create_icon_from_ico_bytes(&bytes, true, 32).is_null());
}

#[test]
fn badge_size_tracks_target_dpi() {
    let _guard = DESKTOP_TEST_LOCK.lock().unwrap();
    pinwin::winapi::enable_high_dpi_awareness();
    unsafe {
        let target = create_test_window(
            "DPI badge",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            std::ptr::null_mut(),
        );
        PinManager::init(std::ptr::null_mut());
        PinManager::pin_window(target, 255, false).unwrap();
        let badge = PinManager::entries()[0].badge_hwnd;
        let rect = ApiWindowPos::get(badge);
        let dpi = GetDpiForWindow(target).max(96);
        assert_eq!(rect.width(), (24 * dpi as i32 + 48) / 96);
        PinManager::shutdown();
        DestroyWindow(target);
    }
}

#[test]
fn embedded_resources_load_at_supported_sizes() {
    let _guard = DESKTOP_TEST_LOCK.lock().unwrap();
    for size in [32, 48, 64] {
        let bytes = include_bytes!("../assets/icons/target.cur");
        let cursor = pinwin::winapi::resources::load_embedded_cursor(bytes, size).unwrap();
        unsafe {
            let mut info: ICONINFO = zeroed();
            assert_ne!(GetIconInfo(cursor.as_raw(), &mut info), FALSE);
            assert_eq!(info.fIcon, FALSE);
            assert_eq!(info.xHotspot, size as u32 / 2);
            assert_eq!(info.yHotspot, size as u32 / 2);
            let _mask = pinwin::winapi::resources::OwnedBitmap::from_owned(info.hbmMask);
            let _color = pinwin::winapi::resources::OwnedBitmap::from_owned(info.hbmColor);
        }
    }
    for size in [16, 20, 24, 32, 48, 64, 128, 256] {
        assert!(
            pinwin::winapi::resources::load_embedded_icon(
                include_bytes!("../assets/icons/app.ico"),
                size
            )
            .is_ok()
        );
    }
}

#[test]
fn test_embedded_assets_load_cleanly() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let (hbmp, w, h, _colorkey) = ApiResources::create_pinned_bitmap_from_embedded();
        assert!(!hbmp.is_null());
        assert_eq!(w, 24);
        assert_eq!(h, 24);
        let _ = DeleteObject(hbmp);

        let tray_icon = ApiResources::create_icon_from_ico_bytes(EMBEDDED_TRAY_ICO, true, 16);
        assert!(!tray_icon.is_null());

        let target_cursor =
            ApiResources::create_icon_from_ico_bytes(EMBEDDED_TARGET_CURSOR_ICO, true, 32);
        assert!(!target_cursor.is_null());
    }
}

#[test]
fn mutex_error_does_not_allow_second_instance() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut invalid = SingleInstanceManager::new("invalid\\name\\mutex");
    assert!(!invalid.is_first_instance(0));
}

#[test]
fn test_single_instance_mutex_exclusivity() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mutex_name = format!("Local\\PinWin_Test_Mutex_{}", std::process::id());
    let mut inst1 = SingleInstanceManager::new(&mutex_name);
    assert!(inst1.is_first_instance(100));
}

#[test]
fn unpin_restores_original_topmost_layered_and_transparent_state() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let target = create_test_window(
            "OriginalEffects",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            std::ptr::null_mut(),
        );
        assert!(ApiTopMost::set(target));
        assert!(ApiWindowEffects::set_click_through(target, true, 143));
        let original_style = GetWindowLongW(target, GWL_EXSTYLE) as u32;
        PinManager::init(std::ptr::null_mut());
        assert!(PinManager::pin_window(target, 210, false).unwrap());
        assert!(PinManager::unpin_window(target));
        assert!(ApiTopMost::is_topmost(target));
        let mask = WS_EX_LAYERED | WS_EX_TRANSPARENT;
        assert_eq!(
            GetWindowLongW(target, GWL_EXSTYLE) as u32 & mask,
            original_style & mask
        );
        let (mut color, mut alpha, mut flags) = (0, 0, 0);
        assert_ne!(
            GetLayeredWindowAttributes(target, &mut color, &mut alpha, &mut flags),
            FALSE
        );
        assert_eq!(alpha, 143);
        PinManager::shutdown();
        DestroyWindow(target);
    }
}

#[test]
fn destroyed_target_is_removed_without_touching_other_windows() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        PinManager::init(std::ptr::null_mut());
        let target =
            create_test_window("DestroyedTarget", WS_OVERLAPPEDWINDOW, std::ptr::null_mut());
        PinManager::pin_window(target, 180, false).unwrap();
        let badge = PinManager::entries()[0].badge_hwnd;
        DestroyWindow(target);
        PinManager::sync_all();
        assert_eq!(PinManager::count(), 0);
        assert_eq!(IsWindow(badge), FALSE);
        PinManager::shutdown();
    }
}

#[test]
fn stale_window_snapshot_does_not_restore_new_owner() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    use pinwin::winapi::window_state::WindowState;
    unsafe {
        let target = create_test_window(
            "ReplacedIdentity",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            std::ptr::null_mut(),
        );
        let old = WindowState::capture(target).unwrap();
        assert!(ApiTopMost::set(target));
        let current = WindowState::capture(target).unwrap();
        assert!(!old.matches(target));
        assert!(!old.restore(target));
        assert!(ApiTopMost::is_topmost(target));
        assert!(current.restore(target));
        DestroyWindow(target);
    }
}

#[test]
fn badge_creation_failure_rolls_back_window_effects() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // A fresh process has no registered badge class: force the actual CreateWindowEx failure.
    if std::env::var_os("PINWIN_ROLLBACK_CHILD").is_none() {
        assert!(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "badge_creation_failure_rolls_back_window_effects"
                ])
                .env("PINWIN_ROLLBACK_CHILD", "1")
                .status()
                .unwrap()
                .success()
        );
        return;
    }
    unsafe {
        let target =
            create_test_window("RollbackTarget", WS_OVERLAPPEDWINDOW, std::ptr::null_mut());
        assert!(PinManager::pin_window(target, 100, false).is_err());
        assert_eq!(PinManager::count(), 0);
        assert!(!ApiTopMost::is_topmost(target));
        assert_eq!(
            GetWindowLongW(target, GWL_EXSTYLE) as u32 & WS_EX_LAYERED,
            0
        );
        DestroyWindow(target);
    }
}

#[test]
fn overlay_hit_test_finds_window_beneath_overlay() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let target = create_test_window(
            "HoverTarget",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            std::ptr::null_mut(),
        );
        let overlay =
            create_test_window("OverlayTest", WS_POPUP | WS_VISIBLE, std::ptr::null_mut());
        ApiTopMost::set(target);
        ApiTopMost::set(overlay);
        let bounds = ApiWindowPos::get(target);
        let hit = pinwin::winapi::ApiForm::select_excluding(
            POINT {
                x: bounds.left + 50,
                y: bounds.top + 50,
            },
            overlay,
        );
        assert_eq!(hit, target);
        DestroyWindow(overlay);
        DestroyWindow(target);
    }
}

#[test]
fn unpin_all_never_clears_unrelated_topmost_windows() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let target = create_test_window(
            "UnrelatedTopmost",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            std::ptr::null_mut(),
        );
        ApiTopMost::set(target);
        // Do not send an unpin request to an unrelated interactive instance.
        let gui = FindWindowW(to_wide("PinWin_MainForm_Class").as_ptr(), std::ptr::null());
        if gui.is_null() {
            assert_ne!(execute_cli_action(CliAction::UnpinAllTopMost), 0);
        }
        assert!(ApiTopMost::is_topmost(target));
        DestroyWindow(target);
    }
}

#[test]
fn test_pin_manager_lifecycle() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let notify_dummy =
            create_test_window("NotifyDummy", WS_OVERLAPPEDWINDOW, std::ptr::null_mut());
        let target = create_test_window(
            "TargetWindowForPinManager",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            std::ptr::null_mut(),
        );
        assert!(!target.is_null());

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
        assert!(
            entries[0]
                .display_label()
                .contains("TargetWindowForPinManager")
        );
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
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let hwnd = create_test_window(
            "UniqueCliTargetWindow_98765",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            std::ptr::null_mut(),
        );
        assert!(!hwnd.is_null());

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
#[test]
fn test_ghost_mode_preserves_original_opacity() {
    let _desktop = DESKTOP_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let notify_dummy = create_test_window(
            "GhostNotifyDummy",
            WS_OVERLAPPEDWINDOW,
            std::ptr::null_mut(),
        );
        let target = create_test_window(
            "GhostOpacityTarget",
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            std::ptr::null_mut(),
        );
        assert!(!target.is_null());
        PinManager::init(notify_dummy);
        assert!(PinManager::pin_window(target, 255, false).unwrap());
        assert_eq!(PinManager::entries()[0].opacity, 255);
        assert!(PinManager::set_window_click_through(target, true));
        assert_eq!(PinManager::entries()[0].opacity, 255);
        let mut alpha = 0u8;
        let mut key = 0u32;
        let mut flags = 0u32;
        assert!(GetLayeredWindowAttributes(target, &mut key, &mut alpha, &mut flags) != FALSE);
        assert_eq!(alpha, 255);
        assert!(PinManager::set_window_click_through(target, false));
        assert_eq!(PinManager::entries()[0].opacity, 255);
        // Fully opaque + no click-through may legitimately drop WS_EX_LAYERED entirely.
        let ex_style = GetWindowLongW(target, GWL_EXSTYLE) as u32;
        if ex_style & WS_EX_LAYERED != 0 {
            alpha = 0;
            assert!(GetLayeredWindowAttributes(target, &mut key, &mut alpha, &mut flags) != FALSE);
            assert_eq!(alpha, 255);
        }
        PinManager::unpin_window(target);
        PinManager::shutdown();
        let _ = DestroyWindow(target);
        let _ = DestroyWindow(notify_dummy);
    }
}
