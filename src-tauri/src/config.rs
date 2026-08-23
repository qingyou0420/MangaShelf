use std::path::{Path, PathBuf};

const DEFAULT_BOOKSHELF: &str = r"E:\书架";
const LIBRARY_DB_NAME: &str = "manga-library.sqlite";
const LEGACY_DB_NAME: &str = "mangacon-companion.sqlite";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub bookshelf_root: PathBuf,
    pub library_database: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        let bookshelf_root = PathBuf::from(DEFAULT_BOOKSHELF);
        Self {
            library_database: resolve_database_path(&bookshelf_root, None),
            bookshelf_root,
        }
    }
}

/// Prefer the new local index name. If only a previous companion index exists,
/// copy it once to `manga-library.sqlite` so progress is kept without silently
/// writing to the old filename.
pub fn resolve_database_path(bookshelf_root: &Path, configured: Option<&Path>) -> PathBuf {
    if let Some(path) = configured {
        if !path.as_os_str().is_empty() {
            if path.exists() {
                return path.to_path_buf();
            }
            let parent = path.parent().unwrap_or(bookshelf_root);
            if file_name_eq(path, LIBRARY_DB_NAME) {
                migrate_legacy_index(parent, path);
            }
            return path.to_path_buf();
        }
    }

    let preferred = bookshelf_root.join(LIBRARY_DB_NAME);
    migrate_legacy_index(bookshelf_root, &preferred);
    preferred
}

fn migrate_legacy_index(dir: &Path, preferred: &Path) {
    if preferred.exists() {
        return;
    }
    let legacy = dir.join(LEGACY_DB_NAME);
    if !legacy.exists() {
        return;
    }
    if let Some(parent) = preferred.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::copy(&legacy, preferred);
}

fn file_name_eq(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn defaults_point_at_local_bookshelf() {
        let config = AppConfig::default();
        assert_eq!(config.bookshelf_root.display().to_string(), r"E:\书架");
        assert!(
            config.library_database.ends_with(LIBRARY_DB_NAME)
                || config.library_database.ends_with(LEGACY_DB_NAME)
        );
    }

    #[test]
    fn resolve_prefers_existing_legacy_index_when_new_file_is_absent() {
        let temp = tempfile::tempdir().expect("tempdir");
        let legacy = temp.path().join(LEGACY_DB_NAME);
        fs::write(&legacy, b"").expect("legacy db");

        let resolved = resolve_database_path(temp.path(), None);
        let preferred = temp.path().join(LIBRARY_DB_NAME);
        assert_eq!(resolved, preferred);
        assert!(preferred.exists());
        assert!(legacy.exists());
    }

    #[test]
    fn resolve_uses_new_name_when_neither_file_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let resolved = resolve_database_path(temp.path(), None);
        assert_eq!(resolved, temp.path().join(LIBRARY_DB_NAME));
    }

    #[test]
    fn resolve_uses_configured_existing_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let custom = temp.path().join("custom.sqlite");
        fs::write(&custom, b"").expect("custom db");
        let resolved = resolve_database_path(temp.path(), Some(&custom));
        assert_eq!(resolved, custom);
    }
}
