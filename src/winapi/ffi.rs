//! Raw Win32 FFI bindings (`user32`, `gdi32`, `dwmapi`, `shell32`, `comdlg32`, `kernel32`).
//!
//! Uses standard Win32 C ABI (`extern "system"`) so zero external crate dependencies are needed.

#![allow(
    non_snake_case,
    non_camel_case_types,
    clippy::upper_case_acronyms,
    dead_code
)]

use std::ffi::c_void;

pub type HWND = isize;
pub type HINSTANCE = isize;
pub type HMODULE = isize;
pub type HDC = isize;
pub type HGDIOBJ = isize;
pub type HBITMAP = isize;
pub type HICON = isize;
pub type HCURSOR = isize;
pub type HBRUSH = isize;
pub type HPEN = isize;
pub type HFONT = isize;
pub type HMENU = isize;
pub type HANDLE = isize;
pub type HWINEVENTHOOK = isize;
pub type ATOM = u16;
pub type BOOL = i32;
pub type DWORD = u32;
pub type UINT = u32;
pub type WORD = u16;
pub type BYTE = u8;
pub type LONG = i32;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type COLORREF = u32;
pub type ULONG_PTR = usize;
pub type LONG_PTR = isize;
pub type UINT_PTR = usize;

pub const FALSE: BOOL = 0;
pub const TRUE: BOOL = 1;

// Special HWND values
pub const HWND_TOP: HWND = 0;
pub const HWND_BOTTOM: HWND = 1;
pub const HWND_TOPMOST: HWND = -1;
pub const HWND_NOTOPMOST: HWND = -2;
pub const HWND_BROADCAST: HWND = 0xFFFF;

// SetWindowPos flags
pub const SWP_NOSIZE: UINT = 0x0001;
pub const SWP_NOMOVE: UINT = 0x0002;
pub const SWP_NOZORDER: UINT = 0x0004;
pub const SWP_NOREDRAW: UINT = 0x0008;
pub const SWP_NOACTIVATE: UINT = 0x0010;
pub const SWP_FRAMECHANGED: UINT = 0x0020;
pub const SWP_SHOWWINDOW: UINT = 0x0040;
pub const SWP_HIDEWINDOW: UINT = 0x0080;
pub const SWP_NOOWNERZORDER: UINT = 0x0200;
pub const SWP_ASYNCWINDOWPOS: UINT = 0x4000;

// Window Long indices
pub const GWL_WNDPROC: i32 = -4;
pub const GWLP_HINSTANCE: i32 = -6;
pub const GWLP_HWNDPARENT: i32 = -8;
pub const GWL_STYLE: i32 = -16;
pub const GWL_EXSTYLE: i32 = -20;
pub const GWLP_USERDATA: i32 = -21;

// Window Styles
pub const WS_OVERLAPPED: DWORD = 0x0000_0000;
pub const WS_POPUP: DWORD = 0x8000_0000;
pub const WS_CHILD: DWORD = 0x4000_0000;
pub const WS_MINIMIZE: DWORD = 0x2000_0000;
pub const WS_VISIBLE: DWORD = 0x1000_0000;
pub const WS_DISABLED: DWORD = 0x0800_0000;
pub const WS_CLIPSIBLINGS: DWORD = 0x0400_0000;
pub const WS_CLIPCHILDREN: DWORD = 0x0200_0000;
pub const WS_MAXIMIZE: DWORD = 0x0100_0000;
pub const WS_CAPTION: DWORD = 0x00C0_0000;
pub const WS_BORDER: DWORD = 0x0080_0000;
pub const WS_DLGFRAME: DWORD = 0x0040_0000;
pub const WS_VSCROLL: DWORD = 0x0020_0000;
pub const WS_HSCROLL: DWORD = 0x0010_0000;
pub const WS_SYSMENU: DWORD = 0x0008_0000;
pub const WS_THICKFRAME: DWORD = 0x0004_0000;
pub const WS_GROUP: DWORD = 0x0002_0000;
pub const WS_TABSTOP: DWORD = 0x0001_0000;
pub const WS_MINIMIZEBOX: DWORD = 0x0002_0000;
pub const WS_MAXIMIZEBOX: DWORD = 0x0001_0000;
pub const WS_OVERLAPPEDWINDOW: DWORD =
    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;

