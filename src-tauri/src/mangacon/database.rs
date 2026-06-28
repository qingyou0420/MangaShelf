use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedMangaConTask {
    pub manga_id: i64,
    pub volume_id: i64,
    pub manga: String,
    pub uri: String,
    pub volume_key: String,
    pub title: String,
    pub location: String,
    pub extra: String,
    pub order_index: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueMangaConUpdatesResult {
    pub backup_path: String,
    pub total_updates: usize,
    pub queued: usize,
    pub skipped_existing: usize,
    pub tasks: Vec<QueuedMangaConTask>,
}

#[derive(Debug, Clone)]
struct MangaConUpdateCandidate {
    manga_id: i64,
    volume_id: i64,
    manga: String,
    uri: String,
    domain: Option<String>,
    manga_location: String,
    volume_key: String,
    title: String,
}

pub fn queue_all_badged_updates(
    database_path: impl AsRef<Path>,
    max_updates: Option<u32>,
) -> Result<QueueMangaConUpdatesResult> {
    let backup_path = backup_database(database_path.as_ref())?;
    let mut connection = Connection::open(database_path)?;
    let transaction = connection.transaction()?;
    let candidates = list_badged_update_candidates(&transaction)?;
    let mut next_order_index = next_task_order_index(&transaction)?;
    let mut tasks = Vec::new();
    let mut skipped_existing = 0;
    let limit = max_updates
        .map(|value| value as usize)
        .unwrap_or(usize::MAX);

    for candidate in candidates.iter().take(limit) {
        if task_already_exists(&transaction, &candidate.uri, &candidate.volume_key)? {
            skipped_existing += 1;
            continue;
        }

        let location = format!("{}\\{}", candidate.manga_location, candidate.title);
        let extra = serde_json::json!({
            "mid": candidate.manga_id,
            "vid": candidate.volume_id,
        })
        .to_string();
        transaction.execute(
            "INSERT INTO mc3_tasks(mu, domain, vk, location, extra, order_index) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                candidate.uri,
                candidate.domain,
                candidate.volume_key,
                location,
                extra,
                next_order_index,
            ],
        )?;
        tasks.push(QueuedMangaConTask {
            manga_id: candidate.manga_id,
            volume_id: candidate.volume_id,
            manga: candidate.manga.clone(),
            uri: candidate.uri.clone(),
            volume_key: candidate.volume_key.clone(),
            title: candidate.title.clone(),
            location,
            extra,
            order_index: next_order_index,
        });
        next_order_index += 1;
    }

    transaction.commit()?;
    Ok(QueueMangaConUpdatesResult {
        backup_path: backup_path.to_string_lossy().into_owned(),
        total_updates: candidates.len(),
        queued: tasks.len(),
        skipped_existing,
        tasks,
    })
}

fn backup_database(database_path: &Path) -> Result<PathBuf> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let file_name = database_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("MangaCon.dat");
    let backup_path = database_path.with_file_name(format!("{file_name}.companion-backup-{stamp}"));
    fs::copy(database_path, &backup_path)?;
    Ok(backup_path)
}

