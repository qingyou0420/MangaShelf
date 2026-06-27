use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mangacon::{
    badge::{
        detect_detail_chapter_update_badges_from_rgba, detect_favorites_update_badges_from_rgba,
        BadgePoint,
    },
    window::{find_mangacon_windows, MangaConWindow},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedWindowImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaConBadgeScanResult {
    pub window: MangaConWindow,
    pub width: u32,
    pub height: u32,
    pub badges: Vec<BadgePoint>,
    pub fingerprint: u64,
}

#[derive(Debug, Error)]
pub enum WindowCaptureError {
    #[error("仅 Windows 桌面版支持窗口截图")]
    UnsupportedPlatform,
    #[error("没有找到可见的漫画控窗口")]
    NoMangaConWindow,
    #[error("没有可截图的漫画控窗口")]
    NoCapturableMangaConWindow,
    #[error("窗口句柄无效")]
    InvalidWindowHandle,
    #[error("窗口尺寸无效: {width}x{height}")]
    InvalidWindowSize { width: i32, height: i32 },
    #[error("获取窗口 DC 失败")]
    GetWindowDcFailed,
    #[error("创建内存 DC 失败")]
    CreateMemoryDcFailed,
    #[error("创建兼容位图失败")]
    CreateBitmapFailed,
    #[error("复制窗口像素失败: {0}")]
    BitBltFailed(#[from] windows::core::Error),
    #[error("读取位图像素失败")]
    ReadBitmapFailed,
}

pub fn scan_mangacon_badges() -> Result<MangaConBadgeScanResult, WindowCaptureError> {
    let windows = find_mangacon_windows();
    if windows.is_empty() {
        return Err(WindowCaptureError::NoMangaConWindow);
    }

    let mut best_scan: Option<MangaConBadgeScanResult> = None;
    for window in windows {
        let Ok(image) = capture_window_rgba(window.hwnd) else {
            continue;
        };
        let sample = detect_favorites_update_badges_from_rgba(
            image.width as usize,
            image.height as usize,
            &image.rgba,
        );
        let scan = MangaConBadgeScanResult {
            window,
            width: image.width,
            height: image.height,
            badges: sample.badges,
            fingerprint: rgba_fingerprint(image.width, image.height, &image.rgba),
        };

        let current_area = u64::from(scan.width) * u64::from(scan.height);
        let best_area = best_scan
            .as_ref()
            .map(|best| u64::from(best.width) * u64::from(best.height))
            .unwrap_or(0);
        if current_area > best_area {
            best_scan = Some(scan);
        }
    }

    best_scan.ok_or(WindowCaptureError::NoCapturableMangaConWindow)
}

pub fn scan_mangacon_detail_chapter_badges() -> Result<MangaConBadgeScanResult, WindowCaptureError>
{
    let windows = find_mangacon_windows();
    if windows.is_empty() {
        return Err(WindowCaptureError::NoMangaConWindow);
    }

    let mut best_scan: Option<MangaConBadgeScanResult> = None;
    for window in windows {
        let Ok(image) = capture_window_rgba(window.hwnd) else {
            continue;
        };
        let sample = detect_detail_chapter_update_badges_from_rgba(
            image.width as usize,
            image.height as usize,
            &image.rgba,
        );
        let scan = MangaConBadgeScanResult {
            window,
            width: image.width,
            height: image.height,
            badges: sample.badges,
            fingerprint: rgba_fingerprint(image.width, image.height, &image.rgba),
        };

        let current_area = u64::from(scan.width) * u64::from(scan.height);
        let best_area = best_scan
            .as_ref()
            .map(|best| u64::from(best.width) * u64::from(best.height))
            .unwrap_or(0);
        if current_area > best_area {
            best_scan = Some(scan);
        }
    }

    best_scan.ok_or(WindowCaptureError::NoCapturableMangaConWindow)
}

#[cfg(not(windows))]
pub fn capture_window_rgba(_hwnd: isize) -> Result<CapturedWindowImage, WindowCaptureError> {
    Err(WindowCaptureError::UnsupportedPlatform)
}

#[cfg(windows)]
pub fn capture_window_rgba(hwnd: isize) -> Result<CapturedWindowImage, WindowCaptureError> {
    use std::ffi::c_void;
    use windows::Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::{GetWindowDC, HDC},
        UI::WindowsAndMessaging::{
            GetWindowRect, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE,
        },
    };

    #[link(name = "user32")]
    extern "system" {
        fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
    }

    if hwnd == 0 {
        return Err(WindowCaptureError::InvalidWindowHandle);
    }

    let hwnd = HWND(hwnd as *mut c_void);
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            std::thread::sleep(std::time::Duration::from_millis(350));
        }
        let _ = SetForegroundWindow(hwnd);
    }

    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }
        .map_err(|_| WindowCaptureError::InvalidWindowHandle)?;

    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return Err(WindowCaptureError::InvalidWindowSize { width, height });
    }

    let window_dc = unsafe { GetWindowDC(Some(hwnd)) };
    if window_dc.0.is_null() {
        return Err(WindowCaptureError::GetWindowDcFailed);
    }

    let capture_result = unsafe { capture_from_dc(window_dc, width, height) };
    unsafe {
        let _ = ReleaseDC(hwnd, window_dc);
    }

    capture_result
}

