use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaConWindow {
    pub hwnd: isize,
    pub title: String,
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
        if title.contains("漫画控") || title.contains("MangaCon") {
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
