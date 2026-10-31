"""Exercise the release EXE on a real Windows desktop using disposable windows."""
import ctypes as c
from ctypes import wintypes as w
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import time

u = c.WinDLL('user32', use_last_error=True)
k = c.WinDLL('kernel32', use_last_error=True)
shell = c.WinDLL('shell32', use_last_error=True)

def api(lib, name, result, *args):
    fn = getattr(lib, name)
    fn.restype, fn.argtypes = result, args
    return fn

find = api(u, 'FindWindowW', w.HWND, w.LPCWSTR, w.LPCWSTR)
post = api(u, 'PostMessageW', w.BOOL, w.HWND, w.UINT, w.WPARAM, w.LPARAM)
send = api(u, 'SendMessageW', w.LPARAM, w.HWND, w.UINT, w.WPARAM, w.LPARAM)
get_style = api(u, 'GetWindowLongW', c.c_long, w.HWND, c.c_int)
get_rect = api(u, 'GetWindowRect', w.BOOL, w.HWND, c.POINTER(w.RECT))
get_child = api(u, 'GetDlgItem', w.HWND, w.HWND, c.c_int)
set_cursor = api(u, 'SetCursorPos', w.BOOL, c.c_int, c.c_int)
ancestor = api(u, 'GetAncestor', w.HWND, w.HWND, w.UINT)
position = api(u, 'SetWindowPos', w.BOOL, w.HWND, w.HWND, c.c_int, c.c_int, c.c_int, c.c_int, w.UINT)
load = api(k, 'LoadLibraryExW', w.HMODULE, w.LPCWSTR, w.HANDLE, w.DWORD)
resource = api(k, 'FindResourceW', w.HANDLE, w.HMODULE, c.c_void_p, c.c_void_p)
load_resource = api(k, 'LoadResource', w.HANDLE, w.HMODULE, w.HANDLE)
lock_resource = api(k, 'LockResource', c.c_void_p, w.HANDLE)
size_resource = api(k, 'SizeofResource', w.DWORD, w.HMODULE, w.HANDLE)
free = api(k, 'FreeLibrary', w.BOOL, w.HMODULE)
extract = api(shell, 'ExtractIconExW', w.UINT, w.LPCWSTR, c.c_int, c.POINTER(w.HICON), c.POINTER(w.HICON), w.UINT)
destroy_icon = api(u, 'DestroyIcon', w.BOOL, w.HICON)
get_alpha = api(u, 'GetLayeredWindowAttributes', w.BOOL, w.HWND, c.POINTER(w.DWORD), c.POINTER(w.BYTE), c.POINTER(w.DWORD))
open_process = api(k, 'OpenProcess', w.HANDLE, w.DWORD, w.BOOL, w.DWORD)
close_handle = api(k, 'CloseHandle', w.BOOL, w.HANDLE)
get_resources = api(u, 'GetGuiResources', w.DWORD, w.HANDLE, w.DWORD)
u.SetProcessDpiAwarenessContext(c.c_void_p(-4))
exe = Path(__file__).resolve().parents[1] / 'target/release/PinWin.exe'
pe = exe.read_bytes()
pe_offset = struct.unpack_from('<I', pe, 0x3c)[0]
assert struct.unpack_from('<H', pe, pe_offset + 24 + 68)[0] == 2, 'GUI subsystem missing'
module = load(str(exe), None, 2)
assert module, 'Cannot inspect PE resources'
try:
    entry = resource(module, 1, 24)
    assert entry, 'Embedded manifest missing'
    text = c.string_at(lock_resource(load_resource(module, entry)), size_resource(module, entry)).decode('utf-8')
    assert 'asInvoker' in text and 'PerMonitorV2' in text and 'Common-Controls' in text
    assert resource(module, 1, 16), 'Version resource missing'
finally:
    free(module)
large, small = w.HICON(), w.HICON()
assert extract(str(exe), 0, c.byref(large), c.byref(small), 1) > 0
assert large.value and small.value
for icon in (large, small):
    destroy_icon(icon)
assert not find('PinWin_MainForm_Class', None), 'Existing PinWin instance: close it before this smoke test'

create_window = api(u, 'CreateWindowExW', w.HWND, w.DWORD, w.LPCWSTR, w.LPCWSTR,
                    w.DWORD, c.c_int, c.c_int, c.c_int, c.c_int, w.HWND, w.HMENU, w.HINSTANCE, c.c_void_p)
destroy_window = api(u, 'DestroyWindow', w.BOOL, w.HWND)
peek = api(u, 'PeekMessageW', w.BOOL, c.POINTER(w.MSG), w.HWND, w.UINT, w.UINT, w.UINT)
translate = api(u, 'TranslateMessage', w.BOOL, c.POINTER(w.MSG))
dispatch = api(u, 'DispatchMessageW', w.LPARAM, c.POINTER(w.MSG))
point_window = api(u, 'WindowFromPoint', w.HWND, w.POINT)