#[cfg(windows)]
unsafe fn capture_from_dc(
    window_dc: windows::Win32::Graphics::Gdi::HDC,
    width: i32,
    height: i32,
) -> Result<CapturedWindowImage, WindowCaptureError> {
    use std::{ffi::c_void, mem};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ, SRCCOPY,
    };

    let memory_dc = unsafe { CreateCompatibleDC(Some(window_dc)) };
    if memory_dc.0.is_null() {
        return Err(WindowCaptureError::CreateMemoryDcFailed);
    }

    let bitmap = unsafe { CreateCompatibleBitmap(window_dc, width, height) };
    if bitmap.0.is_null() {
        unsafe {
            let _ = DeleteDC(memory_dc);
        }
        return Err(WindowCaptureError::CreateBitmapFailed);
    }

    let old_object = unsafe { SelectObject(memory_dc, HGDIOBJ(bitmap.0)) };
    let bitblt_result = unsafe {
        BitBlt(
            memory_dc,
            0,
            0,
            width,
            height,
            Some(window_dc),
            0,
            0,
            SRCCOPY,
        )
    };

    let mut bgra = vec![0_u8; width as usize * height as usize * 4];
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let lines_read = if bitblt_result.is_ok() {
        unsafe {
            GetDIBits(
                memory_dc,
                bitmap,
                0,
                height as u32,
                Some(bgra.as_mut_ptr().cast::<c_void>()),
                &mut info,
                DIB_RGB_COLORS,
            )
        }
    } else {
        0
    };

    unsafe {
        if !old_object.0.is_null() {
            let _ = SelectObject(memory_dc, old_object);
        }
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(memory_dc);
    }

    bitblt_result?;
    if lines_read == 0 {
        return Err(WindowCaptureError::ReadBitmapFailed);
    }

    Ok(CapturedWindowImage {
        width: width as u32,
        height: height as u32,
        rgba: bgra_to_rgba(bgra),
    })
}

fn bgra_to_rgba(mut pixels: Vec<u8>) -> Vec<u8> {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    pixels
}

fn rgba_fingerprint(width: u32, height: u32, rgba: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in width
        .to_le_bytes()
        .into_iter()
        .chain(height.to_le_bytes())
        .chain(rgba.iter().copied())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mangacon::badge::detect_badge_points_from_rgba;
    use crate::mangacon::window::find_mangacon_windows;

    #[test]
    #[ignore = "requires a visible MangaCon.exe window on Windows"]
    fn manual_captures_running_mangacon_window() {
        let window = find_mangacon_windows()
            .into_iter()
            .next()
            .expect("visible MangaCon window");

        let image = capture_window_rgba(window.hwnd).expect("window capture");
        let badges =
            detect_badge_points_from_rgba(image.width as usize, image.height as usize, &image.rgba);

        assert!(image.width > 300, "unexpected width: {}", image.width);
        assert!(image.height > 200, "unexpected height: {}", image.height);
        println!(
            "captured {}x{}, detected badges: {:?}",
            image.width, image.height, badges.badges
        );
    }

    #[test]
    #[ignore = "requires a visible MangaCon.exe window on Windows"]
    fn manual_scans_running_mangacon_badges() {
        let scan = scan_mangacon_badges().expect("badge scan");

        assert!(scan.width > 300, "unexpected width: {}", scan.width);
        assert!(scan.height > 200, "unexpected height: {}", scan.height);
        println!(
            "scanned {}x{}, window: {}, badges: {:?}",
            scan.width, scan.height, scan.window.title, scan.badges
        );
    }
}
