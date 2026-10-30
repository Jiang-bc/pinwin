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