def pump():
    message = w.MSG()
    while peek(c.byref(message), None, 0, 0, 1):
        translate(c.byref(message))
        dispatch(c.byref(message))

# Use a real top-level class: WindowFromPoint intentionally skips STATIC text.
class WindowClass(c.Structure):
    _fields_ = [('style', w.UINT), ('procedure', c.c_void_p), ('class_extra', c.c_int),
                ('window_extra', c.c_int), ('instance', w.HINSTANCE), ('icon', w.HICON),
                ('cursor', w.HANDLE), ('background', w.HBRUSH), ('menu', w.LPCWSTR),
                ('name', w.LPCWSTR)]
register = api(u, 'RegisterClassW', w.WORD, c.POINTER(WindowClass))
cls = WindowClass()
cls.procedure = c.cast(u.DefWindowProcW, c.c_void_p).value
cls.background = 6
cls.name = 'PinWin_DisposableSmokeTarget'
assert register(c.byref(cls))
a = create_window(8, cls.name, 'PinWin disposable smoke target A', 0x10cf0000,
                  70, 80, 420, 220, None, None, None, None)
b = create_window(8, cls.name, 'PinWin disposable smoke target B', 0x10cf0000,
                  650, 80, 420, 220, None, None, None, None)
assert a and b
pump()
original = [get_style(hwnd, -20) for hwnd in (a, b)]

def wait(predicate, label, timeout=6):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        pump()
        result = predicate()
        if result:
            return result
        time.sleep(.02)
    raise AssertionError(label)

def bounds(hwnd):
    r = w.RECT()
    assert get_rect(hwnd, c.byref(r))
    return r

def command(hwnd, identifier):
    assert post(hwnd, 0x111, identifier, 0)

def pin(hwnd):
    position(hwnd, None, 0, 0, 0, 0, 0x43)
    u.SetForegroundWindow(w.HWND(hwnd))
    pump()
    r = bounds(hwnd)
    set_cursor(r.left + 120, r.top + 80)
    hit = point_window(w.POINT(r.left + 120, r.top + 80))
    name = c.create_unicode_buffer(256)
    api(u, 'GetClassNameW', c.c_int, w.HWND, w.LPWSTR, c.c_int)(hit, name, 256)
    wait(lambda: ancestor(point_window(w.POINT(r.left + 120, r.top + 80)), 2) == hwnd,
         f'Disposable target obscured: expected={hwnd:x} rect={r.left,r.top,r.right,r.bottom} actual={hit} class={name.value}')
    count = send(listbox, 0x18b, 0, 0)
    post(main, 0x312, 2, 0)
    wait(lambda: send(listbox, 0x18b, 0, 0) == count + 1, 'Hotkey did not manage target')
    assert get_style(hwnd, -20) & 8

def select_all():
    send(listbox, 0x185, 1, -1)
    post(main, 0x111, 1004 | (1 << 16), listbox)

