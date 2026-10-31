use pinwin::ui::selection_indices;

#[test]
fn selection_model_preserves_explicit_selection() {
    assert_eq!(selection_indices(&[10, 20, 30], &[20, 99]), vec![1]);
    assert!(selection_indices(&[10, 20], &[]).is_empty());
    assert_eq!(selection_indices(&[30, 20], &[20, 30]), vec![0, 1]);
}
