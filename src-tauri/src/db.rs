use crate::domain::{
    chapter_title_from_id, Chapter, ChapterKind, Comic, FitMode, ReadMode, ReadingDirection,
    ScanStatus,
};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    collections::HashMap,
    path::Path,
    time::Duration,
};

pub struct LibraryDatabase {
    connection: Connection,
}

impl LibraryDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let _ = connection.pragma_update(None, "journal_mode", "WAL");
        connection.busy_timeout(Duration::from_millis(5_000))?;
        Ok(Self { connection })
    }

    pub fn migrate(&self) -> Result<()> {
        const SCHEMA_VERSION: i32 = 4;
        let version: i32 = self
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version >= SCHEMA_VERSION {
            return Ok(());
        }
        self.connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS comics (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                location TEXT NOT NULL,
                source_uri TEXT NOT NULL,
                source_scheme TEXT,
                source_domain TEXT,
                local_path TEXT,
                cover_path TEXT,
                chapter_count INTEGER NOT NULL DEFAULT 0,
                image_count INTEGER NOT NULL DEFAULT 0,
                latest_chapter_title TEXT,
                local_fingerprint TEXT,
                read_progress_page INTEGER NOT NULL DEFAULT 0,
                scan_status TEXT NOT NULL DEFAULT 'pending',
                has_update INTEGER NOT NULL DEFAULT 0,
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
            "#,
        )?;
        self.ensure_comics_column("cover_path", "TEXT")?;
        self.ensure_comics_column("latest_chapter_title", "TEXT")?;
        self.ensure_comics_column("local_fingerprint", "TEXT")?;
        self.ensure_comics_column("has_update", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_comics_column("author", "TEXT")?;
        self.ensure_comics_column("favorited", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_comics_column("last_read_chapter_id", "TEXT")?;
        self.ensure_comics_column("last_read_at", "TEXT")?;
        self.ensure_comics_column("reading_direction", "TEXT NOT NULL DEFAULT 'ltr'")?;
        self.ensure_comics_column("fit_mode", "TEXT NOT NULL DEFAULT 'contain'")?;
        self.ensure_comics_column("last_read_chapter_title", "TEXT")?;
        self.ensure_comics_column("last_read_chapter_ordinal", "REAL")?;
        self.ensure_comics_column("last_read_chapter_pages", "INTEGER NOT NULL DEFAULT 0")?;
        self.ensure_comics_column("read_mode", "TEXT NOT NULL DEFAULT 'page'")?;
        self.ensure_chapters_column("fingerprint", "TEXT")?;
        self.ensure_comics_column("cheap_signature", "TEXT")?;
        self.ensure_comics_column("shelf_updated_at", "TEXT")?;
        self.ensure_comics_column("shelf_update_note", "TEXT")?;
        self.connection.execute_batch(
            r#"
            CREATE INDEX IF NOT EXISTS idx_chapters_comic_id
            ON chapters(comic_id, ordinal);
            PRAGMA user_version = 4;
            "#,
        )?;
        Ok(())
    }

    pub fn now_stamp(&self) -> String {
        self.connection
            .query_row(
                "SELECT strftime('%Y-%m-%d %H:%M:%S','now','localtime')",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "1970-01-01 00:00:00".to_string())
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
            SELECT id, name, location, local_path, cover_path, chapter_count, image_count,
                   latest_chapter_title, read_progress_page, scan_status, author, favorited,
                   last_read_chapter_id, last_read_at, reading_direction, fit_mode,
                   last_read_chapter_title, last_read_chapter_ordinal, last_read_chapter_pages,
                   read_mode, shelf_updated_at, shelf_update_note
            FROM comics
            ORDER BY location
            "#,
        )?;

        let mut comics = statement
            .query_map([], map_comic_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut tags = self.all_tags()?;
        for comic in &mut comics {
            comic.tags = tags.remove(&comic.id).unwrap_or_default();
            fill_chapter_title(comic);
        }
        Ok(comics)
    }

    pub fn get_comic(&self, comic_id: &str) -> Result<Option<Comic>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT id, name, location, local_path, cover_path, chapter_count, image_count,
                   latest_chapter_title, read_progress_page, scan_status, author, favorited,
                   last_read_chapter_id, last_read_at, reading_direction, fit_mode,
                   last_read_chapter_title, last_read_chapter_ordinal, last_read_chapter_pages,
                   read_mode, shelf_updated_at, shelf_update_note
            FROM comics
            WHERE id = ?1
            "#,
        )?;
        let mut rows = statement.query(params![comic_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let mut comic = map_comic_row(row)?;
        comic.tags = self.tags_for_comic(&comic.id).unwrap_or_default();
        fill_chapter_title(&mut comic);
        Ok(Some(comic))
    }

    pub fn commit_scanned_comic(
        &self,
        comic: &Comic,
        chapters: Option<&[Chapter]>,
        fingerprint: Option<&str>,
    ) -> Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        upsert_comic_on(&tx, comic)?;
        if let Some(chapters) = chapters {
            replace_chapters_on(&tx, &comic.id, chapters)?;
        }
        update_fingerprint_on(&tx, &comic.id, fingerprint)?;
        tx.commit()?;
        Ok(())
    }

    pub fn upsert_chapter(&self, chapter: &Chapter) -> Result<()> {
        upsert_chapter_on(&self.connection, chapter)
    }

    pub fn replace_chapters_for_comic(&self, comic_id: &str, chapters: &[Chapter]) -> Result<()> {
        let tx = self.connection.unchecked_transaction()?;
        replace_chapters_on(&tx, comic_id, chapters)?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_chapters_for_comic(&self, comic_id: &str) -> Result<Vec<Chapter>> {
        let mut statement = self.connection.prepare(
            r#"
            SELECT id, comic_id, title, path, ordinal, page_count, read_progress_page, special_kind,
                   fingerprint
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
                fingerprint: row.get(8)?,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn local_fingerprint_for_comic(&self, comic_id: &str) -> Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT local_fingerprint FROM comics WHERE id = ?1",
                params![comic_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map(|value| value.flatten())
            .map_err(Into::into)
    }

    pub fn update_local_fingerprint_for_comic(
        &self,
        comic_id: &str,
        fingerprint: Option<&str>,
    ) -> Result<()> {
        update_fingerprint_on(&self.connection, comic_id, fingerprint)
    }

    pub fn cheap_signature_for_comic(&self, comic_id: &str) -> Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT cheap_signature FROM comics WHERE id = ?1",
                params![comic_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map(|value| value.flatten())
            .map_err(Into::into)
    }

    pub fn update_cheap_signature(&self, comic_id: &str, signature: Option<&str>) -> Result<()> {
        self.connection.execute(
            "UPDATE comics SET cheap_signature = ?1 WHERE id = ?2",
            params![signature, comic_id],
        )?;
        Ok(())
    }

    pub fn clear_read_progress(&self, comic_id: &str) -> Result<Option<Comic>> {
        self.connection.execute(
            "UPDATE chapters SET read_progress_page = 0 WHERE comic_id = ?1",
            params![comic_id],
        )?;
        self.connection.execute(
            r#"
            UPDATE comics
            SET read_progress_page = 0,
                last_read_chapter_id = NULL,
                last_read_chapter_title = NULL,
                last_read_chapter_ordinal = NULL,
                last_read_chapter_pages = 0,
                last_read_at = NULL,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?1
            "#,
            params![comic_id],
        )?;
        self.get_comic(comic_id)
    }

    pub fn save_read_progress(
        &self,
        comic_id: &str,
        chapter_id: &str,
        page: u32,
    ) -> Result<Option<Comic>> {
        let tx = self.connection.unchecked_transaction()?;
        tx.execute(
            "UPDATE chapters SET read_progress_page = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![page as i64, chapter_id],
        )?;
        let title: Option<String> = tx
            .query_row(
                "SELECT title FROM chapters WHERE id = ?1",
                params![chapter_id],
                |row| row.get(0),
            )
            .optional()?;
        let title = title.or_else(|| chapter_title_from_id(comic_id, Some(chapter_id)));
        let (ordinal, chapter_pages): (Option<f64>, i64) = tx
            .query_row(
                "SELECT ordinal, page_count FROM chapters WHERE id = ?1",
                params![chapter_id],
                |row| Ok((row.get(0)?, row.get::<_, i64>(1).unwrap_or(0))),
            )
            .optional()?
            .unwrap_or((None, 0));
        tx.execute(
            r#"
            UPDATE comics
            SET read_progress_page = ?1,
                last_read_chapter_id = ?2,
                last_read_chapter_title = ?3,
                last_read_chapter_ordinal = ?4,
                last_read_chapter_pages = ?5,
                last_read_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?6
            "#,
            params![
                page as i64,
                chapter_id,
                title,
                ordinal,
                chapter_pages,
                comic_id
            ],
        )?;
        tx.commit()?;
        self.get_comic(comic_id)
    }

    pub fn update_comic_metadata(
        &self,
        comic_id: &str,
        name: Option<&str>,
        author: Option<&str>,
        tags: Option<&[String]>,
    ) -> Result<Option<Comic>> {
        if let Some(name) = name {
            self.connection.execute(
                "UPDATE comics SET name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![name, comic_id],
            )?;
        }
        if let Some(author) = author {
            let stored = if author.trim().is_empty() {
                None
            } else {
                Some(author.trim())
            };
            self.connection.execute(
                "UPDATE comics SET author = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![stored, comic_id],
            )?;
        }
        if let Some(tags) = tags {
            self.connection.execute(
                "DELETE FROM comic_tags WHERE comic_id = ?1",
                params![comic_id],
            )?;
            for tag in tags {
                let tag = tag.trim();
                if tag.is_empty() {
                    continue;
                }
                self.connection.execute(
                    "INSERT OR IGNORE INTO comic_tags (comic_id, tag) VALUES (?1, ?2)",
                    params![comic_id, tag],
                )?;
            }
        }
        self.get_comic(comic_id)
    }

    pub fn set_comic_favorite(&self, comic_id: &str, favorited: bool) -> Result<Option<Comic>> {
        self.connection.execute(
            "UPDATE comics SET favorited = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![i64::from(favorited), comic_id],
        )?;
        self.get_comic(comic_id)
    }

    pub fn set_reader_prefs(
        &self,
        comic_id: &str,
        reading_direction: ReadingDirection,
        fit_mode: FitMode,
        read_mode: ReadMode,
    ) -> Result<Option<Comic>> {
        self.connection.execute(
            r#"
            UPDATE comics
            SET reading_direction = ?1, fit_mode = ?2, read_mode = ?3, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?4
            "#,
            params![
                reading_direction.as_str(),
                fit_mode.as_str(),
                read_mode.as_str(),
                comic_id
            ],
        )?;
        self.get_comic(comic_id)
    }

    pub fn delete_comic(&self, comic_id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM comics WHERE id = ?1", params![comic_id])?;
        Ok(())
    }

    pub fn has_table(&self, table: &str) -> Result<bool> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    fn tags_for_comic(&self, comic_id: &str) -> rusqlite::Result<Vec<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT tag FROM comic_tags WHERE comic_id = ?1 ORDER BY tag")?;
        let rows = statement.query_map(params![comic_id], |row| row.get(0))?;
        rows.collect()
    }

    fn all_tags(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut statement = self
            .connection
            .prepare("SELECT comic_id, tag FROM comic_tags ORDER BY comic_id, tag")?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
        let mut tags: HashMap<String, Vec<String>> = HashMap::new();
        for row in rows {
            let (comic_id, tag) = row?;
            tags.entry(comic_id).or_default().push(tag);
        }
        Ok(tags)
    }

    fn ensure_comics_column(&self, column: &str, definition: &str) -> Result<()> {
        self.ensure_column("comics", column, definition)
    }

    fn ensure_chapters_column(&self, column: &str, definition: &str) -> Result<()> {
        self.ensure_column("chapters", column, definition)
    }

    fn ensure_column(&self, table: &str, column: &str, definition: &str) -> Result<()> {
        let mut statement = self
            .connection
            .prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
        let columns = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        if !columns.iter().any(|existing| existing == column) {
            self.connection.execute(
                &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
                [],
            )?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn count_rows(&self, table: &str) -> Result<i64> {
        let sql = match table {
            "comics" => "SELECT COUNT(*) FROM comics",
            "comic_tags" => "SELECT COUNT(*) FROM comic_tags",
            "chapters" => "SELECT COUNT(*) FROM chapters",
            _ => anyhow::bail!("unsupported table for test count: {table}"),
        };
        Ok(self.connection.query_row(sql, [], |row| row.get(0))?)
    }
}

fn empty_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn path_to_string_lossy(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn upsert_comic_on(connection: &Connection, comic: &Comic) -> Result<()> {
    let local_path = comic
        .local_path
        .as_ref()
        .map(|path| path_to_string_lossy(path.as_path()));
    let cover_path = comic
        .cover_path
        .as_ref()
        .map(|path| path_to_string_lossy(path.as_path()));
    let source_uri = comic.id.clone();
    connection.execute(
        r#"
        INSERT INTO comics (
            id, name, location, source_uri, local_path, cover_path, chapter_count, image_count,
            latest_chapter_title, read_progress_page, scan_status, author, favorited,
            last_read_chapter_id, last_read_chapter_title, last_read_at, reading_direction,
            fit_mode, last_read_chapter_ordinal, last_read_chapter_pages, read_mode,
            shelf_updated_at, shelf_update_note, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, CURRENT_TIMESTAMP)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            location = excluded.location,
            source_uri = excluded.source_uri,
            local_path = excluded.local_path,
            cover_path = excluded.cover_path,
            chapter_count = excluded.chapter_count,
            image_count = excluded.image_count,
            latest_chapter_title = excluded.latest_chapter_title,
            read_progress_page = excluded.read_progress_page,
            scan_status = excluded.scan_status,
            author = excluded.author,
            favorited = excluded.favorited,
            last_read_chapter_id = excluded.last_read_chapter_id,
            last_read_chapter_title = excluded.last_read_chapter_title,
            last_read_at = excluded.last_read_at,
            reading_direction = excluded.reading_direction,
            fit_mode = excluded.fit_mode,
            last_read_chapter_ordinal = excluded.last_read_chapter_ordinal,
            last_read_chapter_pages = excluded.last_read_chapter_pages,
            read_mode = excluded.read_mode,
            shelf_updated_at = COALESCE(excluded.shelf_updated_at, comics.shelf_updated_at),
            shelf_update_note = COALESCE(excluded.shelf_update_note, comics.shelf_update_note),
            updated_at = CURRENT_TIMESTAMP
        "#,
        params![
            comic.id,
            comic.name,
            comic.location,
            source_uri,
            local_path,
            cover_path,
            comic.chapter_count as i64,
            comic.image_count as i64,
            comic.latest_chapter_title,
            comic.read_progress_page as i64,
            comic.scan_status.as_str(),
            comic.author,
            i64::from(comic.favorited),
            comic.last_read_chapter_id,
            comic.last_read_chapter_title,
            comic.last_read_at,
            comic.reading_direction.as_str(),
            comic.fit_mode.as_str(),
            comic.last_read_chapter_ordinal.map(|value| value as f64),
            comic.last_read_chapter_pages as i64,
            comic.read_mode.as_str(),
            comic.shelf_updated_at,
            comic.shelf_update_note,
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

fn replace_chapters_on(
    connection: &Connection,
    comic_id: &str,
    chapters: &[Chapter],
) -> Result<()> {
    connection.execute(
        "DELETE FROM chapters WHERE comic_id = ?1",
        params![comic_id],
    )?;
    for chapter in chapters {
        upsert_chapter_on(connection, chapter)?;
    }
    Ok(())
}

fn upsert_chapter_on(connection: &Connection, chapter: &Chapter) -> Result<()> {
    let path = path_to_string_lossy(&chapter.path);
    connection.execute(
        r#"
        INSERT INTO chapters (
            id, comic_id, title, path, ordinal, page_count, read_progress_page, special_kind,
            fingerprint, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP)
        ON CONFLICT(id) DO UPDATE SET
            comic_id = excluded.comic_id,
            title = excluded.title,
            path = excluded.path,
            ordinal = excluded.ordinal,
            page_count = excluded.page_count,
            read_progress_page = excluded.read_progress_page,
            special_kind = excluded.special_kind,
            fingerprint = excluded.fingerprint,
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
            chapter.fingerprint,
        ],
    )?;
    Ok(())
}

fn update_fingerprint_on(
    connection: &Connection,
    comic_id: &str,
    fingerprint: Option<&str>,
) -> Result<()> {
    connection.execute(
        "UPDATE comics SET local_fingerprint = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![fingerprint, comic_id],
    )?;
    Ok(())
}

fn map_comic_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Comic> {
    let local_path: Option<String> = row.get(3)?;
    let cover_path: Option<String> = row.get(4)?;
    let author: Option<String> = row.get(10)?;
    Ok(Comic {
        id: row.get(0)?,
        name: row.get(1)?,
        location: row.get(2)?,
        author: empty_to_none(author),
        tags: Vec::new(),
        local_path: local_path.map(Into::into),
        cover_path: cover_path.map(Into::into),
        chapter_count: row.get::<_, i64>(5)? as usize,
        image_count: row.get::<_, i64>(6)? as usize,
        latest_chapter_title: row.get(7)?,
        read_progress_page: row.get::<_, i64>(8)? as u32,
        last_read_chapter_id: row.get(12)?,
        last_read_at: row.get(13)?,
        scan_status: ScanStatus::from_db_value(&row.get::<_, String>(9)?),
        favorited: row.get::<_, i64>(11).unwrap_or(0) != 0,
        reading_direction: ReadingDirection::from_db_value(
            &row.get::<_, String>(14).unwrap_or_else(|_| "ltr".into()),
        ),
        fit_mode: FitMode::from_db_value(
            &row.get::<_, String>(15).unwrap_or_else(|_| "contain".into()),
        ),
        last_read_chapter_title: row.get(16)?,
        last_read_chapter_ordinal: row
            .get::<_, Option<f64>>(17)
            .ok()
            .flatten()
            .map(|value| value as f32),
        last_read_chapter_pages: row.get::<_, i64>(18).unwrap_or(0) as u32,
        read_mode: ReadMode::from_db_value(
            &row.get::<_, String>(19).unwrap_or_else(|_| "page".into()),
        ),
        shelf_updated_at: row.get(20).ok().flatten(),
        shelf_update_note: row.get(21).ok().flatten(),
    })
}

fn fill_chapter_title(comic: &mut Comic) {
    if comic.last_read_chapter_title.is_none() {
        comic.last_read_chapter_title =
            chapter_title_from_id(&comic.id, comic.last_read_chapter_id.as_deref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Chapter, ChapterKind, Comic, ScanStatus};
    use std::path::PathBuf;

    fn sample_comic() -> Comic {
        let mut comic = Comic::from_local_directory("若世界處於黑夜", PathBuf::from(r"E:\书架\若世界處於黑夜"));
        comic.tags = vec!["むちまろ".to_string()];
        comic.scan_status = ScanStatus::Matched;
        comic
    }

    #[test]
    fn migrates_new_schema_and_upserts_comics_and_chapters() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db = LibraryDatabase::open(temp.path().join("library.sqlite")).expect("db open");
        db.migrate().expect("migrate");

        for table in ["comics", "comic_tags", "chapters"] {
            assert!(
                db.has_table(table).expect("table lookup"),
                "missing {table}"
            );
        }
        assert!(!db.has_table("automation_runs").expect("legacy table lookup"));

        let comic = sample_comic();
        db.upsert_comic(&comic).expect("upsert comic");
        db.upsert_comic(&comic).expect("idempotent comic upsert");

        let chapter = Chapter {
            id: format!("{}::第01话", comic.id),
            comic_id: comic.id.clone(),
            title: "第01话".to_string(),
            path: PathBuf::from(r"E:\书架\若世界處於黑夜\第01话"),
            ordinal: Some(1.0),
            page_count: 32,
            read_progress_page: 0,
            special_kind: ChapterKind::Regular,
            fingerprint: None,
        };
        db.upsert_chapter(&chapter).expect("upsert chapter");

        let comics = db.list_comics().expect("list comics");
        assert_eq!(comics.len(), 1);
        assert_eq!(comics[0].name, "若世界處於黑夜");
        assert_eq!(comics[0].tags, vec!["むちまろ"]);
        assert!(!comics[0].favorited);

        let chapters = db
            .list_chapters_for_comic(&comic.id)
            .expect("list chapters");
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].special_kind, ChapterKind::Regular);
    }

    #[test]
    fn comic_upsert_is_idempotent() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db = LibraryDatabase::open(temp.path().join("state.sqlite")).expect("db open");
        db.migrate().expect("migrate");

        let comic = sample_comic();
        let chapter = Chapter {
            id: format!("{}::第01话", comic.id),
            comic_id: comic.id.clone(),
            title: "第01话".to_string(),
            path: PathBuf::from(r"E:\书架\若世界處於黑夜\第01话"),
            ordinal: Some(1.0),
            page_count: 12,
            read_progress_page: 0,
            special_kind: ChapterKind::Regular,
            fingerprint: None,
        };

        db.upsert_comic(&comic).expect("first comic upsert");
        db.upsert_comic(&comic).expect("second comic upsert");
        db.upsert_chapter(&chapter).expect("first chapter upsert");
        db.upsert_chapter(&chapter).expect("second chapter upsert");

        assert_eq!(db.count_rows("comic_tags").expect("tag count"), 1);
        assert_eq!(db.count_rows("chapters").expect("chapter count"), 1);
        assert_eq!(db.count_rows("comics").expect("comic count"), 1);
    }

    #[test]
    fn foreign_keys_cascade_tags_and_chapters_when_comic_is_deleted() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db = LibraryDatabase::open(temp.path().join("state.sqlite")).expect("db open");
        db.migrate().expect("migrate");

        let comic = sample_comic();
        let chapter = Chapter {
            id: format!("{}::第01话", comic.id),
            comic_id: comic.id.clone(),
            title: "第01话".to_string(),
            path: PathBuf::from(r"E:\书架\若世界處於黑夜\第01话"),
            ordinal: Some(1.0),
            page_count: 12,
            read_progress_page: 0,
            special_kind: ChapterKind::Regular,
            fingerprint: None,
        };

        db.upsert_comic(&comic).expect("upsert comic");
        db.upsert_chapter(&chapter).expect("upsert chapter");
        db.delete_comic(&comic.id).expect("delete comic");

        assert_eq!(db.count_rows("comic_tags").expect("tag count"), 0);
        assert_eq!(db.count_rows("chapters").expect("chapter count"), 0);
    }

    #[test]
    fn saves_read_progress_and_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db = LibraryDatabase::open(temp.path().join("state.sqlite")).expect("db open");
        db.migrate().expect("migrate");
        let comic = sample_comic();
        db.upsert_comic(&comic).expect("upsert");
        let chapter = Chapter {
            id: format!("{}::第01话", comic.id),
            comic_id: comic.id.clone(),
            title: "第01话".to_string(),
            path: PathBuf::from(r"E:\书架\若世界處於黑夜\第01话"),
            ordinal: Some(1.0),
            page_count: 12,
            read_progress_page: 0,
            special_kind: ChapterKind::Regular,
            fingerprint: None,
        };
        db.upsert_chapter(&chapter).expect("chapter");

        let updated = db
            .save_read_progress(&comic.id, &chapter.id, 7)
            .expect("save")
            .expect("comic");
        assert_eq!(updated.read_progress_page, 7);
        assert_eq!(updated.last_read_chapter_id.as_deref(), Some(chapter.id.as_str()));
        assert!(updated.last_read_at.is_some());

        let favorite = db
            .set_comic_favorite(&comic.id, true)
            .expect("fav")
            .expect("comic");
        assert!(favorite.favorited);

        let named = db
            .update_comic_metadata(
                &comic.id,
                Some("自定义标题"),
                Some("作者"),
                Some(&["标签A".to_string()]),
            )
            .expect("meta")
            .expect("comic");
        assert_eq!(named.name, "自定义标题");
        assert_eq!(named.author.as_deref(), Some("作者"));
        assert_eq!(named.tags, vec!["标签A"]);
        assert_eq!(updated.last_read_chapter_title.as_deref(), Some("第01话"));
    }

    #[test]
    fn opens_with_wal_and_gets_comic_by_id() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db_path = temp.path().join("state.sqlite");
        let db = LibraryDatabase::open(&db_path).expect("db open");
        db.migrate().expect("migrate");
        let mode: String = db
            .connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("journal");
        assert_eq!(mode.to_ascii_lowercase(), "wal");

        let first = sample_comic();
        let mut second = sample_comic();
        second.id = "local:other".into();
        second.name = "另一本".into();
        second.location = "另一本".into();
        db.upsert_comic(&first).expect("first");
        db.upsert_comic(&second).expect("second");

        let fetched = db.get_comic(&first.id).expect("get").expect("comic");
        assert_eq!(fetched.name, first.name);
        assert_eq!(fetched.tags, vec!["むちまろ"]);
        assert!(db.get_comic("missing").expect("missing").is_none());
    }

    #[test]
    fn replace_chapters_is_atomic() {
        let temp = tempfile::tempdir().expect("tempdir");
        let db = LibraryDatabase::open(temp.path().join("state.sqlite")).expect("db open");
        db.migrate().expect("migrate");
        let comic = sample_comic();
        db.upsert_comic(&comic).expect("comic");
        let chapters = vec![
            Chapter {
                id: format!("{}::第01话", comic.id),
                comic_id: comic.id.clone(),
                title: "第01话".into(),
                path: PathBuf::from(r"E:\书架\若世界處於黑夜\第01话"),
                ordinal: Some(1.0),
                page_count: 12,
                read_progress_page: 3,
                special_kind: ChapterKind::Regular,
                fingerprint: None,
            },
            Chapter {
                id: format!("{}::第02话", comic.id),
                comic_id: comic.id.clone(),
                title: "第02话".into(),
                path: PathBuf::from(r"E:\书架\若世界處於黑夜\第02话"),
                ordinal: Some(2.0),
                page_count: 10,
                read_progress_page: 0,
                special_kind: ChapterKind::Regular,
                fingerprint: None,
            },
        ];
        db.replace_chapters_for_comic(&comic.id, &chapters)
            .expect("replace");
        db.replace_chapters_for_comic(&comic.id, &chapters[..1])
            .expect("replace one");
        assert_eq!(db.count_rows("chapters").expect("count"), 1);
        let stored = db.list_chapters_for_comic(&comic.id).expect("list");
        assert_eq!(stored[0].read_progress_page, 3);
    }
}
