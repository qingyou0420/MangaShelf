use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::mangacon::{
    badge::BadgePoint,
    capture::{scan_mangacon_badges, scan_mangacon_detail_chapter_badges, WindowCaptureError},
    window::MangaConWindow,
};

const REFERENCE_WINDOW_WIDTH: u32 = 850;
const REFERENCE_WINDOW_HEIGHT: u32 = 600;
const HOME_FAVORITES_BUTTON_X: i32 = 212;
const HOME_FAVORITES_BUTTON_Y: i32 = 330;
const BACK_BUTTON_X: i32 = 28;
const BACK_BUTTON_Y: i32 = 54;
const TITLE_BAR_HEIGHT: i32 = 31;
const BADGE_TO_CARD_CENTER_X_OFFSET: i32 = -57;
const BADGE_TO_CARD_CENTER_Y_OFFSET: i32 = 76;
const DETAIL_BADGE_TO_CHAPTER_BUTTON_X_OFFSET: i32 = 52;
const DETAIL_SCAN_MAX_SCROLLS: u32 = 8;
const DETAIL_SCAN_SCROLL_NOTCHES: i32 = 6;
const FAVORITES_SCAN_MAX_SCROLLS: u32 = 48;
const FAVORITES_SCAN_SCROLL_NOTCHES: i32 = 6;
const FAVORITES_SCAN_RESET_NOTCHES: i32 = 360;
const FAVORITE_UPDATE_BATCH_DEFAULT_LIMIT: u32 = 3;
const FAVORITE_UPDATE_BATCH_MAX_LIMIT: u32 = 10;
const FAVORITE_UPDATE_ALL_DEFAULT_LIMIT: u32 = 500;
const FAVORITE_UPDATE_ALL_MAX_LIMIT: u32 = 1_000;
const DETAIL_UPDATE_BATCH_DEFAULT_LIMIT: u32 = 20;
const DETAIL_UPDATE_BATCH_MAX_LIMIT: u32 = 80;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenComicResult {
    pub window: MangaConWindow,
    pub badge: BadgePoint,
    pub clicked: WindowPoint,
    pub width: u32,
    pub height: u32,
    pub remaining_badges: Vec<BadgePoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenScrolledComicResult {
    pub window: MangaConWindow,
    pub badge: BadgePoint,
    pub clicked: WindowPoint,
    pub width: u32,
    pub height: u32,
    pub remaining_badges: Vec<BadgePoint>,
    pub scroll_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailUpdateScanResult {
    pub window: MangaConWindow,
    pub width: u32,
    pub height: u32,
    pub badges: Vec<BadgePoint>,
    pub scroll_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoritesUpdateScanPage {
    pub scroll_attempts: u32,
    pub badges: Vec<BadgePoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoritesUpdateScanResult {
    pub window: MangaConWindow,
    pub width: u32,
    pub height: u32,
    pub badges: Vec<BadgePoint>,
    pub pages: Vec<FavoritesUpdateScanPage>,
    pub scroll_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerDetailDownloadResult {
    pub window: MangaConWindow,
    pub badge: BadgePoint,
    pub clicked: WindowPoint,
    pub width: u32,
    pub height: u32,
    pub remaining_badges: Vec<BadgePoint>,
    pub scroll_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerNextFavoriteUpdateDownloadResult {
    pub comic: OpenScrolledComicResult,
    pub download: TriggerDetailDownloadResult,
    pub download_batch: TriggerDetailDownloadBatchResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetailUpdateBatchStoppedReason {
    LimitReached,
    NoUpdateBadge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerDetailDownloadBatchResult {
    pub requested_limit: u32,
    pub processed: u32,
    pub stopped_reason: DetailUpdateBatchStoppedReason,
    pub downloads: Vec<TriggerDetailDownloadResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FavoriteUpdateSkipReason {
    DetailNoUpdateBadge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedFavoriteUpdateResult {
    pub comic: OpenScrolledComicResult,
    pub reason: FavoriteUpdateSkipReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FavoriteUpdateBatchStoppedReason {
    LimitReached,
    NoUpdateBadge,
    DetailNoUpdateBadge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggerFavoriteUpdateBatchResult {
    pub requested_limit: u32,
    pub processed: u32,
    pub downloaded_chapters: u32,
    pub stopped_reason: FavoriteUpdateBatchStoppedReason,
    pub skipped: Vec<SkippedFavoriteUpdateResult>,
    pub items: Vec<TriggerNextFavoriteUpdateDownloadResult>,
}

#[derive(Debug, Error)]
pub enum NavigationError {
    #[error(transparent)]
    Capture(#[from] WindowCaptureError),
    #[error("发送漫画控点击消息失败: {0}")]
    ClickFailed(String),
    #[error("当前收藏夹页面没有识别到更新红点")]
    NoUpdateBadge,
}

pub fn favorites_button_point(width: u32, height: u32) -> WindowPoint {
    WindowPoint {
        x: scale_axis(HOME_FAVORITES_BUTTON_X, width, REFERENCE_WINDOW_WIDTH),
        y: scale_axis(HOME_FAVORITES_BUTTON_Y, height, REFERENCE_WINDOW_HEIGHT),
    }
}

pub fn back_button_point(width: u32, height: u32) -> WindowPoint {
    WindowPoint {
        x: scale_axis(BACK_BUTTON_X, width, REFERENCE_WINDOW_WIDTH),
        y: scale_axis(BACK_BUTTON_Y, height, REFERENCE_WINDOW_HEIGHT),
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

pub fn return_to_favorites_from_detail() -> Result<OpenFavoritesResult, NavigationError> {
    let before_scan = scan_mangacon_badges()?;
    let clicked = back_button_point(before_scan.width, before_scan.height);
    foreground_click_window_point_once(before_scan.window.hwnd, clicked)?;

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

pub fn open_first_badged_comic_from_favorites() -> Result<OpenComicResult, NavigationError> {
    let before_scan = scan_mangacon_badges()?;
    let badge = first_update_badge(&before_scan.badges).ok_or(NavigationError::NoUpdateBadge)?;
    let clicked = comic_card_point_from_badge(badge);
    foreground_click_window_point(before_scan.window.hwnd, clicked)?;

    std::thread::sleep(std::time::Duration::from_millis(1_000));

    let after_scan = scan_mangacon_badges()?;
    Ok(OpenComicResult {
        window: after_scan.window,
        badge,
        clicked,
        width: after_scan.width,
        height: after_scan.height,
        remaining_badges: after_scan.badges,
    })
}

pub fn open_next_badged_comic_from_favorites() -> Result<OpenScrolledComicResult, NavigationError> {
    open_next_badged_comic_from_favorites_excluding(&[])
}

fn open_next_badged_comic_from_favorites_excluding(
    attempted_targets: &[NextFavoriteUpdateTarget],
) -> Result<OpenScrolledComicResult, NavigationError> {
    let scan = scan_next_favorite_update_with_scroll_excluding(attempted_targets)?;
    let clicked = comic_card_point_from_badge(scan.badge);
    foreground_click_window_point(scan.window.hwnd, clicked)?;

    std::thread::sleep(std::time::Duration::from_millis(1_000));

    let after_scan = scan_mangacon_badges()?;
    Ok(OpenScrolledComicResult {
        window: after_scan.window,
        badge: scan.badge,
        clicked,
        width: after_scan.width,
        height: after_scan.height,
        remaining_badges: after_scan.badges,
        scroll_attempts: scan.scroll_attempts,
    })
}

pub fn scan_favorites_updates_with_scroll() -> Result<FavoritesUpdateScanResult, NavigationError> {
    let mut scan = scan_mangacon_badges()?;
    scroll_window_up(scan.window.hwnd, FAVORITES_SCAN_RESET_NOTCHES)?;
    std::thread::sleep(std::time::Duration::from_millis(650));

    scan = scan_mangacon_badges()?;
    let mut samples = vec![FavoritesScrollScanSample::new(
        0,
        scan.fingerprint,
        scan.badges.clone(),
    )];
    let mut previous_fingerprint = scan.fingerprint;
    let mut unchanged_viewports = 0;

    for scroll_attempts in 1..=FAVORITES_SCAN_MAX_SCROLLS {
        scroll_window_down(scan.window.hwnd, FAVORITES_SCAN_SCROLL_NOTCHES)?;
        std::thread::sleep(std::time::Duration::from_millis(420));
        scan = scan_mangacon_badges()?;
        if scan.fingerprint == previous_fingerprint {
            unchanged_viewports += 1;
        } else {
            previous_fingerprint = scan.fingerprint;
            unchanged_viewports = 0;
        }

        samples.push(FavoritesScrollScanSample::new(
            scroll_attempts,
            scan.fingerprint,
            scan.badges.clone(),
        ));

        if unchanged_viewports >= 2 {
            break;
        }
    }

    let summary = favorites_scroll_scan_summary_from_samples(samples);
    Ok(FavoritesUpdateScanResult {
        window: scan.window,
        width: scan.width,
        height: scan.height,
        badges: summary.badges,
        pages: summary.pages,
        scroll_attempts: summary.scroll_attempts,
    })
}

pub fn scan_next_favorite_update_with_scroll(
) -> Result<NextFavoriteUpdateScanResult, NavigationError> {
    scan_next_favorite_update_with_scroll_excluding(&[])
}

fn scan_next_favorite_update_with_scroll_excluding(
    attempted_targets: &[NextFavoriteUpdateTarget],
) -> Result<NextFavoriteUpdateScanResult, NavigationError> {
    let mut scan = scan_mangacon_badges()?;
    scroll_window_up(scan.window.hwnd, FAVORITES_SCAN_RESET_NOTCHES)?;
    std::thread::sleep(std::time::Duration::from_millis(650));

    scan = scan_mangacon_badges()?;
    if let Some(target) = next_favorite_update_target_excluding(0, &scan.badges, attempted_targets)
    {
        return Ok(NextFavoriteUpdateScanResult {
            window: scan.window,
            width: scan.width,
            height: scan.height,
            badge: target.badge,
            badges: scan.badges,
            scroll_attempts: target.scroll_attempts,
        });
    }

    let mut previous_fingerprint = scan.fingerprint;
    let mut unchanged_viewports = 0;
    for scroll_attempts in 1..=FAVORITES_SCAN_MAX_SCROLLS {
        scroll_window_down(scan.window.hwnd, FAVORITES_SCAN_SCROLL_NOTCHES)?;
        std::thread::sleep(std::time::Duration::from_millis(420));
        scan = scan_mangacon_badges()?;

        if let Some(target) =
            next_favorite_update_target_excluding(scroll_attempts, &scan.badges, attempted_targets)
        {
            return Ok(NextFavoriteUpdateScanResult {
                window: scan.window,
                width: scan.width,
                height: scan.height,
                badge: target.badge,
                badges: scan.badges,
                scroll_attempts: target.scroll_attempts,
            });
        }

        if scan.fingerprint == previous_fingerprint {
            unchanged_viewports += 1;
        } else {
            previous_fingerprint = scan.fingerprint;
            unchanged_viewports = 0;
        }
        if unchanged_viewports >= 2 {
            break;
        }
    }

    Err(NavigationError::NoUpdateBadge)
}

pub fn scan_detail_updates_with_scroll() -> Result<DetailUpdateScanResult, NavigationError> {
    let mut scan = scan_mangacon_detail_chapter_badges()?;
    if !scan.badges.is_empty() {
        return Ok(DetailUpdateScanResult {
            window: scan.window,
            width: scan.width,
            height: scan.height,
            badges: scan.badges,
            scroll_attempts: 0,
        });
    }

    for scroll_attempts in 1..=DETAIL_SCAN_MAX_SCROLLS {
        scroll_window_down(scan.window.hwnd, DETAIL_SCAN_SCROLL_NOTCHES)?;
        std::thread::sleep(std::time::Duration::from_millis(450));
        scan = scan_mangacon_detail_chapter_badges()?;
        if !scan.badges.is_empty() {
            return Ok(DetailUpdateScanResult {
                window: scan.window,
                width: scan.width,
                height: scan.height,
                badges: scan.badges,
                scroll_attempts,
            });
        }
    }

    Ok(DetailUpdateScanResult {
        window: scan.window,
        width: scan.width,
        height: scan.height,
        badges: scan.badges,
        scroll_attempts: DETAIL_SCAN_MAX_SCROLLS,
    })
}

fn scan_next_detail_update_with_scroll_excluding(
    attempted_targets: &[DetailUpdateTarget],
) -> Result<NextDetailUpdateScanResult, NavigationError> {
    let mut scan = scan_mangacon_detail_chapter_badges()?;
    if let Some(target) =
        next_detail_update_target_excluding(0, scan.fingerprint, &scan.badges, attempted_targets)
    {
        return Ok(NextDetailUpdateScanResult {
            window: scan.window,
            target,
            scroll_attempts: target.scroll_attempts,
        });
    }

    for scroll_attempts in 1..=DETAIL_SCAN_MAX_SCROLLS {
        scroll_window_down(scan.window.hwnd, DETAIL_SCAN_SCROLL_NOTCHES)?;
        std::thread::sleep(std::time::Duration::from_millis(450));
        scan = scan_mangacon_detail_chapter_badges()?;
        if let Some(target) = next_detail_update_target_excluding(
            scroll_attempts,
            scan.fingerprint,
            &scan.badges,
            attempted_targets,
        ) {
            return Ok(NextDetailUpdateScanResult {
                window: scan.window,
                target,
                scroll_attempts: target.scroll_attempts,
            });
        }
    }

    Err(NavigationError::NoUpdateBadge)
}

pub fn trigger_first_detail_update_download() -> Result<TriggerDetailDownloadResult, NavigationError>
{
    let (download, _) = trigger_next_detail_update_download_excluding(&[])?;

    Ok(download)
}

fn trigger_next_detail_update_download_excluding(
    attempted_targets: &[DetailUpdateTarget],
) -> Result<(TriggerDetailDownloadResult, DetailUpdateTarget), NavigationError> {
    let scan = scan_next_detail_update_with_scroll_excluding(attempted_targets)?;
    let badge = scan.target.badge;
    let clicked = detail_chapter_button_point_from_badge(badge);
    foreground_click_window_point_once(scan.window.hwnd, clicked)?;

    std::thread::sleep(std::time::Duration::from_millis(1_000));

    let after_scan = scan_mangacon_detail_chapter_badges()?;
    Ok((
        TriggerDetailDownloadResult {
            window: after_scan.window,
            badge,
            clicked,
            width: after_scan.width,
            height: after_scan.height,
            remaining_badges: after_scan.badges,
            scroll_attempts: scan.scroll_attempts,
        },
        scan.target,
    ))
}

pub fn trigger_detail_update_download_batch(
    max_chapters: Option<u32>,
) -> Result<TriggerDetailDownloadBatchResult, NavigationError> {
    let requested_limit = detail_update_batch_limit(max_chapters);
    let mut downloads = Vec::new();
    let mut attempted_targets = Vec::new();
    let mut stopped_reason = DetailUpdateBatchStoppedReason::LimitReached;

    for _ in 0..requested_limit {
        let (download, target) =
            match trigger_next_detail_update_download_excluding(&attempted_targets) {
                Ok(result) => result,
                Err(NavigationError::NoUpdateBadge) => {
                    stopped_reason = DetailUpdateBatchStoppedReason::NoUpdateBadge;
                    break;
                }
                Err(error) => return Err(error),
            };

        attempted_targets.push(target);
        downloads.push(download);
    }

    Ok(TriggerDetailDownloadBatchResult {
        requested_limit,
        processed: downloads.len() as u32,
        stopped_reason,
        downloads,
    })
}

pub fn trigger_next_favorite_update_download(
) -> Result<TriggerNextFavoriteUpdateDownloadResult, NavigationError> {
    let comic = open_next_badged_comic_from_favorites()?;
    let download_batch = trigger_detail_update_download_batch(None)?;
    let Some(download) = download_batch.downloads.first().cloned() else {
        return Err(NavigationError::NoUpdateBadge);
    };

    Ok(TriggerNextFavoriteUpdateDownloadResult {
        comic,
        download,
        download_batch,
    })
}

pub fn trigger_favorite_update_batch(
    max_updates: Option<u32>,
) -> Result<TriggerFavoriteUpdateBatchResult, NavigationError> {
    let requested_limit = favorite_update_batch_limit(max_updates);
    trigger_favorite_update_loop(requested_limit)
}

pub fn trigger_all_favorite_updates(
    max_comics: Option<u32>,
) -> Result<TriggerFavoriteUpdateBatchResult, NavigationError> {
    let requested_limit = favorite_update_all_limit(max_comics);
    trigger_favorite_update_loop(requested_limit)
}

fn trigger_favorite_update_loop(
    requested_limit: u32,
) -> Result<TriggerFavoriteUpdateBatchResult, NavigationError> {
    let mut items = Vec::new();
    let mut skipped = Vec::new();
    let mut attempted_targets = Vec::new();
    let mut downloaded_chapters = 0;
    let mut stopped_reason = FavoriteUpdateBatchStoppedReason::LimitReached;

    for _ in 0..requested_limit {
        let comic = match open_next_badged_comic_from_favorites_excluding(&attempted_targets) {
            Ok(comic) => comic,
            Err(NavigationError::NoUpdateBadge) => {
                stopped_reason = FavoriteUpdateBatchStoppedReason::NoUpdateBadge;
                break;
            }
            Err(error) => return Err(error),
        };
        let attempted_target = NextFavoriteUpdateTarget {
            badge: comic.badge,
            scroll_attempts: comic.scroll_attempts,
        };

        let download_batch = trigger_detail_update_download_batch(None)?;
        let Some(download) = download_batch.downloads.first().cloned() else {
            attempted_targets.push(attempted_target);
            skipped.push(SkippedFavoriteUpdateResult {
                comic,
                reason: FavoriteUpdateSkipReason::DetailNoUpdateBadge,
            });
            return_to_favorites_from_detail()?;
            continue;
        };

        attempted_targets.push(attempted_target);
        downloaded_chapters += download_batch.processed;
        items.push(TriggerNextFavoriteUpdateDownloadResult {
            comic,
            download,
            download_batch,
        });
        return_to_favorites_from_detail()?;
    }

    Ok(TriggerFavoriteUpdateBatchResult {
        requested_limit,
        processed: items.len() as u32,
        downloaded_chapters,
        stopped_reason,
        skipped,
        items,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FavoritesScrollScanSummary {
    badges: Vec<BadgePoint>,
    pages: Vec<FavoritesUpdateScanPage>,
    scroll_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextFavoriteUpdateScanResult {
    pub window: MangaConWindow,
    pub width: u32,
    pub height: u32,
    pub badge: BadgePoint,
    pub badges: Vec<BadgePoint>,
    pub scroll_attempts: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NextFavoriteUpdateTarget {
    badge: BadgePoint,
    scroll_attempts: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DetailUpdateTarget {
    fingerprint: u64,
    badge: BadgePoint,
    scroll_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NextDetailUpdateScanResult {
    window: MangaConWindow,
    target: DetailUpdateTarget,
    scroll_attempts: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FavoritesScrollScanSample {
    scroll_attempts: u32,
    fingerprint: u64,
    badges: Vec<BadgePoint>,
}

impl FavoritesScrollScanSample {
    fn new(scroll_attempts: u32, fingerprint: u64, badges: Vec<BadgePoint>) -> Self {
        Self {
            scroll_attempts,
            fingerprint,
            badges,
        }
    }
}

fn next_favorite_update_target_excluding(
    scroll_attempts: u32,
    badges: &[BadgePoint],
    attempted_targets: &[NextFavoriteUpdateTarget],
) -> Option<NextFavoriteUpdateTarget> {
    badges
        .iter()
        .copied()
        .map(|badge| NextFavoriteUpdateTarget {
            badge,
            scroll_attempts,
        })
        .filter(|target| !attempted_targets.contains(target))
        .min_by_key(|target| (target.badge.y, target.badge.x))
}

fn next_detail_update_target_excluding(
    scroll_attempts: u32,
    fingerprint: u64,
    badges: &[BadgePoint],
    attempted_targets: &[DetailUpdateTarget],
) -> Option<DetailUpdateTarget> {
    badges
        .iter()
        .copied()
        .map(|badge| DetailUpdateTarget {
            fingerprint,
            badge,
            scroll_attempts,
        })
        .filter(|target| !attempted_targets.contains(target))
        .min_by_key(|target| (target.badge.y, target.badge.x))
}

fn favorite_update_batch_limit(max_updates: Option<u32>) -> u32 {
    max_updates
        .unwrap_or(FAVORITE_UPDATE_BATCH_DEFAULT_LIMIT)
        .clamp(1, FAVORITE_UPDATE_BATCH_MAX_LIMIT)
}

pub(crate) fn favorite_update_all_limit(max_comics: Option<u32>) -> u32 {
    max_comics
        .unwrap_or(FAVORITE_UPDATE_ALL_DEFAULT_LIMIT)
        .clamp(1, FAVORITE_UPDATE_ALL_MAX_LIMIT)
}

fn detail_update_batch_limit(max_chapters: Option<u32>) -> u32 {
    max_chapters
        .unwrap_or(DETAIL_UPDATE_BATCH_DEFAULT_LIMIT)
        .clamp(1, DETAIL_UPDATE_BATCH_MAX_LIMIT)
}

fn favorites_scroll_scan_summary_from_samples(
    samples: Vec<FavoritesScrollScanSample>,
) -> FavoritesScrollScanSummary {
    let scroll_attempts = samples
        .last()
        .map(|sample| sample.scroll_attempts)
        .unwrap_or(0);
    let mut previous_fingerprint = None;
    let mut pages = Vec::new();
    for sample in samples {
        if previous_fingerprint == Some(sample.fingerprint) {
            continue;
        }
        previous_fingerprint = Some(sample.fingerprint);
        if !sample.badges.is_empty() {
            pages.push(FavoritesUpdateScanPage {
                scroll_attempts: sample.scroll_attempts,
                badges: sample.badges,
            });
        }
    }
    let badges = pages
        .iter()
        .flat_map(|page| page.badges.iter().copied())
        .collect::<Vec<_>>();

    FavoritesScrollScanSummary {
        badges,
        pages,
        scroll_attempts,
    }
}

fn comic_card_point_from_badge(badge: BadgePoint) -> WindowPoint {
    WindowPoint {
        x: badge.x + BADGE_TO_CARD_CENTER_X_OFFSET,
        y: badge.y + BADGE_TO_CARD_CENTER_Y_OFFSET,
    }
}

fn detail_chapter_button_point_from_badge(badge: BadgePoint) -> WindowPoint {
    WindowPoint {
        x: badge.x + DETAIL_BADGE_TO_CHAPTER_BUTTON_X_OFFSET,
        y: badge.y,
    }
}

fn first_update_badge(badges: &[BadgePoint]) -> Option<BadgePoint> {
    badges
        .iter()
        .copied()
        .min_by_key(|badge| (badge.y, badge.x))
}

fn screen_point_from_window_origin(
    point: WindowPoint,
    window_left: i32,
    window_top: i32,
) -> WindowPoint {
    WindowPoint {
        x: window_left + point.x,
        y: window_top + point.y,
    }
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
    post_click_window_point(hwnd, point, 1)
}

#[cfg(windows)]
fn post_click_window_point(
    hwnd: isize,
    point: WindowPoint,
    count: usize,
) -> Result<(), NavigationError> {
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
        for _ in 0..count {
            PostMessageW(Some(hwnd), WM_MOUSEMOVE, WPARAM(0), lparam)
                .map_err(|err| NavigationError::ClickFailed(err.to_string()))?;
            PostMessageW(Some(hwnd), WM_LBUTTONDOWN, WPARAM(1), lparam)
                .map_err(|err| NavigationError::ClickFailed(err.to_string()))?;
            PostMessageW(Some(hwnd), WM_LBUTTONUP, WPARAM(0), lparam)
                .map_err(|err| NavigationError::ClickFailed(err.to_string()))?;
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn foreground_click_window_point(_hwnd: isize, _point: WindowPoint) -> Result<(), NavigationError> {
    Err(NavigationError::ClickFailed(
        "仅 Windows 桌面版支持前台窗口点击".to_string(),
    ))
}

#[cfg(windows)]
fn foreground_click_window_point(hwnd: isize, point: WindowPoint) -> Result<(), NavigationError> {
    use std::ffi::c_void;
    use windows::Win32::{
        Foundation::{HWND, RECT},
        UI::WindowsAndMessaging::{GetWindowRect, SetCursorPos, SetForegroundWindow},
    };

    if hwnd == 0 {
        return Err(NavigationError::ClickFailed("窗口句柄无效".to_string()));
    }

    let raw_hwnd = hwnd;
    let hwnd = HWND(raw_hwnd as *mut c_void);
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }
        .map_err(|err| NavigationError::ClickFailed(err.to_string()))?;
    let screen_point = screen_point_from_window_origin(point, rect.left, rect.top);

    unsafe {
        let _ = SetForegroundWindow(hwnd);
    }
    std::thread::sleep(std::time::Duration::from_millis(200));

    unsafe {
        if SetCursorPos(screen_point.x, screen_point.y).is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(80));
            send_left_clicks(2);
        } else {
            post_click_window_point(raw_hwnd, point, 2)?;
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn foreground_click_window_point_once(
    _hwnd: isize,
    _point: WindowPoint,
) -> Result<(), NavigationError> {
    Err(NavigationError::ClickFailed(
        "仅 Windows 桌面版支持前台窗口点击".to_string(),
    ))
}

#[cfg(windows)]
fn foreground_click_window_point_once(
    hwnd: isize,
    point: WindowPoint,
) -> Result<(), NavigationError> {
    use std::ffi::c_void;
    use windows::Win32::{
        Foundation::{HWND, RECT},
        UI::WindowsAndMessaging::{GetWindowRect, SetCursorPos, SetForegroundWindow},
    };

    if hwnd == 0 {
        return Err(NavigationError::ClickFailed("窗口句柄无效".to_string()));
    }

    let raw_hwnd = hwnd;
    let hwnd = HWND(raw_hwnd as *mut c_void);
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }
        .map_err(|err| NavigationError::ClickFailed(err.to_string()))?;
    let screen_point = screen_point_from_window_origin(point, rect.left, rect.top);

    unsafe {
        let _ = SetForegroundWindow(hwnd);
    }
    std::thread::sleep(std::time::Duration::from_millis(200));

    unsafe {
        if SetCursorPos(screen_point.x, screen_point.y).is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(80));
            send_left_clicks(1);
        } else {
            post_click_window_point(raw_hwnd, point, 1)?;
        }
    }

    Ok(())
}

#[cfg(windows)]
unsafe fn send_left_clicks(count: usize) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        mouse_event, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    };

    for _ in 0..count {
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
        std::thread::sleep(std::time::Duration::from_millis(60));
        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[cfg(not(windows))]
fn scroll_window_down(_hwnd: isize, _notches: i32) -> Result<(), NavigationError> {
    Err(NavigationError::ClickFailed(
        "仅 Windows 桌面版支持窗口滚动".to_string(),
    ))
}

#[cfg(not(windows))]
fn scroll_window_up(_hwnd: isize, _notches: i32) -> Result<(), NavigationError> {
    Err(NavigationError::ClickFailed(
        "仅 Windows 桌面版支持窗口滚动".to_string(),
    ))
}

#[cfg(windows)]
fn scroll_window_down(hwnd: isize, notches: i32) -> Result<(), NavigationError> {
    scroll_window_by_delta(hwnd, -120 * notches)
}

#[cfg(windows)]
fn scroll_window_up(hwnd: isize, notches: i32) -> Result<(), NavigationError> {
    scroll_window_by_delta(hwnd, 120 * notches)
}

#[cfg(windows)]
fn scroll_window_by_delta(hwnd: isize, wheel_delta: i32) -> Result<(), NavigationError> {
    use std::ffi::c_void;
    use windows::Win32::{
        Foundation::{HWND, LPARAM, RECT, WPARAM},
        UI::{
            Input::KeyboardAndMouse::{mouse_event, MOUSEEVENTF_WHEEL},
            WindowsAndMessaging::{
                GetWindowRect, PostMessageW, SetCursorPos, SetForegroundWindow, WM_MOUSEWHEEL,
            },
        },
    };

    if hwnd == 0 {
        return Err(NavigationError::ClickFailed("窗口句柄无效".to_string()));
    }

    let raw_hwnd = hwnd;
    let hwnd = HWND(raw_hwnd as *mut c_void);
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }
        .map_err(|err| NavigationError::ClickFailed(err.to_string()))?;

    let center = WindowPoint {
        x: (rect.left + rect.right) / 2,
        y: (rect.top + rect.bottom) / 2,
    };

    unsafe {
        let _ = SetForegroundWindow(hwnd);
    }
    std::thread::sleep(std::time::Duration::from_millis(160));

    unsafe {
        if SetCursorPos(center.x, center.y).is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(60));
            mouse_event(MOUSEEVENTF_WHEEL, 0, 0, wheel_delta, 0);
        } else {
            let wparam = WPARAM(pack_wheel_delta_wparam(wheel_delta));
            let lparam = LPARAM(pack_client_point(center));
            PostMessageW(
                Some(HWND(raw_hwnd as *mut c_void)),
                WM_MOUSEWHEEL,
                wparam,
                lparam,
            )
            .map_err(|err| NavigationError::ClickFailed(err.to_string()))?;
        }
    }

    Ok(())
}

fn pack_wheel_delta_wparam(wheel_delta: i32) -> usize {
    ((wheel_delta as i16 as u16 as u32) << 16) as usize
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
    fn back_button_point_uses_window_reference() {
        assert_eq!(back_button_point(850, 600), WindowPoint { x: 28, y: 54 });
        assert_eq!(back_button_point(1700, 1200), WindowPoint { x: 56, y: 108 });
    }

    #[test]
    fn favorite_update_batch_limit_is_safe_and_bounded() {
        assert_eq!(favorite_update_batch_limit(None), 3);
        assert_eq!(favorite_update_batch_limit(Some(0)), 1);
        assert_eq!(favorite_update_batch_limit(Some(2)), 2);
        assert_eq!(favorite_update_batch_limit(Some(99)), 10);
    }

    #[test]
    fn favorite_update_all_limit_covers_full_collection_and_stays_bounded() {
        assert_eq!(favorite_update_all_limit(None), 500);
        assert_eq!(favorite_update_all_limit(Some(0)), 1);
        assert_eq!(favorite_update_all_limit(Some(447)), 447);
        assert_eq!(favorite_update_all_limit(Some(2_000)), 1_000);
    }

    #[test]
    fn client_point_packs_into_lparam_low_x_high_y() {
        assert_eq!(
            pack_client_point(WindowPoint { x: 212, y: 299 }),
            19_595_476
        );
    }

    #[test]
    fn wheel_delta_packs_into_wparam_high_word() {
        assert_eq!(pack_wheel_delta_wparam(120), 0x0078_0000);
        assert_eq!(pack_wheel_delta_wparam(-120), 0xff88_0000);
    }

    #[test]
    fn comic_card_point_uses_update_badge_corner_reference() {
        assert_eq!(
            comic_card_point_from_badge(BadgePoint { x: 174, y: 95 }),
            WindowPoint { x: 117, y: 171 }
        );
        assert_eq!(
            comic_card_point_from_badge(BadgePoint { x: 372, y: 95 }),
            WindowPoint { x: 315, y: 171 }
        );
        assert_eq!(
            comic_card_point_from_badge(BadgePoint { x: 174, y: 295 }),
            WindowPoint { x: 117, y: 371 }
        );
    }

    #[test]
    fn first_update_badge_prefers_top_left_position() {
        let badges = vec![
            BadgePoint { x: 572, y: 95 },
            BadgePoint { x: 174, y: 295 },
            BadgePoint { x: 174, y: 95 },
        ];

        assert_eq!(
            first_update_badge(&badges),
            Some(BadgePoint { x: 174, y: 95 })
        );
    }

    #[test]
    fn screen_point_uses_window_origin_without_client_offset() {
        assert_eq!(
            screen_point_from_window_origin(WindowPoint { x: 117, y: 171 }, 1300, 500),
            WindowPoint { x: 1417, y: 671 }
        );
    }

    #[test]
    fn detail_chapter_button_point_uses_badge_left_marker() {
        assert_eq!(
            detail_chapter_button_point_from_badge(BadgePoint { x: 151, y: 516 }),
            WindowPoint { x: 203, y: 516 }
        );
        assert_eq!(
            detail_chapter_button_point_from_badge(BadgePoint { x: 20, y: 82 }),
            WindowPoint { x: 72, y: 82 }
        );
    }

    #[test]
    fn favorites_scroll_scan_summary_keeps_badged_pages_and_flattened_badges() {
        let summary = favorites_scroll_scan_summary_from_samples(vec![
            FavoritesScrollScanSample::new(0, 100, Vec::new()),
            FavoritesScrollScanSample::new(1, 101, vec![BadgePoint { x: 174, y: 96 }]),
            FavoritesScrollScanSample::new(
                2,
                102,
                vec![BadgePoint { x: 374, y: 296 }, BadgePoint { x: 574, y: 96 }],
            ),
        ]);

        assert_eq!(summary.scroll_attempts, 2);
        assert_eq!(
            summary.badges,
            vec![
                BadgePoint { x: 174, y: 96 },
                BadgePoint { x: 374, y: 296 },
                BadgePoint { x: 574, y: 96 },
            ]
        );
        assert_eq!(summary.pages.len(), 2);
        assert_eq!(summary.pages[0].scroll_attempts, 1);
        assert_eq!(summary.pages[1].scroll_attempts, 2);
    }

    #[test]
    fn favorites_scroll_scan_summary_ignores_unchanged_viewport_repeats() {
        let summary = favorites_scroll_scan_summary_from_samples(vec![
            FavoritesScrollScanSample::new(0, 100, vec![BadgePoint { x: 174, y: 96 }]),
            FavoritesScrollScanSample::new(1, 200, vec![BadgePoint { x: 374, y: 296 }]),
            FavoritesScrollScanSample::new(2, 200, vec![BadgePoint { x: 374, y: 296 }]),
            FavoritesScrollScanSample::new(3, 200, vec![BadgePoint { x: 374, y: 296 }]),
        ]);

        assert_eq!(summary.scroll_attempts, 3);
        assert_eq!(
            summary.badges,
            vec![BadgePoint { x: 174, y: 96 }, BadgePoint { x: 374, y: 296 }]
        );
        assert_eq!(summary.pages.len(), 2);
    }

    #[test]
    fn next_favorite_update_target_keeps_scroll_attempt_and_top_left_badge() {
        let target = next_favorite_update_target_excluding(
            7,
            &[
                BadgePoint { x: 574, y: 296 },
                BadgePoint { x: 174, y: 496 },
                BadgePoint { x: 374, y: 96 },
            ],
            &[],
        );

        assert_eq!(
            target,
            Some(NextFavoriteUpdateTarget {
                badge: BadgePoint { x: 374, y: 96 },
                scroll_attempts: 7,
            })
        );
    }

    #[test]
    fn next_favorite_update_target_skips_excluded_badges() {
        let target = next_favorite_update_target_excluding(
            4,
            &[
                BadgePoint { x: 174, y: 96 },
                BadgePoint { x: 374, y: 96 },
                BadgePoint { x: 174, y: 296 },
            ],
            &[NextFavoriteUpdateTarget {
                badge: BadgePoint { x: 174, y: 96 },
                scroll_attempts: 4,
            }],
        );

        assert_eq!(
            target,
            Some(NextFavoriteUpdateTarget {
                badge: BadgePoint { x: 374, y: 96 },
                scroll_attempts: 4,
            })
        );
    }

    #[test]
    fn detail_update_batch_limit_is_safe_and_bounded() {
        assert_eq!(detail_update_batch_limit(None), 20);
        assert_eq!(detail_update_batch_limit(Some(0)), 1);
        assert_eq!(detail_update_batch_limit(Some(8)), 8);
        assert_eq!(detail_update_batch_limit(Some(200)), 80);
    }

    #[test]
    fn next_detail_update_target_excludes_same_viewport_badge_only() {
        let target = next_detail_update_target_excluding(
            0,
            222,
            &[
                BadgePoint { x: 20, y: 82 },
                BadgePoint { x: 142, y: 82 },
                BadgePoint { x: 20, y: 113 },
            ],
            &[DetailUpdateTarget {
                fingerprint: 222,
                badge: BadgePoint { x: 20, y: 82 },
                scroll_attempts: 0,
            }],
        );

        assert_eq!(
            target,
            Some(DetailUpdateTarget {
                fingerprint: 222,
                badge: BadgePoint { x: 142, y: 82 },
                scroll_attempts: 0,
            })
        );

        let same_coordinate_on_other_viewport = next_detail_update_target_excluding(
            0,
            333,
            &[BadgePoint { x: 20, y: 82 }],
            &[DetailUpdateTarget {
                fingerprint: 222,
                badge: BadgePoint { x: 20, y: 82 },
                scroll_attempts: 0,
            }],
        );

        assert_eq!(
            same_coordinate_on_other_viewport,
            Some(DetailUpdateTarget {
                fingerprint: 333,
                badge: BadgePoint { x: 20, y: 82 },
                scroll_attempts: 0,
            })
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

    #[test]
    #[ignore = "requires MangaCon.exe already on favorites page with update badges"]
    fn manual_opens_first_badged_comic_from_favorites() {
        let result =
            open_first_badged_comic_from_favorites().expect("open first badged comic detail");

        assert!(result.width > 300, "unexpected width: {}", result.width);
        assert!(result.height > 200, "unexpected height: {}", result.height);
        assert!(
            result.remaining_badges.is_empty(),
            "expected detail page without favorites-grid badges, got {:?}",
            result.remaining_badges
        );
        println!(
            "badge {:?}, clicked {:?}, scanned {}x{}, remaining badges: {:?}",
            result.badge, result.clicked, result.width, result.height, result.remaining_badges
        );
    }

    #[test]
    #[ignore = "requires MangaCon.exe already on favorites page"]
    fn manual_scans_favorites_update_badges_with_scroll() {
        let result = scan_favorites_updates_with_scroll().expect("scan favorites updates");

        assert!(result.width > 300, "unexpected width: {}", result.width);
        assert!(result.height > 200, "unexpected height: {}", result.height);
        println!(
            "scrolled {}, scanned {}x{}, pages: {}, favorites badges: {:?}",
            result.scroll_attempts,
            result.width,
            result.height,
            result.pages.len(),
            result.badges
        );
    }

    #[test]
    #[ignore = "requires MangaCon.exe already on a comic detail page"]
    fn manual_scans_detail_update_badges_with_scroll() {
        let result = scan_detail_updates_with_scroll().expect("scan detail updates");

        assert!(result.width > 300, "unexpected width: {}", result.width);
        assert!(result.height > 200, "unexpected height: {}", result.height);
        assert!(
            !result.badges.is_empty(),
            "expected detail update badges after scrolling"
        );
        println!(
            "scrolled {}, scanned {}x{}, detail badges: {:?}",
            result.scroll_attempts, result.width, result.height, result.badges
        );
    }

    #[test]
    #[ignore = "requires MangaCon.exe already on a comic detail page with update badges"]
    fn manual_triggers_first_detail_update_download() {
        let result =
            trigger_first_detail_update_download().expect("trigger detail update download");

        assert!(result.width > 300, "unexpected width: {}", result.width);
        assert!(result.height > 200, "unexpected height: {}", result.height);
        println!(
            "badge {:?}, clicked {:?}, scrolled {}, remaining detail badges: {:?}",
            result.badge, result.clicked, result.scroll_attempts, result.remaining_badges
        );
    }

    #[test]
    #[ignore = "requires MangaCon.exe already on favorites page with update badges"]
    fn manual_triggers_next_favorite_update_download() {
        let result =
            trigger_next_favorite_update_download().expect("trigger next favorite update download");

        assert!(
            result.comic.width > 300,
            "unexpected width: {}",
            result.comic.width
        );
        assert!(
            result.download.width > 300,
            "unexpected width: {}",
            result.download.width
        );
        println!(
            "comic badge {:?}, comic clicked {:?}, favorite scrolled {}, detail badge {:?}, detail clicked {:?}, detail scrolled {}, remaining detail badges: {:?}",
            result.comic.badge,
            result.comic.clicked,
            result.comic.scroll_attempts,
            result.download.badge,
            result.download.clicked,
            result.download.scroll_attempts,
            result.download.remaining_badges
        );
    }

    #[test]
    #[ignore = "requires MangaCon.exe already on a comic detail page"]
    fn manual_returns_to_favorites_from_detail() {
        let result = return_to_favorites_from_detail().expect("return to favorites");

        assert!(result.width > 300, "unexpected width: {}", result.width);
        assert!(result.height > 200, "unexpected height: {}", result.height);
        println!(
            "clicked back {:?}, scanned {}x{}, favorites badges: {:?}",
            result.clicked, result.width, result.height, result.badges
        );
    }

    #[test]
    #[ignore = "requires MangaCon.exe already on favorites page with update badges"]
    fn manual_triggers_favorite_update_batch() {
        let result = trigger_favorite_update_batch(Some(3)).expect("trigger favorite update batch");

        assert!(result.requested_limit <= 3);
        println!(
            "batch processed {}/{}, stopped {:?}",
            result.processed, result.requested_limit, result.stopped_reason
        );
        for (index, item) in result.items.iter().enumerate() {
            println!(
                "#{index}: comic {:?} -> detail {:?}, favorite scrolled {}, detail scrolled {}",
                item.comic.clicked,
                item.download.clicked,
                item.comic.scroll_attempts,
                item.download.scroll_attempts
            );
        }
    }
}
