use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mangacon::{
    badge::BadgePoint,
    capture::{scan_mangacon_badges, WindowCaptureError},
    window::MangaConWindow,
};

const REFERENCE_WINDOW_WIDTH: u32 = 850;
const REFERENCE_WINDOW_HEIGHT: u32 = 600;
const HOME_FAVORITES_BUTTON_X: i32 = 212;
const HOME_FAVORITES_BUTTON_Y: i32 = 330;
const TITLE_BAR_HEIGHT: i32 = 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenFavoritesResult {
    pub window: MangaConWindow,
    pub clicked: WindowPoint,
    pub width: u32,
    pub height: u32,
    pub badges: Vec<BadgePoint>,
}

#[derive(Debug, Error)]
pub enum NavigationError {
    #[error(transparent)]
    Capture(#[from] WindowCaptureError),
    #[error("发送漫画控点击消息失败: {0}")]
    ClickFailed(String),
}

pub fn favorites_button_point(width: u32, height: u32) -> WindowPoint {
    WindowPoint {
        x: scale_axis(HOME_FAVORITES_BUTTON_X, width, REFERENCE_WINDOW_WIDTH),
        y: scale_axis(HOME_FAVORITES_BUTTON_Y, height, REFERENCE_WINDOW_HEIGHT),
    }
}

pub fn open_favorites_from_home() -> Result<OpenFavoritesResult, NavigationError> {
    let before_scan = scan_mangacon_badges()?;
    let clicked = favorites_button_point(before_scan.width, before_scan.height);
    click_window_point(before_scan.window.hwnd, clicked)?;

    std::thread::sleep(std::time::Duration::from_millis(900));

    let after_scan = scan_mangacon_badges()?;
    Ok(OpenFavoritesResult {
        window: after_scan.window,
        clicked,
        width: after_scan.width,
        height: after_scan.height,
        badges: after_scan.badges,
    })
}

fn scale_axis(value: i32, actual: u32, reference: u32) -> i32 {
    ((i64::from(value) * i64::from(actual)) / i64::from(reference)) as i32
}

#[cfg(not(windows))]
fn click_window_point(_hwnd: isize, _point: WindowPoint) -> Result<(), NavigationError> {
    Err(NavigationError::ClickFailed(
        "仅 Windows 桌面版支持窗口点击".to_string(),
    ))
}

#[cfg(windows)]
fn click_window_point(hwnd: isize, point: WindowPoint) -> Result<(), NavigationError> {
    use std::ffi::c_void;
    use windows::Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::{PostMessageW, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE},
    };

    if hwnd == 0 {
        return Err(NavigationError::ClickFailed("窗口句柄无效".to_string()));
    }

    let client_point = WindowPoint {
        x: point.x,
        y: point.y - TITLE_BAR_HEIGHT,
    };
    let lparam = LPARAM(pack_client_point(client_point));
    let hwnd = HWND(hwnd as *mut c_void);

    unsafe {
        PostMessageW(Some(hwnd), WM_MOUSEMOVE, WPARAM(0), lparam)
            .map_err(|err| NavigationError::ClickFailed(err.to_string()))?;
        PostMessageW(Some(hwnd), WM_LBUTTONDOWN, WPARAM(1), lparam)
            .map_err(|err| NavigationError::ClickFailed(err.to_string()))?;
        PostMessageW(Some(hwnd), WM_LBUTTONUP, WPARAM(0), lparam)
            .map_err(|err| NavigationError::ClickFailed(err.to_string()))?;
    }

    Ok(())
}

fn pack_client_point(point: WindowPoint) -> isize {
    let x = (point.x as u16) as u32;
    let y = (point.y as u16) as u32;
    ((y << 16) | x) as isize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favorites_button_point_uses_home_screen_reference() {
        assert_eq!(
            favorites_button_point(850, 600),
            WindowPoint { x: 212, y: 330 }
        );
        assert_eq!(
            favorites_button_point(1700, 1200),
            WindowPoint { x: 424, y: 660 }
        );
    }

    #[test]
    fn client_point_packs_into_lparam_low_x_high_y() {
        assert_eq!(
            pack_client_point(WindowPoint { x: 212, y: 299 }),
            19_595_476
        );
    }

    #[test]
    #[ignore = "requires MangaCon.exe on home screen"]
    fn manual_opens_favorites_from_home() {
        let result = open_favorites_from_home().expect("open favorites");

        assert!(result.width > 300, "unexpected width: {}", result.width);
        assert!(result.height > 200, "unexpected height: {}", result.height);
        println!(
            "clicked {:?}, scanned {}x{}, badges: {:?}",
            result.clicked, result.width, result.height, result.badges
        );
    }
}
