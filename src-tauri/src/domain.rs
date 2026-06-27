use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comic {
    pub id: String,
    pub name: String,
    pub location: String,
    pub source_uri: String,
    pub source_scheme: Option<String>,
    pub source_domain: Option<String>,
    pub tags: Vec<String>,
    pub local_path: Option<PathBuf>,
    pub chapter_count: usize,
    pub image_count: usize,
    pub read_progress_page: u32,
    pub scan_status: ScanStatus,
}

impl Comic {
    pub fn from_mangacon_favorite(
        name: impl Into<String>,
        location: impl Into<String>,
        uri: impl Into<String>,
        domain: Option<&str>,
        tags: Vec<String>,
    ) -> Self {
        let source_uri = uri.into();
        let source_scheme = source_scheme(&source_uri);

        Self {
            id: source_uri.clone(),
            name: name.into(),
            location: location.into(),
            source_uri,
            source_scheme,
            source_domain: domain.map(str::to_string),
            tags,
            local_path: None,
            chapter_count: 0,
            image_count: 0,
            read_progress_page: 0,
            scan_status: ScanStatus::Pending,
        }
    }

    pub fn title(&self) -> &str {
        if self.location.is_empty() {
            &self.name
        } else {
            &self.location
        }
    }
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
}

fn source_scheme(uri: &str) -> Option<String> {
    uri.split_once(':')
        .map(|(scheme, _)| scheme.trim())
        .filter(|scheme| !scheme.is_empty())
        .map(str::to_string)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Favorite {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub source_url: Option<String>,
    pub tags: Vec<String>,
    pub favorited_at: Option<String>,
}

impl Favorite {
    pub fn new(title: impl Into<String>, source_url: Option<&str>) -> Self {
        let title = title.into();
        let source_url = source_url.map(str::to_string);
        let id = match &source_url {
            Some(url) => format!("{title}|{url}"),
            None => title.clone(),
        };

        Self {
            id,
            title,
            author: None,
            source_url,
            tags: Vec::new(),
            favorited_at: None,
        }
    }

    pub fn stable_id(&self) -> String {
        match &self.source_url {
            Some(url) => format!("{}|{}", self.title, url),
            None => self.title.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalManga {
    pub title: String,
    pub directory: PathBuf,
    pub chapter_count: usize,
    pub image_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteRecord {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub source_url: Option<String>,
    pub source_uri: String,
    pub source_scheme: Option<String>,
    pub source_domain: Option<String>,
    pub tags: Vec<String>,
    pub favorited_at: Option<String>,
    pub local_match: Option<LocalManga>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub imported: usize,
    pub matched: usize,
    pub favorites: Vec<Comic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comic_identity_uses_real_mangacon_uri_and_scheme() {
        let comic = Comic::from_mangacon_favorite(
            "渴盼已久的惡役千金",
            "渴盼已久的惡役千金",
            "cp:kepanyijiudeeyiqianjinlastbossdeshentizhongyudaosh",
            Some("www.2025copy.com"),
            vec!["羽田遼亮".to_string()],
        );

        assert_eq!(
            comic.id,
            "cp:kepanyijiudeeyiqianjinlastbossdeshentizhongyudaosh"
        );
        assert_eq!(
            comic.source_uri,
            "cp:kepanyijiudeeyiqianjinlastbossdeshentizhongyudaosh"
        );
        assert_eq!(comic.source_scheme.as_deref(), Some("cp"));
        assert_eq!(comic.source_domain.as_deref(), Some("www.2025copy.com"));
        assert_eq!(comic.read_progress_page, 0);
        assert_eq!(comic.scan_status, ScanStatus::Pending);
    }

    #[test]
    fn comic_serializes_with_frontend_friendly_field_names() {
        let comic = Comic::from_mangacon_favorite(
            "若世界處於黑夜",
            "若世界處於黑夜",
            "cp:ruoshijiechuyuheiye",
            None,
            vec!["むちまろ".to_string()],
        );

        let value = serde_json::to_value(&comic).expect("serialize comic");

        assert_eq!(value["sourceUri"], "cp:ruoshijiechuyuheiye");
        assert_eq!(value["sourceScheme"], "cp");
        assert_eq!(value["readProgressPage"], 0);
        assert_eq!(value["scanStatus"], "pending");
        assert!(value.get("source_uri").is_none());
    }
}
