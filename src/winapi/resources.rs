//! Exclusive ownership of resources created by Windows. Shared stock objects stay borrowed.

use super::ffi::*;

macro_rules! owned_resource {
    ($name:ident, $handle:ty, $destroy:path) => {
        #[derive(Debug)]
        pub struct $name($handle);

        impl $name {
            /// Take exclusive ownership of a non-shared handle.
            ///
            /// # Safety
            /// The handle must be live, owned by the caller and released only through this wrapper.
            pub unsafe fn from_owned(raw: $handle) -> Option<Self> {
                if raw.is_null() { None } else { Some(Self(raw)) }
            }

            pub fn as_raw(&self) -> $handle {
                self.0
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                unsafe {
                    $destroy(self.0);
                }
            }
        }
    };
}

owned_resource!(OwnedIcon, HICON, DestroyIcon);
owned_resource!(OwnedCursor, HCURSOR, DestroyCursor);
owned_resource!(OwnedBitmap, HBITMAP, DeleteObject);
owned_resource!(OwnedFont, HFONT, DeleteObject);
owned_resource!(OwnedDc, HDC, DeleteDC);
owned_resource!(OwnedMenu, HMENU, DestroyMenu);
owned_resource!(OwnedHook, HWINEVENTHOOK, UnhookWinEvent);
owned_resource!(OwnedHandle, HANDLE, CloseHandle);

pub fn load_embedded_icon(bytes: &[u8], size: i32) -> std::io::Result<OwnedIcon> {
    unsafe {
        OwnedIcon::from_owned(super::ApiResources::create_icon_from_ico_bytes(
            bytes, true, size,
        ))
    }
    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid embedded icon"))
}

pub fn load_embedded_cursor(bytes: &[u8], size: i32) -> std::io::Result<OwnedCursor> {
    use std::io::{Error, ErrorKind};
    let invalid = || Error::new(ErrorKind::InvalidData, "Invalid embedded cursor");
    if bytes.len() < 6 || bytes[..4] != [0, 0, 2, 0] || size <= 0 {
        return Err(invalid());
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let end_dir = 6 + count * 16;
    if count == 0 || end_dir > bytes.len() {
        return Err(invalid());
    }
    let mut best = None;
    for e in bytes[6..end_dir].as_chunks::<16>().0 {
        let width = if e[0] == 0 { 256 } else { e[0] as i32 };
        let height = if e[1] == 0 { 256 } else { e[1] as i32 };
        let x = u16::from_le_bytes(e[4..6].try_into().unwrap());
        let y = u16::from_le_bytes(e[6..8].try_into().unwrap());
        let length = u32::from_le_bytes(e[8..12].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(e[12..16].try_into().unwrap()) as usize;
        let end = offset.checked_add(length).ok_or_else(invalid)?;
        if x as i32 >= width
            || y as i32 >= height
            || length < 40
            || offset < end_dir
            || end > bytes.len()
        {
            return Err(invalid());
        }
        if best
            .as_ref()
            .is_none_or(|(distance, _, _, _, _)| (width - size).abs() < *distance)
        {
            best = Some(((width - size).abs(), offset, end, x, y));
        }
    }
    let (_, offset, end, x, y) = best.ok_or_else(invalid)?;
    // RT_CURSOR resource data starts with hotspot words, unlike a CUR directory frame.
    let mut data = Vec::with_capacity(end - offset + 4);
    data.extend_from_slice(&x.to_le_bytes());
    data.extend_from_slice(&y.to_le_bytes());
    data.extend_from_slice(&bytes[offset..end]);
    let raw = unsafe {
        CreateIconFromResourceEx(
            data.as_ptr(),
            data.len() as u32,
            FALSE,
            0x0003_0000,
            size,
            size,
            0,
        )
    };
    unsafe { OwnedCursor::from_owned(raw) }.ok_or_else(std::io::Error::last_os_error)
}