// Extended Window Styles
pub const WS_EX_DLGMODALFRAME: DWORD = 0x0000_0001;
pub const WS_EX_NOPARENTNOTIFY: DWORD = 0x0000_0004;
pub const WS_EX_TOPMOST: DWORD = 0x0000_0008;
pub const WS_EX_TRANSPARENT: DWORD = 0x0000_0020;
pub const WS_EX_TOOLWINDOW: DWORD = 0x0000_0080;
pub const WS_EX_WINDOWEDGE: DWORD = 0x0000_0100;
pub const WS_EX_CLIENTEDGE: DWORD = 0x0000_0200;
pub const WS_EX_APPWINDOW: DWORD = 0x0004_0000;
pub const WS_EX_LAYERED: DWORD = 0x0008_0000;
pub const WS_EX_COMPOSITED: DWORD = 0x0200_0000;
pub const WS_EX_NOACTIVATE: DWORD = 0x0800_0000;

// Layered Window flags
pub const LWA_COLORKEY: DWORD = 0x0000_0001;
pub const LWA_ALPHA: DWORD = 0x0000_0002;

// ShowWindow commands
pub const SW_HIDE: i32 = 0;
pub const SW_SHOWNORMAL: i32 = 1;
pub const SW_SHOWMINIMIZED: i32 = 2;
pub const SW_SHOWMAXIMIZED: i32 = 3;
pub const SW_SHOWNOACTIVATE: i32 = 4;
pub const SW_SHOW: i32 = 5;
pub const SW_MINIMIZE: i32 = 6;
pub const SW_RESTORE: i32 = 9;

// Window Messages
pub const WM_NULL: UINT = 0x0000;
pub const WM_CREATE: UINT = 0x0001;
pub const WM_DESTROY: UINT = 0x0002;
pub const WM_MOVE: UINT = 0x0003;
pub const WM_SIZE: UINT = 0x0005;
pub const WM_ACTIVATE: UINT = 0x0006;
pub const WM_SETFOCUS: UINT = 0x0007;
pub const WM_KILLFOCUS: UINT = 0x0008;
pub const WM_ENABLE: UINT = 0x000A;
pub const WM_SETTEXT: UINT = 0x000C;
pub const WM_GETTEXT: UINT = 0x000D;
pub const WM_GETTEXTLENGTH: UINT = 0x000E;
pub const WM_PAINT: UINT = 0x000F;
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_QUIT: UINT = 0x0012;
pub const WM_ERASEBKGND: UINT = 0x0014;
pub const WM_SHOWWINDOW: UINT = 0x0018;
pub const WM_SETCURSOR: UINT = 0x0020;
pub const WM_MOUSEACTIVATE: UINT = 0x0021;
pub const WM_GETMINMAXINFO: UINT = 0x0024;
pub const WM_SETFONT: UINT = 0x0030;
pub const WM_NCDESTROY: UINT = 0x0082;
pub const WM_NCHITTEST: UINT = 0x0084;
pub const WM_KEYDOWN: UINT = 0x0100;
pub const WM_KEYUP: UINT = 0x0101;
pub const WM_CHAR: UINT = 0x0102;
pub const WM_SYSKEYDOWN: UINT = 0x0104;
pub const WM_SYSKEYUP: UINT = 0x0105;
pub const WM_COMMAND: UINT = 0x0111;
pub const WM_SYSCOMMAND: UINT = 0x0112;
pub const WM_TIMER: UINT = 0x0113;
pub const WM_CTLCOLORSTATIC: UINT = 0x0138;
pub const WM_MOUSEMOVE: UINT = 0x0200;
pub const WM_LBUTTONDOWN: UINT = 0x0201;
pub const WM_LBUTTONUP: UINT = 0x0202;
pub const WM_LBUTTONDBLCLK: UINT = 0x0203;
pub const WM_RBUTTONDOWN: UINT = 0x0204;
pub const WM_RBUTTONUP: UINT = 0x0205;
pub const WM_MBUTTONUP: UINT = 0x0208;
pub const WM_MOUSEWHEEL: UINT = 0x020A;
pub const WM_HOTKEY: UINT = 0x0312;
pub const WM_USER: UINT = 0x0400;
pub const WM_APP: UINT = 0x8000;

pub const SIZE_MINIMIZED: WPARAM = 1;
pub const MA_NOACTIVATE: LRESULT = 3;

