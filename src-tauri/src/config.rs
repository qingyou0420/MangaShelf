use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub mangacon_exe: PathBuf,
    pub mangacon_favorites_json: PathBuf,
    pub mangacon_database: PathBuf,
    pub bookshelf_root: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mangacon_exe: PathBuf::from("E:\\漫画控\\MangaCon.exe"),
            mangacon_favorites_json: PathBuf::from("E:\\漫画控\\20260528184624.mc3db.json"),
            mangacon_database: PathBuf::from(
                "C:\\Users\\Administrator\\AppData\\Local\\MangaCon3\\MangaCon.dat",
            ),
            bookshelf_root: PathBuf::from("E:\\书架"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_approved_paths() {
        let config = AppConfig::default();

        assert_eq!(
            config.mangacon_exe.display().to_string(),
            "E:\\漫画控\\MangaCon.exe"
        );
        assert_eq!(
            config.mangacon_favorites_json.display().to_string(),
            "E:\\漫画控\\20260528184624.mc3db.json"
        );
        assert_eq!(
            config.mangacon_database.display().to_string(),
            "C:\\Users\\Administrator\\AppData\\Local\\MangaCon3\\MangaCon.dat"
        );
        assert_eq!(config.bookshelf_root.display().to_string(), "E:\\书架");
    }
}
