use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const CONTINUE_LAST_SESSION_TASKS_KEY: &str = "continue_last_session_tasks";

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
    pub cleared_update_markers: usize,
    pub tasks: Vec<QueuedMangaConTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaConTaskStatus {
    pub total_tasks: usize,
    pub active_tasks: usize,
    pub failed_tasks: usize,
    pub finished_tasks: usize,
    pub total_errors: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequeuedMangaConRepairTask {
    pub task_id: i64,
    pub uri: String,
    pub volume_key: String,
    pub location: String,
    pub errors: i64,
    pub order_index: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairMangaConFailedTasksResult {
    pub backup_path: String,
    pub total_failed: usize,
    pub requeued: usize,
    pub tasks: Vec<RequeuedMangaConRepairTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeMangaConUnfinishedTasksResult {
    pub backup_path: String,
    pub total_unfinished: usize,
    pub resume_configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MangaConMangaCacheRecord {
    pub rowid: i64,
    pub uri: String,
    pub cover_path: Option<PathBuf>,
    pub has_update: bool,
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

#[derive(Debug, Clone)]
struct FailedMangaConTaskCandidate {
    task_id: i64,
    uri: String,
    volume_key: String,
    location: String,
    errors: i64,
}

pub fn queue_all_badged_updates(
    database_path: impl AsRef<Path>,
    max_updates: Option<u32>,
) -> Result<QueueMangaConUpdatesResult> {
    queue_updates_including_local_gaps(database_path, None, max_updates)
}

pub fn queue_updates_including_local_gaps(
    database_path: impl AsRef<Path>,
    companion_database_path: Option<&Path>,
    max_updates: Option<u32>,
) -> Result<QueueMangaConUpdatesResult> {
    let backup_path = backup_database(database_path.as_ref())?;
    let mut connection = Connection::open(database_path)?;
    let transaction = connection.transaction()?;
    let mut candidates = list_badged_update_candidates(&transaction)?;
    if let Some(companion_database_path) = companion_database_path {
        let gap_candidates =
            list_missing_local_chapter_candidates(&transaction, companion_database_path)?;
        append_unique_candidates(&mut candidates, gap_candidates);
    }
    let mut next_order_index = next_task_order_index(&transaction)?;
    let mut tasks = Vec::new();
    let mut skipped_existing = 0;
    let mut cleared_update_markers = 0;
    let mut affected_manga_ids = BTreeSet::new();
    let limit = max_updates
        .map(|value| value as usize)
        .unwrap_or(usize::MAX);

    for candidate in candidates.iter().take(limit) {
        if task_already_exists(&transaction, &candidate.uri, &candidate.volume_key)? {
            skipped_existing += 1;
        } else {
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

        cleared_update_markers += clear_volume_update_marker(&transaction, candidate.volume_id)?;
        affected_manga_ids.insert(candidate.manga_id);
    }

    for manga_id in affected_manga_ids {
        sync_badge_value(&transaction, manga_id)?;
    }
    if !tasks.is_empty() || skipped_existing > 0 {
        enable_continue_last_session_tasks(&transaction)?;
    }

    transaction.commit()?;
    Ok(QueueMangaConUpdatesResult {
        backup_path: backup_path.to_string_lossy().into_owned(),
        total_updates: candidates.len(),
        queued: tasks.len(),
        skipped_existing,
        cleared_update_markers,
        tasks,
    })
}

pub fn read_task_status(database_path: impl AsRef<Path>) -> Result<MangaConTaskStatus> {
    let connection = Connection::open(database_path)?;
    let (total_tasks, active_tasks, failed_tasks, finished_tasks, total_errors) = connection
        .query_row(
            r#"
            SELECT
                COUNT(*),
                SUM(CASE WHEN finished_tick IS NULL THEN 1 ELSE 0 END),
                SUM(CASE WHEN COALESCE(errors, 0) > 0 THEN 1 ELSE 0 END),
                SUM(CASE WHEN finished_tick IS NOT NULL THEN 1 ELSE 0 END),
                SUM(COALESCE(errors, 0))
            FROM mc3_tasks
            "#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                    row.get::<_, Option<i64>>(4)?.unwrap_or(0),
                ))
            },
        )?;

    Ok(MangaConTaskStatus {
        total_tasks: total_tasks as usize,
        active_tasks: active_tasks as usize,
        failed_tasks: failed_tasks as usize,
        finished_tasks: finished_tasks as usize,
        total_errors: total_errors as usize,
    })
}

pub fn requeue_failed_tasks_for_repair(
    database_path: impl AsRef<Path>,
    max_tasks: Option<u32>,
) -> Result<RepairMangaConFailedTasksResult> {
    let backup_path = backup_database(database_path.as_ref())?;
    let mut connection = Connection::open(database_path)?;
    let transaction = connection.transaction()?;
    let failed_tasks = list_failed_task_candidates(&transaction)?;
    let mut next_order_index = next_task_order_index(&transaction)?;
    let limit = max_tasks.map(|value| value as usize).unwrap_or(usize::MAX);
    let mut tasks = Vec::new();

    for task in failed_tasks.iter().take(limit) {
        transaction.execute(
            "UPDATE mc3_tasks SET errors = NULL, finished_tick = NULL, order_index = ?1 WHERE rowid = ?2",
            params![next_order_index, task.task_id],
        )?;
        tasks.push(RequeuedMangaConRepairTask {
            task_id: task.task_id,
            uri: task.uri.clone(),
            volume_key: task.volume_key.clone(),
            location: task.location.clone(),
            errors: task.errors,
            order_index: next_order_index,
        });
        next_order_index += 1;
    }
    if !tasks.is_empty() {
        enable_continue_last_session_tasks(&transaction)?;
    }

    transaction.commit()?;
    Ok(RepairMangaConFailedTasksResult {
        backup_path: backup_path.to_string_lossy().into_owned(),
        total_failed: failed_tasks.len(),
        requeued: tasks.len(),
        tasks,
    })
}

pub fn prepare_unfinished_tasks_for_resume(
    database_path: impl AsRef<Path>,
) -> Result<ResumeMangaConUnfinishedTasksResult> {
    let backup_path = backup_database(database_path.as_ref())?;
    let mut connection = Connection::open(database_path)?;
    let transaction = connection.transaction()?;
    let total_unfinished = count_unfinished_tasks(&transaction)?;
    if total_unfinished > 0 {
        enable_continue_last_session_tasks(&transaction)?;
    }
    transaction.commit()?;

    Ok(ResumeMangaConUnfinishedTasksResult {
        backup_path: backup_path.to_string_lossy().into_owned(),
        total_unfinished,
        resume_configured: total_unfinished > 0,
    })
}

pub fn read_manga_cache_records(
    database_path: impl AsRef<Path>,
    cover_cache_dir: impl AsRef<Path>,
) -> Result<HashMap<String, MangaConMangaCacheRecord>> {
    let database_path = database_path.as_ref();
    let cover_cache_dir = cover_cache_dir.as_ref();
    let covers_dir = database_path
        .parent()
        .map(|parent| parent.join("Covers"))
        .unwrap_or_else(|| PathBuf::from("Covers"));
    let connection = Connection::open(database_path)?;
    let mut statement = connection.prepare(
        r#"
        SELECT m.rowid, m.uri,
               EXISTS(
                   SELECT 1
                   FROM mc3_badges b
                   WHERE b.category = 1 AND b.id = m.rowid AND COALESCE(b.value, 0) > 0
               )
        FROM mc3_mangas m
        "#,
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)? != 0,
        ))
    })?;

    let rows = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut records = HashMap::new();
    for (rowid, uri, has_update) in rows {
        let source_cover_path = covers_dir.join(rowid.to_string());
        let cover_path = materialize_cover_cache(&source_cover_path, cover_cache_dir, rowid)?;
        let record = MangaConMangaCacheRecord {
            rowid,
            uri: uri.clone(),
            cover_path,
            has_update,
        };
        records.insert(uri, record);
    }

    Ok(records)
}

