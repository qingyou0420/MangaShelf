use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueDownloadConfirmResult {
    pub found: bool,
    pub clicked: bool,
    pub dialog_title: Option<String>,
}

pub fn is_continue_download_dialog(title: &str, child_texts: &[&str]) -> bool {
    let is_mangacon_title = title.starts_with("漫画控") || title.starts_with("MangaCon");
    let has_continue_message = child_texts.iter().any(|text| {
        text.contains("继续之前未完成的下载任务吗") || text.contains("繼續之前未完成的下載任務嗎")
    });

    is_mangacon_title && has_continue_message && find_yes_button_text_index(child_texts).is_some()
}

pub fn find_yes_button_text_index(child_texts: &[&str]) -> Option<usize> {
    child_texts
        .iter()
        .position(|text| text.trim() == "是" || text.trim().eq_ignore_ascii_case("yes"))
}

pub fn is_mangacon_webview_continue_dialog_candidate(
    title: &str,
    class_name: &str,
    client_width: i32,
    client_height: i32,
) -> bool {
    let is_dialog_title = title == "漫画控" || title == "MangaCon";
    let is_mangacon_webview = class_name == "MangaCon3_WebViewWindow";
    let is_small_dialog =
        (240..=420).contains(&client_width) && (80..=180).contains(&client_height);

    is_dialog_title && is_mangacon_webview && is_small_dialog
}

pub fn estimated_webview_yes_button_point(client_width: i32, client_height: i32) -> (i32, i32) {
    let x = (client_width * 35 / 100).clamp(0, client_width.saturating_sub(1));
    let y = (client_height * 73 / 100).clamp(0, client_height.saturating_sub(1));
    (x, y)
}

#[cfg(not(windows))]
pub fn confirm_continue_download_dialog() -> Result<ContinueDownloadConfirmResult> {
    Ok(ContinueDownloadConfirmResult {
        found: false,
        clicked: false,
        dialog_title: None,
    })
}

#[cfg(windows)]
pub fn confirm_continue_download_dialog() -> Result<ContinueDownloadConfirmResult> {
    use windows::core::BOOL;
    use windows::Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::{
            EnumChildWindows, EnumWindows, IsWindowVisible, SendMessageW, BM_CLICK,
        },
    };

    #[derive(Default)]
    struct DialogSearch {
        dialog: Option<HWND>,
        button: Option<HWND>,
        webview_dialog: Option<HWND>,
        title: Option<String>,
    }

    #[derive(Default)]
    struct ChildSearch {
        texts: Vec<String>,
        yes_button: Option<HWND>,
    }

    unsafe extern "system" fn enum_child(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let child_search = unsafe { &mut *(lparam.0 as *mut ChildSearch) };
        let text = unsafe { window_text(hwnd) };
        if text.is_empty() {
            return BOOL(1);
        }

        if text.trim() == "是" || text.trim().eq_ignore_ascii_case("yes") {
            child_search.yes_button = Some(hwnd);
        }
        child_search.texts.push(text);
        BOOL(1)
    }

    unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return BOOL(1);
        }

        let title = unsafe { window_text(hwnd) };
        if title.is_empty() {
            return BOOL(1);
        }
        let class_name = unsafe { window_class_name(hwnd) };
        let (client_width, client_height) = unsafe { client_size(hwnd) };

        let mut child_search = ChildSearch::default();
        let child_lparam = LPARAM(&mut child_search as *mut ChildSearch as isize);
        unsafe {
            let _ = EnumChildWindows(Some(hwnd), Some(enum_child), child_lparam);
        }
        let child_refs = child_search
            .texts
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();

        if is_continue_download_dialog(&title, &child_refs) {
            let dialog_search = unsafe { &mut *(lparam.0 as *mut DialogSearch) };
            dialog_search.dialog = Some(hwnd);
            dialog_search.button = child_search.yes_button;
            dialog_search.title = Some(title);
            return BOOL(0);
        }

        if is_mangacon_webview_continue_dialog_candidate(
            &title,
            &class_name,
            client_width,
            client_height,
        ) {
            let dialog_search = unsafe { &mut *(lparam.0 as *mut DialogSearch) };
            dialog_search.dialog = Some(hwnd);
            dialog_search.webview_dialog = Some(hwnd);
            dialog_search.title = Some(title);
            return BOOL(0);
        }

        BOOL(1)
    }

    let mut search = DialogSearch::default();
    let lparam = LPARAM(&mut search as *mut DialogSearch as isize);
    unsafe {
        let _ = EnumWindows(Some(enum_window), lparam);
    }

    if let Some(button) = search.button {
        unsafe {
            let _ = SendMessageW(button, BM_CLICK, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        return Ok(ContinueDownloadConfirmResult {
            found: true,
            clicked: true,
            dialog_title: search.title,
        });
    }

    if let Some(webview_dialog) = search.webview_dialog {
        unsafe {
            click_webview_yes_button(webview_dialog)?;
        }
        return Ok(ContinueDownloadConfirmResult {
            found: true,
            clicked: true,
            dialog_title: search.title,
        });
    }

    Ok(ContinueDownloadConfirmResult {
        found: search.dialog.is_some(),
        clicked: false,
        dialog_title: search.title,
    })
}

#[cfg(windows)]
unsafe fn window_text(hwnd: windows::Win32::Foundation::HWND) -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW};

    let title_length = unsafe { GetWindowTextLengthW(hwnd) };
    if title_length <= 0 {
        return String::new();
    }

    let mut buffer = vec![0_u16; title_length as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if copied <= 0 {
        return String::new();
    }

    String::from_utf16_lossy(&buffer[..copied as usize])
}