with tempfile.TemporaryDirectory(prefix='PinWin-portable-smoke-') as folder:
    app = Path(folder) / 'PinWin.exe'
    shutil.copy2(exe, app)
    for args, status in [(['--help'], 0), (['--version'], 0), (['--print-settings'], 0), (['--unknown'], 2)]:
        result = subprocess.run([str(app), *args], capture_output=True, text=True, timeout=10)
        assert result.returncode == status, (args, result.returncode, result.stderr)
        if status == 0:
            assert result.stdout.strip(), (args, 'Missing CLI output')
    process = subprocess.Popen([str(app)])
    try:
        main = wait(lambda: find('PinWin_MainForm_Class', None), 'Main window missing')
        listbox = wait(lambda: get_child(main, 1004), 'Main controls not ready')
        pin(a)
        wait(lambda: send(listbox, 0x18b, 0, 0) == 1, 'Pinned list not updated')
        badge = wait(lambda: find('PinWin_BadgeWindow_Class', None), 'Badge missing')
        before = bounds(badge)
        assert position(a, None, 160, 220, 0, 0, 0x15)
        wait(lambda: bounds(badge).left != before.left, 'Badge did not follow movement')
        assert get_style(b, -20) == original[1], ('B before pin', hex(get_style(b, -20)))
        pin(b)
        wait(lambda: send(listbox, 0x18b, 0, 0) == 2, 'Second target missing')
        select_all()
        wait(lambda: send(listbox, 0x190, 0, 0) == 2, 'Multi-selection failed')
        command(main, 1009)
        alpha, color, flags = w.BYTE(), w.DWORD(), w.DWORD()
        wait(lambda: get_alpha(a, c.byref(color), c.byref(alpha), c.byref(flags)) and alpha.value == 204, 'Opacity change failed')
        command(main, 1011)
        wait(lambda: get_style(a, -20) & 0x20, 'Click-through failed')
        command(main, 2005)
        wait(lambda: not (get_style(a, -20) & 0x20), 'Tray ghost recovery failed')
        request = subprocess.Popen([str(app), '--unpin-all'], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        wait(lambda: request.poll() is not None, 'CLI bulk request stalled')
        output, errors = request.communicate()
        assert request.returncode == 0, (request.returncode, output, errors)
        wait(lambda: send(listbox, 0x18b, 0, 0) == 0, 'Managed bulk restore failed')
        for hwnd, style in zip((a, b), original):
            wait(lambda hwnd=hwnd, style=style: get_style(hwnd, -20) & (8 | 0x20 | 0x80000) == style & (8 | 0x20 | 0x80000), 'Bulk original effects restoration')
        # Stress sent CLI requests while WinEvent/timer synchronization is active.
        for _ in range(6):
            pin(a)
            select_all()
            command(main, 1009)
            wait(lambda: get_alpha(a, c.byref(color), c.byref(alpha), c.byref(flags)) and alpha.value == 204,
                 'Stress opacity change failed')
            request = subprocess.Popen([str(app), '--unpin-all'], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            wait(lambda: request.poll() is not None, 'Stress bulk request stalled')
            output, errors = request.communicate()
            assert request.returncode == 0, (output, errors)
            wait(lambda: send(listbox, 0x18b, 0, 0) == 0, 'Stress bulk list not cleared')
            wait(lambda: get_style(a, -20) & (8 | 0x20 | 0x80000) == original[0] & (8 | 0x20 | 0x80000),
                 'Stress original effects restoration')
        command(main, 1001)
        overlay = wait(lambda: find('PinWin_DesktopOverlay_Class', None), 'Selection overlay missing')
        target_bounds = bounds(a)
        set_cursor(target_bounds.left + 120, target_bounds.top + 80)
        post(overlay, 0x202, 0, 0)
        wait(lambda: send(listbox, 0x18b, 0, 0) == 1, 'Overlay selection did not manage target')
        command(main, 1006)
        wait(lambda: send(listbox, 0x18b, 0, 0) == 0, 'Overlay-selected target did not unpin')
        command(main, 1001)
        overlay = wait(lambda: find('PinWin_DesktopOverlay_Class', None), 'Second overlay missing')
        post(overlay, 0x100, 27, 0)
        wait(lambda: not find('PinWin_DesktopOverlay_Class', None), 'Escape cancellation failed')
        resource_process = open_process(0x400, False, process.pid)
        assert resource_process
        try:
            counts = []
            for _ in range(8):
                command(main, 1003)
                dialog = wait(lambda: find('PinWin_SettingsForm_Class', None), 'Settings reopen failed')
                wait(lambda: get_child(dialog, 3015), 'Settings controls not ready')
                command(dialog, 3015)
                wait(lambda: not find('PinWin_SettingsForm_Class', None), 'Settings cancel failed')
                # WM_DESTROY precedes the modal loop's deferred owner cleanup.
                settled = time.monotonic() + .15
                wait(lambda: time.monotonic() >= settled, 'Dialog cleanup did not settle')
                counts.append(get_resources(resource_process, 0))
            assert counts[-1] <= counts[0] + 3, ('Settings GDI resources grew', counts)
        finally:
            close_handle(resource_process)
        command(main, 1003)
        settings = wait(lambda: find('PinWin_SettingsForm_Class', None), 'Settings dialog missing')
        wait(lambda: get_child(settings, 3014), 'Settings save controls missing')
        command(settings, 3001)
        command(settings, 3002)
        command(settings, 3014)
        wait(lambda: (Path(folder) / 'settings.json').exists(), 'Portable settings file missing')
        wait(lambda: not find('PinWin_SettingsForm_Class', None), 'Settings did not save')
        data = json.loads((Path(folder) / 'settings.json').read_text(encoding='utf-8'))
        assert data['shortcut_pin_window_prompt'] == '' and data['shortcut_pin_window_under_cursor'] == ''
        pin(a)  # Exercise orderly exit restoration with a managed window still active.
        post(main, 0x10, 0, 0)
        wait(lambda: process.poll() is not None, 'Process did not exit')
        assert process.returncode == 0
        for hwnd, style in zip((a, b), original):
            assert (get_style(hwnd, -20) & (8 | 0x20 | 0x80000)) == (style & (8 | 0x20 | 0x80000)), ('Original window state not restored', hwnd, hex(style), hex(get_style(hwnd, -20)))
        assert not find('PinWin_BadgeWindow_Class', None), 'Badge left behind'
        print('PASS: PE GUI subsystem, manifest/version/icon; release CLI; GUI pinning, badge movement, multi-selection, opacity, ghost recovery, managed bulk restore, overlay cancellation, portable settings, exit restoration')
    finally:
        if process.poll() is None:
            if find('PinWin_MainForm_Class', None):
                post(find('PinWin_MainForm_Class', None), 0x10, 0, 0)
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.terminate()
                process.wait(timeout=3)
        destroy_window(a)
        destroy_window(b)