fn materialize_cover_cache(
    source_cover_path: &Path,
    cover_cache_dir: &Path,
    rowid: i64,
) -> Result<Option<PathBuf>> {
    if !source_cover_path.is_file() {
        return Ok(None);
    }

    let bytes = fs::read(source_cover_path)?;
    let Some(extension) = detect_image_extension(&bytes) else {
        return Ok(None);
    };
    fs::create_dir_all(cover_cache_dir)?;
    let cached_cover_path = cover_cache_dir.join(format!("{rowid}.{extension}"));
    fs::write(&cached_cover_path, bytes)?;
    Ok(Some(cached_cover_path))
}

fn detect_image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
        Some("png")
    } else if bytes.starts_with(b"\xFF\xD8\xFF") {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.starts_with(b"BM") {
        Some("bmp")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && &bytes[8..12] == b"avif" {
        Some("avif")
    } else {
        None
    }
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
        FROM mc3_volumes v
        JOIN mc3_mangas m ON m.rowid = v.mid
        WHERE v.status = 1
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

fn list_missing_local_chapter_candidates(
    manga_connection: &Connection,
    companion_database_path: &Path,
) -> Result<Vec<MangaConUpdateCandidate>> {
    if !companion_database_path.is_file() {
        return Ok(Vec::new());
    }

    let companion = Connection::open(companion_database_path)?;
    if !table_exists(&companion, "comics")? || !table_exists(&companion, "chapters")? {
        return Ok(Vec::new());
    }

    let mut comic_statement = companion.prepare(
        r#"
        SELECT id, local_path
        FROM comics
        WHERE local_path IS NOT NULL
          AND TRIM(local_path) <> ''
          AND scan_status = 'matched'
        ORDER BY id
        "#,
    )?;
    let comic_ids = comic_statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                PathBuf::from(row.get::<_, String>(1)?),
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut candidates = Vec::new();
    for (comic_id, local_path) in comic_ids {
        let local_titles = local_chapter_title_index(&companion, &comic_id, &local_path)?;
        if local_titles.normalized_titles.is_empty() {
            continue;
        }

        let mut volume_statement = manga_connection.prepare(
            r#"
            SELECT m.rowid, v.rowid, m.name, m.uri, m.domain, m.location, v.key, v.title
            FROM mc3_mangas m
            JOIN mc3_volumes v ON v.mid = m.rowid
            WHERE m.uri = ?1 AND COALESCE(v.status, 0) = 0
            ORDER BY m.rowid, v.rowid
            "#,
        )?;
        let volume_rows = volume_statement.query_map(params![comic_id], |row| {
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

        for candidate in volume_rows {
            let candidate = candidate?;
            if !should_check_local_gap(&candidate.title, &local_titles) {
                continue;
            }

            let normalized = normalize_chapter_title_for_gap(&candidate.title);
            if !local_titles.normalized_titles.contains(&normalized) {
                candidates.push(candidate);
            }
        }
    }

    Ok(candidates)
}

#[derive(Debug)]
struct LocalChapterTitleIndex {
    normalized_titles: HashSet<String>,
    has_regular_chapters: bool,
    has_volume_chapters: bool,
}

fn local_chapter_title_index(
    companion: &Connection,
    comic_id: &str,
    local_path: &Path,
) -> Result<LocalChapterTitleIndex> {
    if let Some(index) = local_chapter_title_index_from_directory(local_path)? {
        return Ok(index);
    }

    let mut statement = companion.prepare("SELECT title FROM chapters WHERE comic_id = ?1")?;
    let titles = statement
        .query_map(params![comic_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut normalized_titles = HashSet::new();
    let mut has_regular_chapters = false;
    let mut has_volume_chapters = false;
    for title in titles {
        let normalized = normalize_chapter_title_for_gap(&title);
        has_regular_chapters |= is_regular_chapter_title(&normalized);
        has_volume_chapters |= is_volume_chapter_title(&normalized);
        normalized_titles.insert(normalized);
    }

    Ok(LocalChapterTitleIndex {
        normalized_titles,
        has_regular_chapters,
        has_volume_chapters,
    })
}

fn local_chapter_title_index_from_directory(
    local_path: &Path,
) -> Result<Option<LocalChapterTitleIndex>> {
    if !local_path.is_dir() {
        return Ok(None);
    }

    let mut saw_chapter_directories = false;
    let mut normalized_titles = HashSet::new();
    let mut has_regular_chapters = false;
    let mut has_volume_chapters = false;

    for entry in fs::read_dir(local_path)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        saw_chapter_directories = true;
        if !directory_contains_image(&path)? {
            continue;
        }

        let title = entry.file_name().to_string_lossy().trim().to_string();
        if title.is_empty() {
            continue;
        }

        let normalized = normalize_chapter_title_for_gap(&title);
        has_regular_chapters |= is_regular_chapter_title(&normalized);
        has_volume_chapters |= is_volume_chapter_title(&normalized);
        normalized_titles.insert(normalized);
    }

    if !saw_chapter_directories {
        return Ok(None);
    }

    Ok(Some(LocalChapterTitleIndex {
        normalized_titles,
        has_regular_chapters,
        has_volume_chapters,
    }))
}

fn directory_contains_image(path: &Path) -> Result<bool> {
    for entry in fs::read_dir(path)? {
        let child = entry?.path();
        if child.is_dir() {
            if directory_contains_image(&child)? {
                return Ok(true);
            }
        } else if is_gap_image_file(&child) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn is_gap_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "avif"
            )
        })
        .unwrap_or(false)
}

fn table_exists(connection: &Connection, table_name: &str) -> Result<bool> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table_name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn append_unique_candidates(
    candidates: &mut Vec<MangaConUpdateCandidate>,
    extra_candidates: Vec<MangaConUpdateCandidate>,
) {
    let mut seen = candidates
        .iter()
        .map(|candidate| (candidate.uri.clone(), candidate.volume_key.clone()))
        .collect::<HashSet<_>>();

    for candidate in extra_candidates {
        let key = (candidate.uri.clone(), candidate.volume_key.clone());
        if seen.insert(key) {
            candidates.push(candidate);
        }
    }
}

fn should_check_local_gap(title: &str, local_titles: &LocalChapterTitleIndex) -> bool {
    let normalized = normalize_chapter_title_for_gap(title);
    if is_regular_chapter_title(&normalized) {
        return true;
    }

    if is_volume_chapter_title(&normalized) {
        return local_titles.has_volume_chapters || !local_titles.has_regular_chapters;
    }

    false
}

fn is_regular_chapter_title(normalized_title: &str) -> bool {
    normalized_title.contains('话')
}

fn is_volume_chapter_title(normalized_title: &str) -> bool {
    normalized_title.contains('卷')
}

fn normalize_chapter_title_for_gap(title: &str) -> String {
    let compact = title
        .trim()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(|ch| normalize_chapter_char(ch).to_lowercase())
        .collect::<String>();
    strip_digit_padding(&compact)
}

fn normalize_chapter_char(ch: char) -> char {
    match ch {
        '話' => '话',
        '巻' => '卷',
        '０' => '0',
        '１' => '1',
        '２' => '2',
        '３' => '3',
        '４' => '4',
        '５' => '5',
        '６' => '6',
        '７' => '7',
        '８' => '8',
        '９' => '9',
        _ => ch,
    }
}

fn strip_digit_padding(value: &str) -> String {
    let mut output = String::new();
    let mut digits = String::new();

    for ch in value.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            push_unpadded_digits(&mut output, &mut digits);
            output.push(ch);
        }
    }
    push_unpadded_digits(&mut output, &mut digits);

    output
}

fn push_unpadded_digits(output: &mut String, digits: &mut String) {
    if digits.is_empty() {
        return;
    }

    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        output.push('0');
    } else {
        output.push_str(trimmed);
    }
    digits.clear();
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

fn clear_volume_update_marker(connection: &Connection, volume_id: i64) -> Result<usize> {
    connection
        .execute(
            "UPDATE mc3_volumes SET status = 0 WHERE rowid = ?1 AND status = 1",
            params![volume_id],
        )
        .map_err(Into::into)
}

fn count_unfinished_tasks(connection: &Connection) -> Result<usize> {
    let total: i64 = connection.query_row(
        "SELECT COUNT(*) FROM mc3_tasks WHERE finished_tick IS NULL",
        [],
        |row| row.get(0),
    )?;
    Ok(total as usize)
}

fn list_failed_task_candidates(
    connection: &Connection,
) -> Result<Vec<FailedMangaConTaskCandidate>> {
    let mut statement = connection.prepare(
        r#"
        SELECT rowid, mu, vk, location, errors
        FROM mc3_tasks
        WHERE COALESCE(errors, 0) > 0
        ORDER BY rowid
        "#,
    )?;
    let rows = statement.query_map([], |row| {
        Ok(FailedMangaConTaskCandidate {
            task_id: row.get(0)?,
            uri: row.get(1)?,
            volume_key: row.get(2)?,
            location: row.get(3)?,
            errors: row.get(4)?,
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn sync_badge_value(connection: &Connection, manga_id: i64) -> Result<()> {
    let remaining_updates: i64 = connection.query_row(
        "SELECT COUNT(*) FROM mc3_volumes WHERE mid = ?1 AND status = 1",
        params![manga_id],
        |row| row.get(0),
    )?;

    if remaining_updates > 0 {
        connection.execute(
            "UPDATE mc3_badges SET value = ?1 WHERE category = 1 AND id = ?2",
            params![remaining_updates, manga_id],
        )?;
    } else {
        connection.execute(
            "DELETE FROM mc3_badges WHERE category = 1 AND id = ?1",
            params![manga_id],
        )?;
    }

    Ok(())
}

fn enable_continue_last_session_tasks(connection: &Connection) -> Result<()> {
    connection.execute(
        "CREATE TABLE IF NOT EXISTS mc3_config(key TEXT PRIMARY KEY, value TEXT) WITHOUT ROWID",
        [],
    )?;
    connection.execute(
        "REPLACE INTO mc3_config(key, value) VALUES(?1, ?2)",
        params![CONTINUE_LAST_SESSION_TASKS_KEY, "1"],
    )?;
    Ok(())
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
        assert_eq!(result.cleared_update_markers, 1);
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

        let volume_status: i64 = connection
            .query_row(
                "SELECT status FROM mc3_volumes WHERE rowid = 37246",
                [],
                |row| row.get(0),
            )
            .expect("updated volume status");
        let badge_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM mc3_badges WHERE category = 1 AND id = 31",
                [],
                |row| row.get(0),
            )
            .expect("badge count");
        assert_eq!(volume_status, 0);
        assert_eq!(badge_count, 0);
        assert_eq!(
            read_config_value(&connection, "continue_last_session_tasks").as_deref(),
            Some("1")
        );
    }

    #[test]
    fn queues_status_one_volumes_even_when_manga_badge_is_missing() {
        let (_temp, path) = create_fixture_db();
        let connection = Connection::open(&path).expect("open");
        connection
            .execute("DELETE FROM mc3_badges WHERE category = 1 AND id = 31", [])
            .expect("delete badge");
        drop(connection);

        let result = queue_all_badged_updates(&path, None).expect("queue updates");

        assert_eq!(result.total_updates, 1);
        assert_eq!(result.queued, 1);
        assert_eq!(result.tasks[0].uri, "cp:jianmingyidongdescp");
        assert_eq!(
            result.tasks[0].volume_key,
            "b1b787e2-7252-11f1-98ad-fa163e02432f"
        );

        let connection = Connection::open(path).expect("reopen");
        let volume_status: i64 = connection
            .query_row(
                "SELECT status FROM mc3_volumes WHERE rowid = 37246",
                [],
                |row| row.get(0),
            )
            .expect("updated volume status");
        assert_eq!(volume_status, 0);
    }

    #[test]
    fn queues_missing_local_chapters_even_when_mangacon_has_no_update_marker() {
        let (_temp, manga_db_path) = create_fixture_db();
        let connection = Connection::open(&manga_db_path).expect("open manga db");
        connection
            .execute("DELETE FROM mc3_volumes WHERE mid = 31", [])
            .expect("clear fixture volumes");
        connection
            .execute(
                "INSERT INTO mc3_volumes(rowid, mid, key, title, status) VALUES (?1, ?2, ?3, ?4, 0)",
                params![37246, 31, "local-one", "第01话"],
            )
            .expect("local first chapter");
        connection
            .execute(
                "INSERT INTO mc3_volumes(rowid, mid, key, title, status) VALUES (?1, ?2, ?3, ?4, 0)",
                params![37247, 31, "local-two", "第02话"],
            )
            .expect("local second chapter");
        connection
            .execute(
                "INSERT INTO mc3_volumes(rowid, mid, key, title, status) VALUES (?1, ?2, ?3, ?4, 0)",
                params![37248, 31, "missing-chapter", "第03话"],
            )
            .expect("missing local chapter");
        connection
            .execute(
                "INSERT INTO mc3_volumes(rowid, mid, key, title, status) VALUES (?1, ?2, ?3, ?4, 0)",
                params![37249, 31, "single-book-volume", "第01卷"],
            )
            .expect("single book volume");
        drop(connection);

        let companion_temp = tempfile::tempdir().expect("companion tempdir");
        let companion_db_path = companion_temp.path().join("state.sqlite");
        let companion = Connection::open(&companion_db_path).expect("open companion db");
        companion
            .execute_batch(
                r#"
                CREATE TABLE comics(id TEXT PRIMARY KEY, local_path TEXT, scan_status TEXT NOT NULL);
                CREATE TABLE chapters(comic_id TEXT NOT NULL, title TEXT NOT NULL);
                INSERT INTO comics(id, local_path, scan_status) VALUES ('cp:jianmingyidongdescp', 'E:\books\sample', 'matched');
                INSERT INTO chapters(comic_id, title) VALUES ('cp:jianmingyidongdescp', '第01話');
                INSERT INTO chapters(comic_id, title) VALUES ('cp:jianmingyidongdescp', '第2话');
                "#,
            )
            .expect("companion schema");
        drop(companion);

        let result = queue_updates_including_local_gaps(
            &manga_db_path,
            Some(companion_db_path.as_path()),
            None,
        )
        .expect("queue updates");

        assert_eq!(result.total_updates, 1);
        assert_eq!(result.queued, 1);
        assert_eq!(result.cleared_update_markers, 0);
        assert_eq!(result.tasks[0].volume_key, "missing-chapter");
        assert_eq!(result.tasks[0].title, "第03话");

        let connection = Connection::open(manga_db_path).expect("reopen manga db");
        let queued_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM mc3_tasks", [], |row| row.get(0))
            .expect("queued task count");
        let volume_task_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM mc3_tasks WHERE vk = 'single-book-volume'",
                [],
                |row| row.get(0),
            )
            .expect("single book volume task count");
        assert_eq!(queued_count, 1);
        assert_eq!(volume_task_count, 0);
    }

    #[test]
    fn local_gap_detection_uses_real_matched_directory_before_stale_cached_chapters() {
        let (_temp, manga_db_path) = create_fixture_db();
        let connection = Connection::open(&manga_db_path).expect("open manga db");
        connection
            .execute("DELETE FROM mc3_volumes WHERE mid = 31", [])
            .expect("clear fixture volumes");
        for (rowid, key, title) in [
            (37246_i64, "local-one", "第01话"),
            (37247_i64, "local-two", "第02话"),
            (37248_i64, "missing-chapter", "第03话"),
        ] {
            connection
                .execute(
                    "INSERT INTO mc3_volumes(rowid, mid, key, title, status) VALUES (?1, 31, ?2, ?3, 0)",
                    params![rowid, key, title],
                )
                .expect("fixture volume");
        }
        drop(connection);

        let companion_temp = tempfile::tempdir().expect("companion tempdir");
        let local_manga_dir = companion_temp.path().join("sample");
        for title in ["第01話", "第02话"] {
            let chapter = local_manga_dir.join(title);
            fs::create_dir_all(&chapter).expect("local chapter dir");
            fs::write(chapter.join("001.jpg"), b"image").expect("local image");
        }

        let companion_db_path = companion_temp.path().join("state.sqlite");
        let companion = Connection::open(&companion_db_path).expect("open companion db");
        companion
            .execute_batch(
                r#"
                CREATE TABLE comics(id TEXT PRIMARY KEY, local_path TEXT, scan_status TEXT NOT NULL);
                CREATE TABLE chapters(comic_id TEXT NOT NULL, title TEXT NOT NULL);
                INSERT INTO chapters(comic_id, title) VALUES ('cp:jianmingyidongdescp', '第01話');
                INSERT INTO chapters(comic_id, title) VALUES ('cp:jianmingyidongdescp', '第02话');
                INSERT INTO chapters(comic_id, title) VALUES ('cp:jianmingyidongdescp', '第03话');
                "#,
            )
            .expect("companion schema");
        companion
            .execute(
                "INSERT INTO comics(id, local_path, scan_status) VALUES (?1, ?2, 'matched')",
                params![
                    "cp:jianmingyidongdescp",
                    local_manga_dir.to_string_lossy().as_ref(),
                ],
            )
            .expect("companion comic");
        drop(companion);

        let result = queue_updates_including_local_gaps(
            &manga_db_path,
            Some(companion_db_path.as_path()),
            None,
        )
        .expect("queue updates");

        assert_eq!(result.queued, 1);
        assert_eq!(result.tasks[0].volume_key, "missing-chapter");
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
        assert_eq!(result.cleared_update_markers, 1);
        assert!(result.tasks.is_empty());

        let connection = Connection::open(path).expect("reopen");
        let volume_status: i64 = connection
            .query_row(
                "SELECT status FROM mc3_volumes WHERE rowid = 37246",
                [],
                |row| row.get(0),
            )
            .expect("updated volume status");
        let badge_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM mc3_badges WHERE category = 1 AND id = 31",
                [],
                |row| row.get(0),
            )
            .expect("badge count");
        assert_eq!(volume_status, 0);
        assert_eq!(badge_count, 0);
    }

    #[test]
    fn keeps_remaining_badge_count_when_limit_leaves_updates_unprocessed() {
        let (_temp, path) = create_fixture_db();
        let connection = Connection::open(&path).expect("open");
        connection
            .execute(
                "INSERT INTO mc3_volumes(rowid, mid, key, title, status) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![37248, 31, "later-update", "第358话", 1],
            )
            .expect("second updated volume");
        connection
            .execute(
                "UPDATE mc3_badges SET value = 2 WHERE category = 1 AND id = 31",
                [],
            )
            .expect("badge value");
        drop(connection);

        let result = queue_all_badged_updates(&path, Some(1)).expect("queue updates");

        assert_eq!(result.total_updates, 2);
        assert_eq!(result.queued, 1);
        assert_eq!(result.skipped_existing, 0);
        assert_eq!(result.cleared_update_markers, 1);

        let connection = Connection::open(path).expect("reopen");
        let processed_status: i64 = connection
            .query_row(
                "SELECT status FROM mc3_volumes WHERE rowid = 37246",
                [],
                |row| row.get(0),
            )
            .expect("processed volume status");
        let remaining_status: i64 = connection
            .query_row(
                "SELECT status FROM mc3_volumes WHERE rowid = 37248",
                [],
                |row| row.get(0),
            )
            .expect("remaining volume status");
        let badge_value: i64 = connection
            .query_row(
                "SELECT value FROM mc3_badges WHERE category = 1 AND id = 31",
                [],
                |row| row.get(0),
            )
            .expect("badge value");
        assert_eq!(processed_status, 0);
        assert_eq!(remaining_status, 1);
        assert_eq!(badge_value, 1);
    }

    #[test]
    fn task_status_counts_active_finished_and_failed_tasks() {
        let (_temp, path) = create_fixture_db();
        let connection = Connection::open(&path).expect("open");
        connection
            .execute(
                "INSERT INTO mc3_tasks(mu, domain, vk, location, extra, errors, order_index, finished_tick) VALUES (?1, NULL, ?2, ?3, ?4, NULL, 3, NULL)",
                params![
                    "cp:active",
                    "active-volume",
                    "Active\\第01话",
                    r#"{"mid":1,"vid":1}"#,
                ],
            )
            .expect("active task");
        connection
            .execute(
                "INSERT INTO mc3_tasks(mu, domain, vk, location, extra, errors, order_index, finished_tick) VALUES (?1, NULL, ?2, ?3, ?4, 2, NULL, 1234)",
                params![
                    "cp:failed",
                    "failed-volume",
                    "Failed\\第02话",
                    r#"{"mid":2,"vid":2}"#,
                ],
            )
            .expect("failed task");
        connection
            .execute(
                "INSERT INTO mc3_tasks(mu, domain, vk, location, extra, errors, order_index, finished_tick) VALUES (?1, NULL, ?2, ?3, ?4, 0, NULL, 5678)",
                params![
                    "cp:finished",
                    "finished-volume",
                    "Finished\\第03话",
                    r#"{"mid":3,"vid":3}"#,
                ],
            )
            .expect("finished task");
        drop(connection);

        let status = read_task_status(&path).expect("task status");

        assert_eq!(status.total_tasks, 3);
        assert_eq!(status.active_tasks, 1);
        assert_eq!(status.failed_tasks, 1);
        assert_eq!(status.finished_tasks, 2);
        assert_eq!(status.total_errors, 2);
    }

    #[test]
    fn requeues_failed_finished_tasks_for_mangacon_repair() {
        let (_temp, path) = create_fixture_db();
        let connection = Connection::open(&path).expect("open");
        connection
            .execute(
                "INSERT INTO mc3_tasks(mu, domain, vk, location, extra, errors, order_index, finished_tick) VALUES (?1, NULL, ?2, ?3, ?4, NULL, 9, NULL)",
                params![
                    "cp:active",
                    "active-volume",
                    "Active\\第01话",
                    r#"{"mid":1,"vid":1}"#,
                ],
            )
            .expect("active task");
        connection
            .execute(
                "INSERT INTO mc3_tasks(rowid, mu, domain, vk, location, extra, errors, order_index, finished_tick) VALUES (90, ?1, NULL, ?2, ?3, ?4, 3, NULL, 1234)",
                params![
                    "mhg:55324",
                    "892965",
                    "被开除的链金术师、用玩具拯救世界～让一切魔兽起飞的男人～\\第64话",
                    r#"{"mid":378,"vid":37231}"#,
                ],
            )
            .expect("failed task");
        drop(connection);

        let result = requeue_failed_tasks_for_repair(&path, Some(10)).expect("repair failed tasks");

        assert_eq!(result.total_failed, 1);
        assert_eq!(result.requeued, 1);
        assert!(std::path::Path::new(&result.backup_path).exists());
        assert_eq!(result.tasks[0].task_id, 90);
        assert_eq!(result.tasks[0].errors, 3);
        assert_eq!(result.tasks[0].order_index, 10);

        let connection = Connection::open(path).expect("reopen");
        let row = connection
            .query_row(
                "SELECT errors, order_index, finished_tick FROM mc3_tasks WHERE rowid = 90",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<i64>>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                },
            )
            .expect("requeued row");
        assert_eq!(row.0, None);
        assert_eq!(row.1, Some(10));
        assert_eq!(row.2, None);
        assert_eq!(
            read_config_value(&connection, "continue_last_session_tasks").as_deref(),
            Some("1")
        );
    }

    #[test]
    fn prepares_existing_unfinished_tasks_for_database_resume() {
        let (_temp, path) = create_fixture_db();
        let connection = Connection::open(&path).expect("open");
        connection
            .execute(
                "INSERT INTO mc3_tasks(mu, domain, vk, location, extra, errors, order_index, finished_tick) VALUES (?1, NULL, ?2, ?3, ?4, NULL, 12, NULL)",
                params![
                    "cp:unfinished",
                    "unfinished-volume",
                    "Unfinished\\Chapter 1",
                    r#"{"mid":1,"vid":2}"#,
                ],
            )
            .expect("unfinished task");
        drop(connection);

        let result = prepare_unfinished_tasks_for_resume(&path).expect("prepare unfinished tasks");

        assert_eq!(result.total_unfinished, 1);
        assert!(result.resume_configured);
        assert!(std::path::Path::new(&result.backup_path).exists());

        let connection = Connection::open(path).expect("reopen");
        assert_eq!(
            read_config_value(&connection, "continue_last_session_tasks").as_deref(),
            Some("1")
        );
    }

    fn read_config_value(connection: &Connection, key: &str) -> Option<String> {
        connection
            .query_row(
                "SELECT value FROM mc3_config WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok()
    }

    #[test]
    fn requeue_failed_tasks_respects_limit_and_keeps_remaining_errors() {
        let (_temp, path) = create_fixture_db();
        let connection = Connection::open(&path).expect("open");
        for (task_id, uri, key, errors) in [
            (101_i64, "cp:failed-a", "a", 1_i64),
            (102_i64, "cp:failed-b", "b", 2_i64),
        ] {
            connection
                .execute(
                    "INSERT INTO mc3_tasks(rowid, mu, domain, vk, location, extra, errors, order_index, finished_tick) VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, NULL, 1234)",
                    params![
                        task_id,
                        uri,
                        key,
                        format!("Failed\\{key}"),
                        format!(r#"{{"mid":{task_id},"vid":{task_id}}}"#),
                        errors,
                    ],
                )
                .expect("failed task");
        }
        drop(connection);

        let result = requeue_failed_tasks_for_repair(&path, Some(1)).expect("repair failed tasks");

        assert_eq!(result.total_failed, 2);
        assert_eq!(result.requeued, 1);
        assert_eq!(result.tasks[0].task_id, 101);

        let connection = Connection::open(path).expect("reopen");
        let first = connection
            .query_row(
                "SELECT errors, finished_tick FROM mc3_tasks WHERE rowid = 101",
                [],
                |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .expect("first row");
        let second = connection
            .query_row(
                "SELECT errors, finished_tick FROM mc3_tasks WHERE rowid = 102",
                [],
                |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .expect("second row");
        assert_eq!(first, (None, None));
        assert_eq!(second, (Some(2), Some(1234)));
    }
}
