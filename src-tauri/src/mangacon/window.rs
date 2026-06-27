use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaConWindow {
    pub hwnd: isize,
    pub title: String,
}

fn is_mangacon_window_title(title: &str) -> bool {
    title.starts_with("漫画控") || title.starts_with("MangaCon")
}

fn is_mangacon_process_path(path: &str) -> bool {
    Path::new(path).file_name() == Some(std::ffi::OsStr::new("MangaCon.exe"))
}

#[cfg(not(windows))]
pub fn find_mangacon_windows() -> Vec<MangaConWindow> {
    Vec::new()
}

#[cfg(windows)]
pub fn find_mangacon_windows() -> Vec<MangaConWindow> {
    use windows::core::BOOL;
    use windows::Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
        },
    };

    unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return BOOL(1);
        }

        let title_length = unsafe { GetWindowTextLengthW(hwnd) };
        if title_length <= 0 {
            return BOOL(1);
        }

        let mut buffer = vec![0_u16; title_length as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
        if copied <= 0 {
            return BOOL(1);
        }

        let title = String::from_utf16_lossy(&buffer[..copied as usize]);
        let process_path = unsafe { window_process_image_path(hwnd) };
        let is_mangacon_window = process_path
            .as_deref()
            .map(is_mangacon_process_path)
            .unwrap_or_else(|| is_mangacon_window_title(&title));

        if is_mangacon_window {
            let windows = unsafe { &mut *(lparam.0 as *mut Vec<MangaConWindow>) };
            windows.push(MangaConWindow {
                hwnd: hwnd.0 as isize,
                title,
            });
        }

        BOOL(1)
    }

    let mut windows = Vec::new();
    let lparam = LPARAM(&mut windows as *mut Vec<MangaConWindow> as isize);
    unsafe {
        let _ = EnumWindows(Some(enum_window), lparam);
    }
    windows
}

#[cfg(windows)]
unsafe fn window_process_image_path(hwnd: windows::Win32::Foundation::HWND) -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::GetWindowThreadProcessId,
    };

    let mut process_id = 0_u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    if process_id == 0 {
        return None;
    }

    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) };
    let Ok(process) = process else {
        return None;
    };
    if process.is_invalid() {
        return None;
    }

    let mut buffer = vec![0_u16; 32768];
    let mut size = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
    };
    unsafe {
        let _ = CloseHandle(process);
    }

    result
        .ok()
        .map(|_| String::from_utf16_lossy(&buffer[..size as usize]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mangacon_window_title_requires_product_prefix() {
        assert!(is_mangacon_window_title("漫画控 v3.0.15.58 Beta4"));
        assert!(is_mangacon_window_title("MangaCon"));
        assert!(!is_mangacon_window_title(
            "mangacon-companion - 文件资源管理器"
        ));
    }

    #[test]
    fn mangacon_process_path_requires_mangacon_executable() {
        assert!(is_mangacon_process_path(r"E:\漫画控\MangaCon.exe"));
        assert!(!is_mangacon_process_path(r"C:\Windows\Explorer.EXE"));
    }

    #[test]
    #[ignore = "requires a visible MangaCon.exe window on Windows"]
    fn manual_detects_running_mangacon_window() {
        let windows = find_mangacon_windows();

        assert!(
            windows
                .iter()
                .any(|window| window.title.contains("漫画控") || window.title.contains("MangaCon")),
            "expected a visible MangaCon window, found: {windows:?}"
        );
    }
}