#[cfg(windows)]
unsafe fn window_class_name(hwnd: windows::Win32::Foundation::HWND) -> String {
    use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;

    let mut buffer = [0_u16; 128];
    let copied = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if copied <= 0 {
        return String::new();
    }

    String::from_utf16_lossy(&buffer[..copied as usize])
}

#[cfg(windows)]
unsafe fn client_size(hwnd: windows::Win32::Foundation::HWND) -> (i32, i32) {
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    let mut rect = windows::Win32::Foundation::RECT::default();
    if unsafe { GetClientRect(hwnd, &mut rect) }.is_err() {
        return (0, 0);
    }

    (rect.right - rect.left, rect.bottom - rect.top)
}

#[cfg(windows)]
unsafe fn click_webview_yes_button(hwnd: windows::Win32::Foundation::HWND) -> Result<()> {
    use windows::Win32::{
        Foundation::WPARAM,
        UI::WindowsAndMessaging::{
            GetClientRect, PostMessageW, SetForegroundWindow, WM_LBUTTONDOWN, WM_LBUTTONUP,
            WM_MOUSEMOVE,
        },
    };

    let mut rect = windows::Win32::Foundation::RECT::default();
    unsafe {
        GetClientRect(hwnd, &mut rect)?;
    }
    let (x, y) = estimated_webview_yes_button_point(rect.right - rect.left, rect.bottom - rect.top);
    let point = mouse_lparam(x, y);

    unsafe {
        let _ = SetForegroundWindow(hwnd);
        PostMessageW(Some(hwnd), WM_MOUSEMOVE, WPARAM(0), point)?;
        PostMessageW(Some(hwnd), WM_LBUTTONDOWN, WPARAM(1), point)?;
        PostMessageW(Some(hwnd), WM_LBUTTONUP, WPARAM(0), point)?;
    }
    Ok(())
}

#[cfg(windows)]
fn mouse_lparam(x: i32, y: i32) -> windows::Win32::Foundation::LPARAM {
    windows::Win32::Foundation::LPARAM((((y & 0xffff) << 16) | (x & 0xffff)) as isize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_only_continue_unfinished_download_dialog() {
        assert!(is_continue_download_dialog(
            "漫画控",
            &["继续之前未完成的下载任务吗?", "是", "否"]
        ));
        assert!(is_continue_download_dialog(
            "漫画控",
            &["继续之前未完成的下载任务吗？", "是", "否"]
        ));
        assert!(!is_continue_download_dialog(
            "漫画控",
            &["确定要删除任务吗？", "是", "否"]
        ));
        assert!(!is_continue_download_dialog(
            "其他程序",
            &["继续之前未完成的下载任务吗？", "是", "否"]
        ));
    }

    #[test]
    fn finds_yes_button_from_dialog_children() {
        assert_eq!(
            find_yes_button_text_index(&["继续之前未完成的下载任务吗？", "是", "否"]),
            Some(1)
        );
        assert_eq!(
            find_yes_button_text_index(&["继续之前未完成的下载任务吗？", "否"]),
            None
        );
    }

    #[test]
    fn detects_mangacon_webview_continue_dialog_candidate() {
        assert!(is_mangacon_webview_continue_dialog_candidate(
            "漫画控",
            "MangaCon3_WebViewWindow",
            300,
            110
        ));
        assert!(!is_mangacon_webview_continue_dialog_candidate(
            "漫画控 v3.0.15.58 Beta4",
            "MangaCon3",
            834,
            561
        ));
        assert!(!is_mangacon_webview_continue_dialog_candidate(
            "漫画控",
            "MangaCon3_WebViewWindow",
            834,
            561
        ));
    }

    #[test]
    fn estimates_webview_yes_button_point_from_observed_dialog_size() {
        assert_eq!(estimated_webview_yes_button_point(300, 110), (105, 80));
    }
}
