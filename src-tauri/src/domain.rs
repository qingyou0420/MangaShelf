use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
pub struct LocalManga {
    pub title: String,
    pub directory: PathBuf,
    pub chapter_count: usize,
    pub image_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavoriteRecord {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub source_url: Option<String>,
    pub tags: Vec<String>,
    pub favorited_at: Option<String>,
    pub local_match: Option<LocalManga>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSummary {
    pub imported: usize,
    pub matched: usize,
    pub favorites: Vec<FavoriteRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favorite_identity_uses_stable_title_and_source() {
        let favorite = Favorite::new("孤独摇滚", Some("https://example.test/bocchi"));

        assert_eq!(favorite.stable_id(), "孤独摇滚|https://example.test/bocchi");
    }
}
