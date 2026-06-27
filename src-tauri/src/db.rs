use crate::domain::{Favorite, FavoriteRecord, LocalManga};
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct CompanionDatabase {
    connection: Connection,
}

impl CompanionDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            connection: Connection::open(path)?,
        })
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
        let local_directory = local_match.map(|local| local.directory.display().to_string());
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
                tags,
                favorited_at: row.get(5)?,
                local_match,
            })
        })?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Favorite, LocalManga};

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
}
