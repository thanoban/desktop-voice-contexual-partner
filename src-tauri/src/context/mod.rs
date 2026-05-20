/// Returns the title of the currently focused window (not our own window).
/// Returns None if detection fails or is not supported.
pub fn get_active_window_title() -> Option<String> {
    #[cfg(target_os = "windows")]
    return windows_active_title();

    #[cfg(target_os = "macos")]
    return macos_active_title();

    #[cfg(target_os = "linux")]
    return linux_active_title();

    #[allow(unreachable_code)]
    None
}

#[cfg(target_os = "windows")]
fn windows_active_title() -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    // Safe FFI to Win32 APIs
    type HWND = *mut std::ffi::c_void;
    #[allow(dead_code)]
    type BOOL = i32;

    extern "system" {
        fn GetForegroundWindow() -> HWND;
        fn GetWindowTextW(hwnd: HWND, lpstring: *mut u16, nmaxcount: i32) -> i32;
        fn GetWindowTextLengthW(hwnd: HWND) -> i32;
    }

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return None;
        }
        let mut buf: Vec<u16> = vec![0u16; (len + 1) as usize];
        let written = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
        if written <= 0 {
            return None;
        }
        buf.truncate(written as usize);
        let title = OsString::from_wide(&buf).to_string_lossy().to_string();
        if title.is_empty() {
            None
        } else {
            Some(title)
        }
    }
}

#[cfg(target_os = "macos")]
fn macos_active_title() -> Option<String> {
    // Use osascript to get frontmost app name
    let output = std::process::Command::new("osascript")
        .args(["-e", "tell application \"System Events\" to get name of first process whose frontmost is true"])
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() { None } else { Some(name) }
}

#[cfg(target_os = "linux")]
fn linux_active_title() -> Option<String> {
    // Try xdotool first, then wmctrl
    let output = std::process::Command::new("xdotool")
        .args(["getactivewindow", "getwindowname"])
        .output()
        .ok()?;
    let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if title.is_empty() { None } else { Some(title) }
}