fn list_badged_update_candidates(connection: &Connection) -> Result<Vec<MangaConUpdateCandidate>> {
    let mut statement = connection.prepare(
        r#"
        SELECT m.rowid, v.rowid, m.name, m.uri, m.domain, m.location, v.key, v.title
        FROM mc3_badges b
        JOIN mc3_mangas m ON m.rowid = b.id
        JOIN mc3_volumes v ON v.mid = m.rowid AND v.status = 1
        WHERE b.category = 1
        ORDER BY m.rowid, v.rowid
        "#,
    )?;
    let rows = statement.query_map([], |row| {
        Ok(MangaConUpdateCandidate {
            manga_id: row.get(0)?,
            volume_id: row.get(1)?,
            manga: row.get(2)?,
            uri: row.get(3)?,
            domain: row.get(4)?,
            manga_location: row.get(5)?,
            volume_key: row.get(6)?,
            title: row.get(7)?,
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn next_task_order_index(connection: &Connection) -> Result<i64> {
    let max_order = connection.query_row("SELECT MAX(order_index) FROM mc3_tasks", [], |row| {
        row.get::<_, Option<i64>>(0)
    })?;
    Ok(max_order.unwrap_or(0) + 1)
}

fn task_already_exists(connection: &Connection, uri: &str, volume_key: &str) -> Result<bool> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM mc3_tasks WHERE mu = ?1 AND vk = ?2",
        params![uri, volume_key],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    fn create_fixture_db() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("MangaCon.dat");
        let connection = Connection::open(&path).expect("open fixture db");
        connection
            .execute_batch(
                r#"
                CREATE TABLE mc3_badges(category INTEGER NOT NULL, id INTEGER NOT NULL, value INTEGER NOT NULL, PRIMARY KEY(category, id)) WITHOUT ROWID;
                CREATE TABLE mc3_mangas(uri TEXT PRIMARY KEY, name TEXT NOT NULL, domain TEXT, groups TEXT, location TEXT NOT NULL, status INTEGER, order_tick INTEGER NOT NULL DEFAULT 0, update_tick INTEGER NOT NULL DEFAULT 0);
                CREATE TABLE mc3_tasks(mu TEXT NOT NULL, domain TEXT, vk TEXT NOT NULL, location TEXT NOT NULL, extra TEXT, errors INTEGER, order_index INTEGER, finished_tick INTEGER);
                CREATE TABLE mc3_volumes(mid INTEGER NOT NULL, key TEXT NOT NULL, title TEXT NOT NULL, status INTEGER, PRIMARY KEY(mid, key));
                "#,
            )
            .expect("schema");
        connection
            .execute(
                "INSERT INTO mc3_mangas(rowid, uri, name, domain, location, status, order_tick, update_tick) VALUES (?1, ?2, ?3, ?4, ?5, 2, 1, 1)",
                params![31, "cp:jianmingyidongdescp", "簡明易懂的SCP", Option::<String>::None, "簡明易懂的SCP"],
            )
            .expect("manga");
        connection
            .execute(
                "INSERT INTO mc3_volumes(rowid, mid, key, title, status) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![37246, 31, "b1b787e2-7252-11f1-98ad-fa163e02432f", "第357话", 1],
            )
            .expect("updated volume");
        connection
            .execute(
                "INSERT INTO mc3_volumes(rowid, mid, key, title, status) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![37247, 31, "already-normal", "第356话", 0],
            )
            .expect("normal volume");
        connection
            .execute(
                "INSERT INTO mc3_badges(category, id, value) VALUES (1, 31, 1)",
                [],
            )
            .expect("badge");
        (temp, path)
    }

    #[test]
    fn queues_status_one_badged_volumes_as_mangacon_tasks() {
        let (_temp, path) = create_fixture_db();

        let result = queue_all_badged_updates(&path, None).expect("queue updates");

        assert_eq!(result.total_updates, 1);
        assert_eq!(result.queued, 1);
        assert_eq!(result.skipped_existing, 0);
        assert!(
            std::path::Path::new(&result.backup_path).exists(),
            "missing backup at {}",
            result.backup_path
        );
        assert_eq!(result.tasks[0].manga, "簡明易懂的SCP");
        assert_eq!(result.tasks[0].title, "第357话");
        assert_eq!(result.tasks[0].location, "簡明易懂的SCP\\第357话");
        assert_eq!(result.tasks[0].extra, r#"{"mid":31,"vid":37246}"#);

        let connection = Connection::open(path).expect("reopen");
        let row = connection
            .query_row(
                "SELECT mu, domain, vk, location, extra, errors, order_index, finished_tick FROM mc3_tasks",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                        row.get::<_, Option<i64>>(6)?,
                        row.get::<_, Option<i64>>(7)?,
                    ))
                },
            )
            .expect("task row");

        assert_eq!(row.0, "cp:jianmingyidongdescp");
        assert_eq!(row.1, None);
        assert_eq!(row.2, "b1b787e2-7252-11f1-98ad-fa163e02432f");
        assert_eq!(row.3, "簡明易懂的SCP\\第357话");
        assert_eq!(row.4, r#"{"mid":31,"vid":37246}"#);
        assert_eq!(row.5, None);
        assert_eq!(row.6, Some(1));
        assert_eq!(row.7, None);
    }

    #[test]
    fn skips_existing_unfinished_tasks_and_respects_limit() {
        let (_temp, path) = create_fixture_db();
        let connection = Connection::open(&path).expect("open");
        connection
            .execute(
                "INSERT INTO mc3_tasks(mu, domain, vk, location, extra, order_index) VALUES (?1, NULL, ?2, ?3, ?4, 9)",
                params![
                    "cp:jianmingyidongdescp",
                    "b1b787e2-7252-11f1-98ad-fa163e02432f",
                    "簡明易懂的SCP\\第357话",
                    r#"{"mid":31,"vid":37246}"#
                ],
            )
            .expect("existing task");
        drop(connection);

        let result = queue_all_badged_updates(&path, Some(1)).expect("queue updates");

        assert_eq!(result.total_updates, 1);
        assert_eq!(result.queued, 0);
        assert_eq!(result.skipped_existing, 1);
        assert!(result.tasks.is_empty());
    }
}