// Button / Listbox / Edit control styles and messages
pub const BS_PUSHBUTTON: DWORD = 0x0000_0000;
pub const BS_DEFPUSHBUTTON: DWORD = 0x0000_0001;
pub const BS_AUTOCHECKBOX: DWORD = 0x0000_0003;
pub const BS_GROUPBOX: DWORD = 0x0000_0007;
pub const BM_GETCHECK: UINT = 0x00F0;
pub const BM_SETCHECK: UINT = 0x00F1;
pub const BST_UNCHECKED: WPARAM = 0;
pub const BST_CHECKED: WPARAM = 1;
pub const BN_CLICKED: WORD = 0;

pub const ES_AUTOHSCROLL: DWORD = 0x0080;
pub const ES_READONLY: DWORD = 0x0800;
pub const EN_CHANGE: WORD = 0x0300;

pub const SS_LEFT: DWORD = 0x0000_0000;
pub const SS_CENTER: DWORD = 0x0000_0001;
pub const SS_ICON: DWORD = 0x0000_0003;
pub const SS_NOTIFY: DWORD = 0x0000_0100;
pub const STM_SETICON: UINT = 0x0170;

pub const LBS_NOTIFY: DWORD = 0x0001;
pub const LBS_MULTIPLESEL: DWORD = 0x0008;
pub const LBS_NOINTEGRALHEIGHT: DWORD = 0x0100;
pub const LBS_EXTENDEDSEL: DWORD = 0x0800;
pub const LB_ADDSTRING: UINT = 0x0180;
pub const LB_DELETESTRING: UINT = 0x0182;
pub const LB_RESETCONTENT: UINT = 0x0184;
pub const LB_SETSEL: UINT = 0x0185;
pub const LB_SETCURSEL: UINT = 0x0186;
pub const LB_GETSEL: UINT = 0x0187;
pub const LB_GETCURSEL: UINT = 0x0188;
pub const LB_GETCOUNT: UINT = 0x018B;
pub const LB_GETSELCOUNT: UINT = 0x0190;
pub const LB_GETSELITEMS: UINT = 0x0191;
pub const LB_SETITEMDATA: UINT = 0x019A;
pub const LB_GETITEMDATA: UINT = 0x0199;
pub const LBN_SELCHANGE: WORD = 1;
pub const LBN_DBLCLK: WORD = 2;

// SendMessageTimeout flags
pub const SMTO_NORMAL: UINT = 0x0000;
pub const SMTO_BLOCK: UINT = 0x0001;
pub const SMTO_ABORTIFHUNG: UINT = 0x0002;

// GetAncestor / GetWindow flags
pub const GA_PARENT: UINT = 1;
pub const GA_ROOT: UINT = 2;
pub const GA_ROOTOWNER: UINT = 3;
pub const GW_OWNER: UINT = 4;

// System Metrics
pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;
pub const SM_CXICON: i32 = 11;
pub const SM_CYICON: i32 = 12;
pub const SM_CXSIZE: i32 = 30;
pub const SM_CYSIZE: i32 = 31;
pub const SM_CXSMICON: i32 = 49;
pub const SM_CYSMICON: i32 = 50;
pub const SM_XVIRTUALSCREEN: i32 = 76;
pub const SM_YVIRTUALSCREEN: i32 = 77;
pub const SM_CXVIRTUALSCREEN: i32 = 78;
pub const SM_CYVIRTUALSCREEN: i32 = 79;

// DWM Attributes
pub const DWMWA_CAPTION_BUTTON_BOUNDS: DWORD = 5;
pub const DWMWA_EXTENDED_FRAME_BOUNDS: DWORD = 9;
pub const DWMWA_CLOAKED: DWORD = 14;

// WinEventHook constants
pub const WINEVENT_OUTOFCONTEXT: DWORD = 0x0000;
pub const WINEVENT_SKIPOWNPROCESS: DWORD = 0x0002;
pub const EVENT_SYSTEM_FOREGROUND: DWORD = 0x0003;
pub const EVENT_SYSTEM_MOVESIZEEND: DWORD = 0x000B;
pub const EVENT_SYSTEM_MINIMIZESTART: DWORD = 0x0016;
pub const EVENT_SYSTEM_MINIMIZEEND: DWORD = 0x0017;
pub const EVENT_OBJECT_DESTROY: DWORD = 0x8001;
pub const EVENT_OBJECT_SHOW: DWORD = 0x8002;
pub const EVENT_OBJECT_HIDE: DWORD = 0x8003;
pub const EVENT_OBJECT_LOCATIONCHANGE: DWORD = 0x800B;
pub const EVENT_OBJECT_NAMECHANGE: DWORD = 0x800C;
pub const OBJID_WINDOW: LONG = 0;

