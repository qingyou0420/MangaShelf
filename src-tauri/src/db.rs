use crate::domain::{
    Chapter, ChapterKind, Comic, Favorite, FavoriteRecord, LocalManga, ScanStatus,
};
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct CompanionDatabase {
    connection: Connection,
}

impl CompanionDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { connection })
    }

    pub fn migrate(&self) -> Result<()> {
        self.connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS favorites (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                author TEXT,
                source_url TEXT,
                tags_json TEXT NOT NULL DEFAULT '[]',
                favorited_at TEXT,
                local_title TEXT,
                local_directory TEXT,
                local_chapter_count INTEGER,
                local_image_count INTEGER,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS comics (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                location TEXT NOT NULL,
                source_uri TEXT NOT NULL,
                source_scheme TEXT,
                source_domain TEXT,
                local_path TEXT,
                chapter_count INTEGER NOT NULL DEFAULT 0,
                image_count INTEGER NOT NULL DEFAULT 0,
                read_progress_page INTEGER NOT NULL DEFAULT 0,
                scan_status TEXT NOT NULL DEFAULT 'pending',
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS comic_tags (
                comic_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (comic_id, tag),
                FOREIGN KEY (comic_id) REFERENCES comics(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS chapters (
                id TEXT PRIMARY KEY,
                comic_id TEXT NOT NULL,
                title TEXT NOT NULL,
                path TEXT NOT NULL,
                ordinal REAL,
                page_count INTEGER NOT NULL DEFAULT 0,
                read_progress_page INTEGER NOT NULL DEFAULT 0,
                special_kind TEXT NOT NULL DEFAULT 'other',
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (comic_id) REFERENCES comics(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS automation_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                finished_at TEXT,
                message TEXT
            );
            "#,
        )?;
        Ok(())
    }

    pub fn upsert_favorite(
        &self,
        favorite: &Favorite,
        local_match: Option<&LocalManga>,
    ) -> Result<()> {
        let tags_json = serde_json::to_string(&favorite.tags)?;
        let local_title = local_match.map(|local| local.title.as_str());
        let local_directory = local_match.map(|local| path_to_string_lossy(&local.directory));
        let local_chapter_count = local_match.map(|local| local.chapter_count as i64);
        let local_image_count = local_match.map(|local| local.image_count as i64);

        self.connection.execute(
            r#"
            INSERT INTO favorites (
                id, title, author, source_url, tags_json, favorited_at,
                local_title, local_directory, local_chapter_count, local_image_count, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                author = excluded.author,
                source_url = excluded.source_url,
                tags_json = excluded.tags_json,
                favorited_at = excluded.favorited_at,
                local_title = excluded.local_title,
                local_directory = excluded.local_directory,
                local_chapter_count = excluded.local_chapter_count,
                local_image_count = excluded.local_image_count,
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![
                favorite.id,
                favorite.title,
                favorite.author,
                favorite.source_url,
                tags_json,
                favorite.favorited_at,
                local_title,
                local_directory,
                local_chapter_count,
                local_image_count
            ],
        )?;

        Ok(())
    }

    pub fn list_favorites(&self) -> Result<Vec<FavoriteRecord>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT id, title, author, source_url, tags_json, favorited_at,
                   local_title, local_directory, local_chapter_count, local_image_count
            FROM favorites
            ORDER BY title
            "#,
        )?;

        let rows = statement.query_map([], |row| {
            let tags_json: String = row.get(4)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            let local_title: Option<String> = row.get(6)?;
            let local_directory: Option<String> = row.get(7)?;
            let local_chapter_count: Option<i64> = row.get(8)?;
            let local_image_count: Option<i64> = row.get(9)?;
            let local_match = match (
                local_title,
                local_directory,
                local_chapter_count,
                local_image_count,
            ) {
                (Some(title), Some(directory), Some(chapter_count), Some(image_count)) => {
                    Some(LocalManga {
                        title,
                        directory: directory.into(),
                        chapter_count: chapter_count as usize,
                        image_count: image_count as usize,
                    })
                }
                _ => None,
            };

            Ok(FavoriteRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                source_url: row.get(3)?,
                source_uri: row
                    .get::<_, Option<String>>(3)?
                    .unwrap_or_else(|| row.get(0).unwrap_or_default()),
                source_scheme: row
                    .get::<_, Option<String>>(3)?
                    .and_then(|uri| uri.split_once(':').map(|(scheme, _)| scheme.to_string())),
                source_domain: None,
                tags,
                favorited_at: row.get(5)?,
                local_match,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn has_table(&self, table: &str) -> Result<bool> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn upsert_comic(&self, comic: &Comic) -> Result<()> {
        upsert_comic_on(&self.connection, comic)
    }

    pub fn upsert_comics(&mut self, comics: &[Comic]) -> Result<()> {
        let transaction = self.connection.transaction()?;
        for comic in comics {
            upsert_comic_on(&transaction, comic)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn list_comics(&self) -> Result<Vec<Comic>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT id, name, location, source_uri, source_scheme, source_domain, local_path,
                   chapter_count, image_count, read_progress_page, scan_status
            FROM comics
            ORDER BY location
            "#,
        )?;

        let rows = statement.query_map([], |row| {
            let id: String = row.get(0)?;
            let tags = self.tags_for_comic(&id).unwrap_or_default();
            let local_path: Option<String> = row.get(6)?;

            Ok(Comic {
                id,
                name: row.get(1)?,
                location: row.get(2)?,
                source_uri: row.get(3)?,
                source_scheme: row.get(4)?,
                source_domain: row.get(5)?,
                tags,
                local_path: local_path.map(Into::into),
                chapter_count: row.get::<_, i64>(7)? as usize,
                image_count: row.get::<_, i64>(8)? as usize,
                read_progress_page: row.get::<_, i64>(9)? as u32,
                scan_status: ScanStatus::from_db_value(&row.get::<_, String>(10)?),
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn upsert_chapter(&self, chapter: &Chapter) -> Result<()> {
        let path = path_to_string_lossy(&chapter.path);
        self.connection.execute(
            r#"
            INSERT INTO chapters (
                id, comic_id, title, path, ordinal, page_count, read_progress_page, special_kind, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                comic_id = excluded.comic_id,
                title = excluded.title,
                path = excluded.path,
                ordinal = excluded.ordinal,
                page_count = excluded.page_count,
                read_progress_page = excluded.read_progress_page,
                special_kind = excluded.special_kind,
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![
                chapter.id,
                chapter.comic_id,
                chapter.title,
                path,
                chapter.ordinal.map(|value| value as f64),
                chapter.page_count as i64,
                chapter.read_progress_page as i64,
                chapter.special_kind.as_str(),
            ],
        )?;
        Ok(())
    }

    pub fn list_chapters_for_comic(&self, comic_id: &str) -> Result<Vec<Chapter>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT id, comic_id, title, path, ordinal, page_count, read_progress_page, special_kind
            FROM chapters
            WHERE comic_id = ?1
            ORDER BY ordinal, title
            "#,
        )?;

        let rows = statement.query_map(params![comic_id], |row| {
            Ok(Chapter {
                id: row.get(0)?,
                comic_id: row.get(1)?,
                title: row.get(2)?,
                path: row.get::<_, String>(3)?.into(),
                ordinal: row.get::<_, Option<f64>>(4)?.map(|value| value as f32),
                page_count: row.get::<_, i64>(5)? as usize,
                read_progress_page: row.get::<_, i64>(6)? as u32,
                special_kind: ChapterKind::from_db_value(&row.get::<_, String>(7)?),
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn tags_for_comic(&self, comic_id: &str) -> rusqlite::Result<Vec<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT tag FROM comic_tags WHERE comic_id = ?1 ORDER BY tag")?;
        let rows = statement.query_map(params![comic_id], |row| row.get(0))?;
        rows.collect()
    }

    #[cfg(test)]
    fn count_rows(&self, table: &str) -> Result<i64> {
        let sql = match table {
            "comics" => "SELECT COUNT(*) FROM comics",
            "comic_tags" => "SELECT COUNT(*) FROM comic_tags",
            "chapters" => "SELECT COUNT(*) FROM chapters",
            "automation_runs" => "SELECT COUNT(*) FROM automation_runs",
            "favorites" => "SELECT COUNT(*) FROM favorites",
            _ => anyhow::bail!("unsupported table for test count: {table}"),
        };
        Ok(self.connection.query_row(sql, [], |row| row.get(0))?)
    }

    #[cfg(test)]
    fn delete_comic(&self, comic_id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM comics WHERE id = ?1", params![comic_id])?;
        Ok(())
    }
}

fn path_to_string_lossy(path: &Path) -> String {
    // Windows MangaCon/book shelf paths may contain Unicode. SQLite stores UTF-8 text,
    // so use lossy conversion at this boundary instead of scattering display() calls.
    path.to_string_lossy().into_owned()
}

fn upsert_comic_on(connection: &Connection, comic: &Comic) -> Result<()> {
    let local_path = comic
        .local_path
        .as_ref()
        .map(|path| path_to_string_lossy(path.as_path()));
    connection.execute(
        r#"
        INSERT INTO comics (
            id, name, location, source_uri, source_scheme, source_domain, local_path,
            chapter_count, image_count, read_progress_page, scan_status, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, CURRENT_TIMESTAMP)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            location = excluded.location,
            source_uri = excluded.source_uri,
            source_scheme = excluded.source_scheme,
            source_domain = excluded.source_domain,
            local_path = excluded.local_path,
            chapter_count = excluded.chapter_count,
            image_count = excluded.image_count,
            read_progress_page = excluded.read_progress_page,
            scan_status = excluded.scan_status,
            updated_at = CURRENT_TIMESTAMP
        "#,
        params![
            comic.id,
            comic.name,
            comic.location,
            comic.source_uri,
            comic.source_scheme,
            comic.source_domain,
            local_path,
            comic.chapter_count as i64,
            comic.image_count as i64,
            comic.read_progress_page as i64,
            comic.scan_status.as_str(),
        ],
    )?;

    connection.execute(
        "DELETE FROM comic_tags WHERE comic_id = ?1",
        params![comic.id],
    )?;
    for tag in &comic.tags {
        connection.execute(
            "INSERT OR IGNORE INTO comic_tags (comic_id, tag) VALUES (?1, ?2)",
            params![comic.id, tag],
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Chapter, ChapterKind, Comic, LocalManga, ScanStatus};

    #[test]
    fn migrates_and_upserts_favorites_with_local_matches() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("mangacon-companion.sqlite");
        let db = CompanionDatabase::open(&db_path).expect("db open");
        db.migrate().expect("migrate");

        let favorite = Favorite::new("孤独摇滚", Some("https://mangacon.example/bocchi"));
        let local = LocalManga {
            title: "孤独摇滚".to_string(),
            directory: "E:\\书架\\孤独摇滚".into(),
            chapter_count: 12,
            image_count: 240,
        };

        db.upsert_favorite(&favorite, Some(&local)).expect("upsert");
        db.upsert_favorite(&favorite, Some(&local))
            .expect("idempotent upsert");

        let rows = db.list_favorites().expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "孤独摇滚");
        assert_eq!(rows[0].local_match.as_ref().unwrap().chapter_count, 12);
    }

    #[test]
    fn migrates_new_schema_and_upserts_comics_and_chapters() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("mangacon-companion.sqlite");
        let db = CompanionDatabase::open(&db_path).expect("db open");
        db.migrate().expect("migrate");

        for table in ["comics", "comic_tags", "chapters", "automation_runs"] {
            assert!(
                db.has_table(table).expect("table lookup"),
                "missing {table}"
            );
        }

        let mut comic = Comic::from_mangacon_favorite(
            "若世界處於黑夜",
            "若世界處於黑夜",
            "cp:ruoshijiechuyuheiye",
            None,
            vec!["むちまろ".to_string()],
        );
        comic.scan_status = ScanStatus::Matched;
        comic.local_path = Some("E:\\书架\\若世界處於黑夜".into());

        db.upsert_comic(&comic).expect("upsert comic");
        db.upsert_comic(&comic).expect("idempotent comic upsert");

        let chapter = Chapter {
            id: "cp:ruoshijiechuyuheiye::第01话".to_string(),
            comic_id: comic.id.clone(),
            title: "第01话".to_string(),
            path: "E:\\书架\\若世界處於黑夜\\第01话".into(),
            ordinal: Some(1.0),
            page_count: 32,
            read_progress_page: 0,
            special_kind: ChapterKind::Regular,
        };
        db.upsert_chapter(&chapter).expect("upsert chapter");

        let comics = db.list_comics().expect("list comics");
        assert_eq!(comics.len(), 1);
        assert_eq!(comics[0].source_uri, "cp:ruoshijiechuyuheiye");
        assert_eq!(comics[0].source_scheme.as_deref(), Some("cp"));
        assert_eq!(comics[0].tags, vec!["むちまろ"]);

        let chapters = db
            .list_chapters_for_comic(&comic.id)
            .expect("list chapters");
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].special_kind, ChapterKind::Regular);
    }

    #[test]
    fn comic_upsert_is_idempotent_and_source_uri_is_not_a_global_unique_key() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db = CompanionDatabase::open(temp.path().join("state.sqlite")).expect("db open");
        db.migrate().expect("migrate");

        let comic = Comic::from_mangacon_favorite(
            "若世界處於黑夜",
            "若世界處於黑夜",
            "cp:ruoshijiechuyuheiye",
            None,
            vec!["むちまろ".to_string(), "むちまろ".to_string()],
        );
        let chapter = Chapter {
            id: "cp:ruoshijiechuyuheiye::第01话".to_string(),
            comic_id: comic.id.clone(),
            title: "第01话".to_string(),
            path: "E:\\书架\\若世界處於黑夜\\第01话".into(),
            ordinal: Some(1.0),
            page_count: 12,
            read_progress_page: 0,
            special_kind: ChapterKind::Regular,
        };

        db.upsert_comic(&comic).expect("first comic upsert");
        db.upsert_comic(&comic).expect("second comic upsert");
        db.upsert_chapter(&chapter).expect("first chapter upsert");
        db.upsert_chapter(&chapter).expect("second chapter upsert");

        assert_eq!(db.count_rows("comic_tags").expect("tag count"), 1);
        assert_eq!(db.count_rows("chapters").expect("chapter count"), 1);

        let mut mirror = comic.clone();
        mirror.id = "manual:copy-of-ruoshijie".to_string();
        db.upsert_comic(&mirror)
            .expect("same source_uri under different id should not conflict");
        assert_eq!(db.count_rows("comics").expect("comic count"), 2);
    }

    #[test]
    fn bulk_comic_upsert_imports_many_comics_with_tags() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut db = CompanionDatabase::open(temp.path().join("state.sqlite")).expect("db open");
        db.migrate().expect("migrate");

        let mut first = Comic::from_mangacon_favorite(
            "若世界處於黑夜",
            "若世界處於黑夜",
            "cp:ruoshijiechuyuheiye",
            None,
            vec!["むちまろ".to_string()],
        );
        first.scan_status = ScanStatus::Imported;
        let mut second = Comic::from_mangacon_favorite(
            "虚构推理",
            "虚构推理",
            "mhg:17165",
            None,
            vec!["城平京".to_string(), "片瀬茶柴".to_string()],
        );
        second.scan_status = ScanStatus::Imported;

        db.upsert_comics(&[first, second])
            .expect("bulk comic upsert");

        let comics = db.list_comics().expect("list comics");
        assert_eq!(comics.len(), 2);
        assert!(comics
            .iter()
            .all(|comic| comic.scan_status == ScanStatus::Imported));
        assert_eq!(db.count_rows("comic_tags").expect("tag count"), 3);
    }

    #[test]
    fn foreign_keys_cascade_tags_and_chapters_when_comic_is_deleted() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db = CompanionDatabase::open(temp.path().join("state.sqlite")).expect("db open");
        db.migrate().expect("migrate");

        let comic = Comic::from_mangacon_favorite(
            "若世界處於黑夜",
            "若世界處於黑夜",
            "cp:ruoshijiechuyuheiye",
            None,
            vec!["むちまろ".to_string()],
        );
        let chapter = Chapter {
            id: "cp:ruoshijiechuyuheiye::第01话".to_string(),
            comic_id: comic.id.clone(),
            title: "第01话".to_string(),
            path: "E:\\书架\\若世界處於黑夜\\第01话".into(),
            ordinal: Some(1.0),
            page_count: 12,
            read_progress_page: 0,
            special_kind: ChapterKind::Regular,
        };

        db.upsert_comic(&comic).expect("upsert comic");
        db.upsert_chapter(&chapter).expect("upsert chapter");
        db.delete_comic(&comic.id).expect("delete comic");

        assert_eq!(db.count_rows("comic_tags").expect("tag count"), 0);
        assert_eq!(db.count_rows("chapters").expect("chapter count"), 0);
    }
}
