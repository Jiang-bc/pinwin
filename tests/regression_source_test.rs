use std::fs;

#[test]
fn mouse_move_recomptes_hover_target() {
    let src = fs::read_to_string("src/overlay.rs").unwrap();
    let block = src.split("WM_MOUSEMOVE =>").nth(1).unwrap().split("WM_KEYDOWN").next().unwrap();
    assert!(block.contains("update_hover_target("), "WM_MOUSEMOVE must refresh the hovered target");
}

#[test]
fn refresh_does_not_select_every_pinned_window() {
    let src = fs::read_to_string("src/ui.rs").unwrap();
    assert!(!src.contains("LB_SETSEL, TRUE as WPARAM, -1"), "refresh must not select all rows");
}
