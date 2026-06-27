use crate::domain::Comic;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::{fs, path::Path};

pub fn import_mangacon_favorites(path: impl AsRef<Path>) -> Result<Vec<Comic>> {
    let raw = fs::read_to_string(path)?;
    import_mangacon_favorites_from_str(&raw)
}

pub fn import_mangacon_favorites_from_str(raw: &str) -> Result<Vec<Comic>> {
    let value: Value = serde_json::from_str(raw)?;
    let items = favorite_items(&value)
        .ok_or_else(|| anyhow!("MangaCon favorites JSON did not contain a favorites array"))?;

    items
        .iter()
        .map(parse_favorite)
        .collect::<Result<Vec<Comic>>>()
}

fn favorite_items(value: &Value) -> Option<&Vec<Value>> {
    if let Some(items) = value.as_array() {
        return Some(items);
    }

    ["favorites", "items", "data", "bookmarks"]
        .iter()
        .find_map(|key| value.get(key)?.as_array())
}

fn parse_favorite(value: &Value) -> Result<Comic> {
    let name = string_field(value, &["name"]).ok_or_else(|| anyhow!("favorite is missing name"))?;
    let location = string_field(value, &["location"]).unwrap_or_else(|| name.clone());
    let uri = string_field(value, &["uri"]).ok_or_else(|| anyhow!("favorite is missing uri"))?;
    let domain = string_field(value, &["domain"]);

    Ok(Comic::from_mangacon_favorite(
        name,
        location,
        uri,
        domain.as_deref(),
        tags_field(value),
    ))
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
              "location": "渴盼已久的惡役千金(Last boss)的身體終於到手了！",
              "name": "渴盼已久的惡役千金(Last boss)的身體終於到手了！",
              "tags": ["羽田遼亮"],
              "uri": "cp:kepanyijiudeeyiqianjinlastbossdeshentizhongyudaosh",
              "domain": "www.2025copy.com"
            }
          ]
        }
        "#;

        let comics = import_mangacon_favorites_from_str(raw).expect("favorites import");

        assert_eq!(comics.len(), 1);
        assert_eq!(
            comics[0].id,
            "cp:kepanyijiudeeyiqianjinlastbossdeshentizhongyudaosh"
        );
        assert_eq!(
            comics[0].source_uri,
            "cp:kepanyijiudeeyiqianjinlastbossdeshentizhongyudaosh"
        );
        assert_eq!(comics[0].source_scheme.as_deref(), Some("cp"));
        assert_eq!(comics[0].source_domain.as_deref(), Some("www.2025copy.com"));
        assert_eq!(comics[0].tags, vec!["羽田遼亮"]);
        assert_eq!(
            comics[0].location,
            "渴盼已久的惡役千金(Last boss)的身體終於到手了！"
        );
    }

    #[test]
    #[ignore = "depends on local MangaCon export at E:\\漫画控\\20260528184624.mc3db.json"]
    fn imports_real_mangacon_file_when_available() {
        let path = std::path::Path::new("E:\\漫画控\\20260528184624.mc3db.json");
        if !path.exists() {
            eprintln!("skipping real MangaCon import test: fixture file is not present");
            return;
        }

        let comics = import_mangacon_favorites(path).expect("real MangaCon import");

        assert_eq!(comics.len(), 447);
        assert!(!comics[0].source_uri.is_empty());
        assert!(comics[0]
            .source_scheme
            .as_deref()
            .is_some_and(|scheme| !scheme.is_empty()));
    }
}
