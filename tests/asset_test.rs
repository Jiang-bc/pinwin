use std::path::Path;

#[test]
fn ico_and_cursor_directories_are_valid() {
    for (file, kind, sizes) in [
        ("app.ico", 1u16, vec![16, 20, 24, 32, 48, 64, 128, 256]),
        ("tray.ico", 1, vec![16, 20, 24, 32]),
        ("target.cur", 2, vec![32, 48, 64]),
    ] {
        let bytes = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets/icons")
                .join(file),
        )
        .unwrap();
        assert_eq!(u16::from_le_bytes(bytes[2..4].try_into().unwrap()), kind);
        assert_eq!(
            u16::from_le_bytes(bytes[4..6].try_into().unwrap()) as usize,
            sizes.len()
        );
        for (entry, size) in bytes[6..6 + sizes.len() * 16]
            .as_chunks::<16>()
            .0
            .iter()
            .zip(sizes)
        {
            assert_eq!(
                if entry[0] == 0 {
                    256
                } else {
                    entry[0] as usize
                },
                size
            );
            let len = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
            let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
            assert!(offset >= 6 + sizes_len(file) * 16 && offset + len <= bytes.len());
            if kind == 2 {
                assert_eq!(
                    u16::from_le_bytes(entry[4..6].try_into().unwrap()) as usize,
                    size / 2
                );
                assert_eq!(
                    u16::from_le_bytes(entry[6..8].try_into().unwrap()) as usize,
                    size / 2
                );
            }
        }
    }
}
fn sizes_len(file: &str) -> usize {
    match file {
        "app.ico" => 8,
        "tray.ico" => 4,
        _ => 3,
    }
}