// GDI constants
pub const SRCCOPY: DWORD = 0x00CC_0020;
pub const CAPTUREBLT: DWORD = 0x4000_0000;
pub const TRANSPARENT: i32 = 1;
pub const OPAQUE: i32 = 2;
pub const PS_SOLID: i32 = 0;
pub const NULL_BRUSH: i32 = 5;
pub const DEFAULT_GUI_FONT: i32 = 17;
pub const COLOR_WINDOW: i32 = 5;
pub const COLOR_BTNFACE: i32 = 15;
pub const DT_LEFT: UINT = 0x0000;
pub const DT_CENTER: UINT = 0x0001;
pub const DT_VCENTER: UINT = 0x0004;
pub const DT_SINGLELINE: UINT = 0x0020;
pub const DT_NOPREFIX: UINT = 0x0800;
pub const DT_END_ELLIPSIS: UINT = 0x8000;
pub const BI_RGB: DWORD = 0;
pub const DIB_RGB_COLORS: UINT = 0;

// Cursors & Icons
pub const IDC_ARROW: *const u16 = 32512 as *const u16;
pub const IDC_CROSS: *const u16 = 32515 as *const u16;
pub const IDC_HAND: *const u16 = 32649 as *const u16;
pub const IDI_APPLICATION: *const u16 = 32512 as *const u16;
pub const IMAGE_BITMAP: UINT = 0;
pub const IMAGE_ICON: UINT = 1;
pub const IMAGE_CURSOR: UINT = 2;
pub const LR_LOADFROMFILE: UINT = 0x0010;
pub const LR_DEFAULTSIZE: UINT = 0x0040;
pub const DI_NORMAL: UINT = 0x0003;

// Menus
pub const MF_STRING: UINT = 0x0000;
pub const MF_CHECKED: UINT = 0x0008;
pub const MF_POPUP: UINT = 0x0010;
pub const MF_SEPARATOR: UINT = 0x0800;
pub const TPM_LEFTALIGN: UINT = 0x0000;
pub const TPM_RIGHTBUTTON: UINT = 0x0002;
pub const TPM_BOTTOMALIGN: UINT = 0x0020;
pub const TPM_RETURNCMD: UINT = 0x0100;

// MessageBox
pub const MB_OK: UINT = 0x0000;
pub const MB_ICONINFORMATION: UINT = 0x0040;
pub const MB_ICONWARNING: UINT = 0x0030;
pub const MB_ICONERROR: UINT = 0x0010;

// Mutex & Sync
pub const WAIT_OBJECT_0: DWORD = 0x0000_0000;
pub const WAIT_ABANDONED: DWORD = 0x0000_0080;
pub const WAIT_TIMEOUT: DWORD = 0x0000_0102;
pub const INFINITE: DWORD = 0xFFFF_FFFF;

// NotifyIcon
pub const NIM_ADD: DWORD = 0x0000_0000;
pub const NIM_MODIFY: DWORD = 0x0000_0001;
pub const NIM_DELETE: DWORD = 0x0000_0002;
pub const NIF_MESSAGE: UINT = 0x0000_0001;
pub const NIF_ICON: UINT = 0x0000_0002;
pub const NIF_TIP: UINT = 0x0000_0004;
pub const NIF_INFO: UINT = 0x0000_0010;
pub const NIIF_INFO: DWORD = 0x0000_0001;

// OpenFileName flags
pub const OFN_FILEMUSTEXIST: DWORD = 0x0000_1000;
pub const OFN_PATHMUSTEXIST: DWORD = 0x0000_0800;
pub const OFN_NOCHANGEDIR: DWORD = 0x0000_0008;
pub const OFN_EXPLORER: DWORD = 0x0008_0000;

// DPI Awareness Context
pub const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

pub const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub const fn loword(l: usize) -> WORD {
    (l & 0xFFFF) as WORD
}

pub const fn hiword(l: usize) -> WORD {
    ((l >> 16) & 0xFFFF) as WORD
}

