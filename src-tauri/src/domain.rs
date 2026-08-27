use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanStatus {
    Pending,
    Missing,
    Matched,
    Imported,
    Error,
}

impl ScanStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Missing => "missing",
            Self::Matched => "matched",
            Self::Imported => "imported",
            Self::Error => "error",
        }
    }

    pub fn from_db_value(value: &str) -> Self {
        match value {
            "missing" => Self::Missing,
            "matched" => Self::Matched,
            "imported" => Self::Imported,
            "error" => Self::Error,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChapterKind {
    Regular,
    Volume,
    MachineTranslation,
    Other,
}

impl ChapterKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Volume => "volume",
            Self::MachineTranslation => "machine_translation",
            Self::Other => "other",
        }
    }

    pub fn from_db_value(value: &str) -> Self {
        match value {
            "regular" => Self::Regular,
            "volume" => Self::Volume,
            "machine_translation" => Self::MachineTranslation,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ReadingDirection {
    #[default]
    Ltr,
    Rtl,
}

impl ReadingDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }

    pub fn from_db_value(value: &str) -> Self {
        match value {
            "rtl" => Self::Rtl,
            _ => Self::Ltr,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum FitMode {
    Width,
    Height,
    #[default]
    Contain,
    Original,
}

impl FitMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Width => "width",
            Self::Height => "height",
            Self::Contain => "contain",
            Self::Original => "original",
        }
    }

    pub fn from_db_value(value: &str) -> Self {
        match value {
            "width" => Self::Width,
            "height" => Self::Height,
            "original" => Self::Original,
            _ => Self::Contain,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ReadMode {
    #[default]
    Page,
    Scroll,
    Spread,
}

impl ReadMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Scroll => "scroll",
            Self::Spread => "spread",
        }
    }

    pub fn from_db_value(value: &str) -> Self {
        match value {
            "scroll" => Self::Scroll,
            "spread" => Self::Spread,
            _ => Self::Page,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comic {
    pub id: String,
    pub name: String,
    pub location: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub local_path: Option<PathBuf>,
    pub cover_path: Option<PathBuf>,
    pub chapter_count: usize,
    pub image_count: usize,
    pub latest_chapter_title: Option<String>,
    pub read_progress_page: u32,
    pub last_read_chapter_id: Option<String>,
    pub last_read_chapter_title: Option<String>,
    pub last_read_at: Option<String>,
    pub last_read_chapter_ordinal: Option<f32>,
    pub last_read_chapter_pages: u32,
    pub scan_status: ScanStatus,
    pub favorited: bool,
    pub reading_direction: ReadingDirection,
    pub fit_mode: FitMode,
    pub read_mode: ReadMode,
    #[serde(default)]
    pub shelf_updated_at: Option<String>,
    #[serde(default)]
    pub shelf_update_note: Option<String>,
}

impl Comic {
    pub fn from_local_directory(title: impl Into<String>, directory: PathBuf) -> Self {
        let title = title.into();
        let id = local_comic_id(&directory);
        Self {
            id,
            name: title.clone(),
            location: title,
            author: None,
            tags: Vec::new(),
            local_path: Some(directory),
            cover_path: None,
            chapter_count: 0,
            image_count: 0,
            latest_chapter_title: None,
            read_progress_page: 0,
            last_read_chapter_id: None,
            last_read_chapter_title: None,
            last_read_at: None,
            last_read_chapter_ordinal: None,
            last_read_chapter_pages: 0,
            scan_status: ScanStatus::Matched,
            favorited: false,
            reading_direction: ReadingDirection::Ltr,
            fit_mode: FitMode::Contain,
            read_mode: ReadMode::Page,
            shelf_updated_at: None,
            shelf_update_note: None,
        }
    }

    pub fn title(&self) -> &str {
        if self.name.is_empty() {
            &self.location
        } else {
            &self.name
        }
    }
}

pub fn local_comic_id(path: &Path) -> String {
    let normalized = normalize_path_key(path);
    format!("local:{normalized}")
}

pub fn normalize_path_key(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

pub fn same_path(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        normalize_path_key(left) == normalize_path_key(right)
    } else {
        left == right
    }
}

pub fn path_is_under(child: &Path, parent: &Path) -> bool {
    let child_key = normalize_path_key(child);
    let parent_key = normalize_path_key(parent);
    child_key == parent_key
        || child_key.starts_with(&format!("{parent_key}/"))
}

pub fn chapter_title_from_id(comic_id: &str, chapter_id: Option<&str>) -> Option<String> {
    let chapter_id = chapter_id?;
    let prefix = format!("{comic_id}::");
    chapter_id
        .strip_prefix(&prefix)
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: String,
    pub comic_id: String,
    pub title: String,
    pub path: PathBuf,
    pub ordinal: Option<f32>,
    pub page_count: usize,
    pub read_progress_page: u32,
    pub special_kind: ChapterKind,
    #[serde(default)]
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalManga {
    pub title: String,
    pub directory: PathBuf,
    pub chapter_count: usize,
    pub image_count: usize,
    #[serde(default)]
    pub cheap_signature: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanLibraryResult {
    pub scanned: usize,
    pub added: usize,
    pub updated: usize,
    #[serde(default)]
    pub unchanged: usize,
    pub missing: usize,
    #[serde(default)]
    pub failed: usize,
    #[serde(default)]
    pub failed_items: Vec<ScanFailure>,
    pub database_path: String,
    pub bookshelf_root: String,
    pub comics: Vec<Comic>,
    #[serde(default)]
    pub baseline_completed: bool,
    #[serde(default)]
    pub established_baseline: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanFailure {
    pub title: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub scanned: usize,
    pub total: usize,
    pub current_title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractProgress {
    pub current: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub bytes: u64,
    pub folders: usize,
    pub freed_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadLibraryResult {
    pub database_path: String,
    pub bookshelf_root: String,
    pub comics: Vec<Comic>,
    #[serde(default)]
    pub baseline_completed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_comic_id_normalizes_windows_paths() {
        let left = local_comic_id(Path::new(r"E:\书架\孤独摇滚"));
        let right = local_comic_id(Path::new(r"e:/书架/孤独摇滚/"));
        assert_eq!(left, right);
        assert!(left.starts_with("local:"));
    }

    #[test]
    fn comic_serializes_with_frontend_friendly_field_names() {
        let comic = Comic::from_local_directory("若世界處於黑夜", PathBuf::from(r"E:\书架\若世界處於黑夜"));
        let value = serde_json::to_value(&comic).expect("serialize comic");

        assert_eq!(value["name"], "若世界處於黑夜");
        assert_eq!(value["readProgressPage"], 0);
        assert_eq!(value["scanStatus"], "matched");
        assert_eq!(value["favorited"], false);
        assert_eq!(value["readingDirection"], "ltr");
        assert_eq!(value["fitMode"], "contain");
        assert!(value.get("source_uri").is_none());
        assert!(value.get("sourceUri").is_none());
        assert_eq!(value["lastReadChapterTitle"], serde_json::Value::Null);
    }

    #[test]
    fn chapter_title_strips_comic_id_prefix() {
        let comic_id = r"local:e:/书架/孤独摇滚";
        let chapter_id = format!("{comic_id}::第01话");
        assert_eq!(
            chapter_title_from_id(comic_id, Some(&chapter_id)).as_deref(),
            Some("第01话")
        );
        assert_eq!(chapter_title_from_id(comic_id, Some("unrelated")), None);
    }
}
