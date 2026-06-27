use crate::domain::Favorite;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::{fs, path::Path};

pub fn import_mangacon_favorites(path: impl AsRef<Path>) -> Result<Vec<Favorite>> {
    let raw = fs::read_to_string(path)?;
    import_mangacon_favorites_from_str(&raw)
}

pub fn import_mangacon_favorites_from_str(raw: &str) -> Result<Vec<Favorite>> {
    let value: Value = serde_json::from_str(raw)?;
    let items = favorite_items(&value)
        .ok_or_else(|| anyhow!("MangaCon favorites JSON did not contain a favorites array"))?;

    items
        .iter()
        .map(parse_favorite)
        .collect::<Result<Vec<Favorite>>>()
}

fn favorite_items(value: &Value) -> Option<&Vec<Value>> {
    if let Some(items) = value.as_array() {
        return Some(items);
    }

    ["favorites", "items", "data", "bookmarks"]
        .iter()
        .find_map(|key| value.get(key)?.as_array())
}

fn parse_favorite(value: &Value) -> Result<Favorite> {
    let title = string_field(value, &["title", "name", "comicName", "bookName"])
        .ok_or_else(|| anyhow!("favorite is missing title/name"))?;
    let source_url = string_field(value, &["sourceUrl", "url", "link", "href"]);
    let mut favorite = Favorite::new(title, source_url.as_deref());

    favorite.id =
        string_field(value, &["id", "uuid", "favoriteId"]).unwrap_or_else(|| favorite.stable_id());
    favorite.author = string_field(value, &["author", "artist"]);
    favorite.tags = tags_field(value);
    favorite.favorited_at = string_field(value, &["favoritedAt", "favoriteTime", "createdAt"]);

    Ok(favorite)
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(key)?.as_str())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn tags_field(value: &Value) -> Vec<String> {
    match value.get("tags") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str())
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(str::to_string)
            .collect(),
        Some(Value::String(tags)) => tags
            .split([',', ';', '，', '；'])
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_mangacon_favorites() {
        let raw = r#"
        {
          "favorites": [
            {
              "id": "mc-1",
              "title": "孤独摇滚",
              "author": "はまじあき",
              "url": "https://mangacon.example/bocchi",
              "tags": ["音乐", "日常"],
              "favoriteTime": "2026-05-28T18:46:24Z"
            }
          ]
        }
        "#;

        let favorites = import_mangacon_favorites_from_str(raw).expect("favorites import");

        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].id, "mc-1");
        assert_eq!(favorites[0].title, "孤独摇滚");
        assert_eq!(favorites[0].tags, vec!["音乐", "日常"]);
        assert_eq!(
            favorites[0].source_url.as_deref(),
            Some("https://mangacon.example/bocchi")
        );
    }
}