pub const fn get_x_lparam(lp: LPARAM) -> i32 {
    (lp as i16) as i32
}

pub const fn get_y_lparam(lp: LPARAM) -> i32 {
    ((lp >> 16) as i16) as i32
}

pub const fn get_wheel_delta_wparam(wp: WPARAM) -> i16 {
    ((wp >> 16) & 0xFFFF) as i16
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct POINT {
    pub x: LONG,
    pub y: LONG,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RECT {
    pub left: LONG,
    pub top: LONG,
    pub right: LONG,
    pub bottom: LONG,
}

impl RECT {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn width(&self) -> i32 {
        self.right - self.left
    }

    pub const fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
}

pub type WNDPROC = Option<
    unsafe extern "system" fn(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT,
>;

pub type WINEVENTPROC = Option<
    unsafe extern "system" fn(
        h_win_event_hook: HWINEVENTHOOK,
        event: DWORD,
        hwnd: HWND,
        id_object: LONG,
        id_child: LONG,
        id_event_thread: DWORD,
        dwms_event_time: DWORD,
    ),
>;

pub type WNDENUMPROC = Option<unsafe extern "system" fn(hwnd: HWND, lparam: LPARAM) -> BOOL>;

#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: UINT,
    pub style: UINT,
    pub lpfnWndProc: WNDPROC,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: HCURSOR,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
    pub hIconSm: HICON,
}

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: BOOL,
    pub rcPaint: RECT,
    pub fRestore: BOOL,
    pub fIncUpdate: BOOL,
    pub rgbReserved: [BYTE; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BITMAPINFOHEADER {
    pub biSize: DWORD,
    pub biWidth: LONG,
    pub biHeight: LONG,
    pub biPlanes: WORD,
    pub biBitCount: WORD,
    pub biCompression: DWORD,
    pub biSizeImage: DWORD,
    pub biXPelsPerMeter: LONG,
    pub biYPelsPerMeter: LONG,
    pub biClrUsed: DWORD,
    pub biClrImportant: DWORD,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RGBQUAD {
    pub rgbBlue: BYTE,
    pub rgbGreen: BYTE,
    pub rgbRed: BYTE,
    pub rgbReserved: BYTE,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct BITMAPINFO {
    pub bmiHeader: BITMAPINFOHEADER,
    pub bmiColors: [RGBQUAD; 1],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[repr(C)]
pub struct NOTIFYICONDATAW {
    pub cbSize: DWORD,
    pub hWnd: HWND,
    pub uID: UINT,
    pub uFlags: UINT,
    pub uCallbackMessage: UINT,
    pub hIcon: HICON,
    pub szTip: [u16; 128],
    pub dwState: DWORD,
    pub dwStateMask: DWORD,
    pub szInfo: [u16; 256],
    pub uTimeoutOrVersion: UINT,
    pub szInfoTitle: [u16; 64],
    pub dwInfoFlags: DWORD,
    pub guidItem: GUID,
    pub hBalloonIcon: HICON,
}

#[repr(C)]
pub struct OPENFILENAMEW {
    pub lStructSize: DWORD,
    pub hwndOwner: HWND,
    pub hInstance: HINSTANCE,
    pub lpstrFilter: *const u16,
    pub lpstrCustomFilter: *mut u16,
    pub nMaxCustFilter: DWORD,
    pub nFilterIndex: DWORD,
    pub lpstrFile: *mut u16,
    pub nMaxFile: DWORD,
    pub lpstrFileTitle: *mut u16,
    pub nMaxFileTitle: DWORD,
    pub lpstrInitialDir: *const u16,
    pub lpstrTitle: *const u16,
    pub Flags: DWORD,
    pub nFileOffset: WORD,
    pub nFileExtension: WORD,
    pub lpstrDefExt: *const u16,
    pub lCustData: LPARAM,
    pub lpfnHook: *const c_void,
    pub lpTemplateName: *const u16,
    pub pvReserved: *mut c_void,
    pub dwReserved: DWORD,
    pub FlagsEx: DWORD,
}

#[link(name = "user32")]
extern "system" {
    pub fn SetProcessDPIAware() -> BOOL;
    pub fn GetSystemMetrics(nIndex: i32) -> i32;

    pub fn RegisterClassExW(lpwcx: *const WNDCLASSEXW) -> ATOM;
    pub fn CreateWindowExW(
        dwExStyle: DWORD,
        lpClassName: *const u16,
        lpWindowName: *const u16,
        dwStyle: DWORD,
        X: i32,
        Y: i32,
        nWidth: i32,
        nHeight: i32,
        hWndParent: HWND,
        hMenu: HMENU,
        hInstance: HINSTANCE,
        lpParam: *const c_void,
    ) -> HWND;
    pub fn DestroyWindow(hWnd: HWND) -> BOOL;
    pub fn DefWindowProcW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    pub fn UpdateWindow(hWnd: HWND) -> BOOL;
    pub fn MoveWindow(
        hWnd: HWND,
        X: i32,
        Y: i32,
        nWidth: i32,
        nHeight: i32,
        bRepaint: BOOL,
    ) -> BOOL;
    pub fn SetWindowPos(
        hWnd: HWND,
        hWndInsertAfter: HWND,
        X: i32,
        Y: i32,
        cx: i32,
        cy: i32,
        uFlags: UINT,
    ) -> BOOL;
    pub fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn InvalidateRect(hWnd: HWND, lpRect: *const RECT, bErase: BOOL) -> BOOL;

    pub fn IsWindow(hWnd: HWND) -> BOOL;
    pub fn IsIconic(hWnd: HWND) -> BOOL;
    pub fn IsWindowVisible(hWnd: HWND) -> BOOL;
    pub fn SetForegroundWindow(hWnd: HWND) -> BOOL;
    pub fn GetForegroundWindow() -> HWND;
    pub fn GetDesktopWindow() -> HWND;

    pub fn GetParent(hWnd: HWND) -> HWND;
    pub fn GetAncestor(hWnd: HWND, gaFlags: UINT) -> HWND;
    pub fn GetWindow(hWnd: HWND, uCmd: UINT) -> HWND;
    pub fn WindowFromPoint(Point: POINT) -> HWND;
    pub fn EnumWindows(lpEnumFunc: WNDENUMPROC, lParam: LPARAM) -> BOOL;

    pub fn GetWindowLongW(hWnd: HWND, nIndex: i32) -> LONG;
    pub fn SetWindowLongW(hWnd: HWND, nIndex: i32, dwNewLong: LONG) -> LONG;
    pub fn GetWindowLongPtrW(hWnd: HWND, nIndex: i32) -> LONG_PTR;
    pub fn SetWindowLongPtrW(hWnd: HWND, nIndex: i32, dwNewLong: LONG_PTR) -> LONG_PTR;

    pub fn SetLayeredWindowAttributes(
        hwnd: HWND,
        crKey: COLORREF,
        bAlpha: BYTE,
        dwFlags: DWORD,
    ) -> BOOL;
    pub fn GetLayeredWindowAttributes(
        hwnd: HWND,
        pcrKey: *mut COLORREF,
        pbAlpha: *mut BYTE,
        pdwFlags: *mut DWORD,
    ) -> BOOL;

    pub fn SendMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn SendMessageTimeoutW(
        hWnd: HWND,
        Msg: UINT,
        wParam: WPARAM,
        lParam: LPARAM,
        fuFlags: UINT,
        uTimeout: UINT,
        lpdwResult: *mut usize,
    ) -> LRESULT;
    pub fn PostMessageW(hWnd: HWND, Msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    pub fn PostQuitMessage(nExitCode: i32);
    pub fn GetMessageW(
        lpMsg: *mut MSG,
        hWnd: HWND,
        wMsgFilterMin: UINT,
        wMsgFilterMax: UINT,
    ) -> BOOL;
    pub fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;

    pub fn SetWindowTextW(hWnd: HWND, lpString: *const u16) -> BOOL;
    pub fn GetWindowTextW(hWnd: HWND, lpString: *mut u16, nMaxCount: i32) -> i32;
    pub fn GetWindowTextLengthW(hWnd: HWND) -> i32;
    pub fn GetClassNameW(hWnd: HWND, lpClassName: *mut u16, nMaxCount: i32) -> i32;
    pub fn GetWindowThreadProcessId(hWnd: HWND, lpdwProcessId: *mut DWORD) -> DWORD;
    pub fn EnableWindow(hWnd: HWND, bEnable: BOOL) -> BOOL;
    pub fn SetFocus(hWnd: HWND) -> HWND;

    pub fn RegisterHotKey(hWnd: HWND, id: i32, fsModifiers: UINT, vk: UINT) -> BOOL;
    pub fn UnregisterHotKey(hWnd: HWND, id: i32) -> BOOL;
    pub fn GetAsyncKeyState(vKey: i32) -> i16;

    pub fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
    pub fn SetCursor(hCursor: HCURSOR) -> HCURSOR;
    pub fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> HCURSOR;
    pub fn LoadCursorFromFileW(lpFileName: *const u16) -> HCURSOR;
    pub fn LoadIconW(hInstance: HINSTANCE, lpIconName: *const u16) -> HICON;
    pub fn LoadImageW(
        hInst: HINSTANCE,
        name: *const u16,
        type_: UINT,
        cx: i32,
        cy: i32,
        fuLoad: UINT,
    ) -> HANDLE;
    pub fn LookupIconIdFromDirectoryEx(
        presbits: *const BYTE,
        fIcon: BOOL,
        cxDesired: i32,
        cyDesired: i32,
        Flags: UINT,
    ) -> i32;
    pub fn CreateIconFromResourceEx(
        presbits: *const BYTE,
        dwResSize: DWORD,
        fIcon: BOOL,
        dwVer: DWORD,
        cxDesired: i32,
        desired_cy: i32,
        Flags: UINT,
    ) -> HICON;
    pub fn DestroyIcon(hIcon: HICON) -> BOOL;
    pub fn DrawIconEx(
        hdc: HDC,
        xLeft: i32,
        yTop: i32,
        hIcon: HICON,
        cxWidth: i32,
        cyWidth: i32,
        istepIfAniCur: UINT,
        hbrFlickerFreeDraw: HBRUSH,
        diFlags: UINT,
    ) -> BOOL;

    pub fn SetTimer(
        hWnd: HWND,
        nIDEvent: UINT_PTR,
        uElapse: UINT,
        lpTimerFunc: *const c_void,
    ) -> UINT_PTR;
    pub fn KillTimer(hWnd: HWND, uIDEvent: UINT_PTR) -> BOOL;

    pub fn SetWinEventHook(
        eventMin: DWORD,
        eventMax: DWORD,
        hmodWinEventProc: HMODULE,
        pfnWinEventProc: WINEVENTPROC,
        idProcess: DWORD,
        idThread: DWORD,
        dwFlags: DWORD,
    ) -> HWINEVENTHOOK;
    pub fn UnhookWinEvent(hWinEventHook: HWINEVENTHOOK) -> BOOL;

    pub fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    pub fn GetDC(hWnd: HWND) -> HDC;
    pub fn GetWindowDC(hWnd: HWND) -> HDC;
    pub fn ReleaseDC(hWnd: HWND, hDC: HDC) -> i32;
    pub fn FillRect(hDC: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
    pub fn FrameRect(hDC: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
    pub fn DrawTextW(
        hdc: HDC,
        lpchText: *const u16,
        cchText: i32,
        lprc: *mut RECT,
        format: UINT,
    ) -> i32;

    pub fn CreatePopupMenu() -> HMENU;
    pub fn AppendMenuW(
        hMenu: HMENU,
        uFlags: UINT,
        uIDNewItem: UINT_PTR,
        lpNewItem: *const u16,
    ) -> BOOL;
    pub fn SetMenuDefaultItem(hMenu: HMENU, uItem: UINT, fByPos: UINT) -> BOOL;
    pub fn TrackPopupMenu(
        hMenu: HMENU,
        uFlags: UINT,
        x: i32,
        y: i32,
        nReserved: i32,
        hWnd: HWND,
        prcRect: *const RECT,
    ) -> BOOL;
    pub fn DestroyMenu(hMenu: HMENU) -> BOOL;
    pub fn MessageBoxW(hWnd: HWND, lpText: *const u16, lpCaption: *const u16, uType: UINT) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
    pub fn DeleteDC(hdc: HDC) -> BOOL;
    pub fn CreateCompatibleBitmap(hdc: HDC, cx: i32, cy: i32) -> HBITMAP;
    pub fn SelectObject(hdc: HDC, h: HGDIOBJ) -> HGDIOBJ;
    pub fn DeleteObject(ho: HGDIOBJ) -> BOOL;
    pub fn BitBlt(
        hdc: HDC,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        hdcSrc: HDC,
        x1: i32,
        y1: i32,
        rop: DWORD,
    ) -> BOOL;
    pub fn CreateSolidBrush(color: COLORREF) -> HBRUSH;
    pub fn CreatePen(iStyle: i32, cWidth: i32, color: COLORREF) -> HPEN;
    pub fn GetStockObject(i: i32) -> HGDIOBJ;
    pub fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    pub fn SetBkColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn SetTextColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn CreateFontW(
        cHeight: i32,
        cWidth: i32,
        cEscapement: i32,
        cOrientation: i32,
        cWeight: i32,
        bItalic: DWORD,
        bUnderline: DWORD,
        bStrikeOut: DWORD,
        iCharSet: DWORD,
        iOutPrecision: DWORD,
        iClipPrecision: DWORD,
        iQuality: DWORD,
        iPitchAndFamily: DWORD,
        pszFaceName: *const u16,
    ) -> HFONT;
    pub fn CreateDIBSection(
        hdc: HDC,
        pbmi: *const BITMAPINFO,
        usage: UINT,
        ppvBits: *mut *mut c_void,
        hSection: HANDLE,
        offset: DWORD,
    ) -> HBITMAP;
    pub fn GetPixel(hdc: HDC, x: i32, y: i32) -> COLORREF;
    pub fn SetPixel(hdc: HDC, x: i32, y: i32, color: COLORREF) -> COLORREF;
}

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(dwMessage: DWORD, lpData: *const NOTIFYICONDATAW) -> BOOL;
}

#[link(name = "comdlg32")]
extern "system" {
    pub fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> BOOL;
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> HMODULE;
    pub fn LoadLibraryW(lpLibFileName: *const u16) -> HMODULE;
    pub fn GetProcAddress(hModule: HMODULE, lpProcName: *const u8) -> *const c_void;
    pub fn GetLastError() -> DWORD;
    pub fn CreateMutexW(
        lpMutexAttributes: *const c_void,
        bInitialOwner: BOOL,
        lpName: *const u16,
    ) -> HANDLE;
    pub fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: DWORD) -> DWORD;
    pub fn ReleaseMutex(hMutex: HANDLE) -> BOOL;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
}

/// Dynamically call `SetProcessDpiAwarenessContext` from `user32.dll` if available on this OS.
///
/// # Safety
/// Calls raw Win32 dynamic symbol resolution (`GetModuleHandleW` + `GetProcAddress`).
pub unsafe fn SetProcessDpiAwarenessContext(value: isize) -> BOOL {
    type FnType = unsafe extern "system" fn(isize) -> BOOL;
    let mod_name = [
        b'u' as u16,
        b's' as u16,
        b'e' as u16,
        b'r' as u16,
        b'3' as u16,
        b'2' as u16,
        0,
    ];
    let h_mod = GetModuleHandleW(mod_name.as_ptr());
    if h_mod != 0 {
        let proc = GetProcAddress(h_mod, c"SetProcessDpiAwarenessContext".as_ptr().cast());
        if !proc.is_null() {
            let f: FnType = std::mem::transmute(proc);
            return f(value);
        }
    }
    FALSE
}

/// Dynamically call `DwmGetWindowAttribute` from `dwmapi.dll`.
///
/// # Safety
/// Caller must ensure `pvAttribute` points to a valid buffer of size `cbAttribute`.
pub unsafe fn DwmGetWindowAttribute(
    hwnd: HWND,
    dwAttribute: DWORD,
    pvAttribute: *mut c_void,
    cbAttribute: DWORD,
) -> i32 {
    type FnType = unsafe extern "system" fn(HWND, DWORD, *mut c_void, DWORD) -> i32;
    let mod_name = [
        b'd' as u16,
        b'w' as u16,
        b'm' as u16,
        b'a' as u16,
        b'p' as u16,
        b'i' as u16,
        b'.' as u16,
        b'd' as u16,
        b'l' as u16,
        b'l' as u16,
        0,
    ];
    let mut h_mod = GetModuleHandleW(mod_name.as_ptr());
    if h_mod == 0 {
        h_mod = LoadLibraryW(mod_name.as_ptr());
    }
    if h_mod != 0 {
        let proc = GetProcAddress(h_mod, c"DwmGetWindowAttribute".as_ptr().cast());
        if !proc.is_null() {
            let f: FnType = std::mem::transmute(proc);
            return f(hwnd, dwAttribute, pvAttribute, cbAttribute);
        }
    }
    -1
}
