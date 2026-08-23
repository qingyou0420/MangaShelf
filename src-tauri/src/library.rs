use crate::{
    bookshelf::{
        cheap_signature, extract_cache_stats, find_first_image_page, list_cover_candidates,
        manga_directory_fingerprint, prune_extract_cache, scan_bookshelf, scan_comic_chapters,
        scan_comic_chapters_reusing,
    },
    config::resolve_database_path,
    cover::cover_or_source,
    db::LibraryDatabase,
    domain::{
        path_is_under, same_path, CacheStats, Chapter, Comic, FitMode, LoadLibraryResult,
        ReadMode, ReadingDirection, ScanFailure, ScanLibraryResult, ScanProgress, ScanStatus,
    },
};
use anyhow::Result;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

enum LocalScanOutcome {
    Changed,
    Unchanged,
    SkippedEmpty,
}

pub fn load_library(
    bookshelf_root: impl AsRef<Path>,
    database_path: Option<impl AsRef<Path>>,
) -> Result<LoadLibraryResult> {
    let bookshelf_root = bookshelf_root.as_ref().to_path_buf();
    let database_path = resolve_database_path(
        &bookshelf_root,
        database_path.as_ref().map(|path| path.as_ref()),
    );
    if !database_path.exists() {
        return Ok(LoadLibraryResult {
            database_path: path_display(&database_path),
            bookshelf_root: path_display(&bookshelf_root),
            comics: Vec::new(),
        });
    }
    let db = LibraryDatabase::open(&database_path)?;
    db.migrate()?;
    Ok(LoadLibraryResult {
        database_path: path_display(&database_path),
        bookshelf_root: path_display(&bookshelf_root),
        comics: db.list_comics()?,
    })
}

pub fn scan_library(
    bookshelf_root: impl AsRef<Path>,
    database_path: Option<impl AsRef<Path>>,
) -> Result<ScanLibraryResult> {
    scan_library_with_progress(bookshelf_root, database_path, &[] as &[&Path], |_| true)
}

pub fn scan_library_with_progress(
    bookshelf_root: impl AsRef<Path>,
    database_path: Option<impl AsRef<Path>>,
    extra_roots: &[impl AsRef<Path>],
    mut on_progress: impl FnMut(&ScanProgress) -> bool,
) -> Result<ScanLibraryResult> {
    let bookshelf_root = bookshelf_root.as_ref().to_path_buf();
    let database_path = resolve_database_path(
        &bookshelf_root,
        database_path.as_ref().map(|path| path.as_ref()),
    );
    let extra_owned: Vec<PathBuf> = extra_roots
        .iter()
        .map(|path| path.as_ref().to_path_buf())
        .collect();
    let db = LibraryDatabase::open(&database_path)?;
    db.migrate()?;

    let existing = db.list_comics()?;
    let mut locals = scan_bookshelf(&bookshelf_root)?;
    for extra in &extra_owned {
        locals.extend(scan_bookshelf(extra)?);
    }
    let mut matched_ids = HashSet::new();
    let mut added = 0;
    let mut updated = 0;
    let mut unchanged = 0;
    let mut failed = 0;
    let mut failed_items = Vec::new();
    let mut cancelled = false;
    let total = locals.len();

    for (index, local) in locals.iter().enumerate() {
        let progress = ScanProgress {
            scanned: index,
            total,
            current_title: local.title.clone(),
        };
        if !on_progress(&progress) {
            cancelled = true;
            break;
        }

        let result = (|| -> Result<(bool, LocalScanOutcome)> {
            let Some(existing_index) = find_existing_comic(&existing, local) else {
                let mut comic =
                    Comic::from_local_directory(local.title.clone(), local.directory.clone());
                apply_local_scan(&db, &mut comic, local, None, &bookshelf_root, &extra_owned)?;
                return Ok((true, LocalScanOutcome::Changed));
            };

            let mut comic = existing[existing_index].clone();
            matched_ids.insert(comic.id.clone());
            let previous_fingerprint = db.local_fingerprint_for_comic(&comic.id)?;
            let outcome = apply_local_scan(
                &db,
                &mut comic,
                local,
                previous_fingerprint.as_deref(),
                &bookshelf_root,
                &extra_owned,
            )?;
            Ok((false, outcome))
        })();

        match result {
            Ok((_, LocalScanOutcome::SkippedEmpty)) => {}
            Ok((true, _)) => added += 1,
            Ok((false, LocalScanOutcome::Changed)) => updated += 1,
            Ok((false, LocalScanOutcome::Unchanged)) => unchanged += 1,
            Err(error) => {
                failed += 1;
                failed_items.push(ScanFailure {
                    title: local.title.clone(),
                    error: error.to_string(),
                });
            }
        }
    }

    let mut missing = 0;
    if !cancelled {
        for comic in &existing {
            if matched_ids.contains(&comic.id) {
                continue;
            }
            let mut missing_comic = comic.clone();
            missing_comic.local_path = None;
            missing_comic.chapter_count = 0;
            missing_comic.image_count = 0;
            missing_comic.latest_chapter_title = None;
            missing_comic.scan_status = ScanStatus::Missing;
            match db.commit_scanned_comic(&missing_comic, Some(&[]), None) {
                Ok(()) => missing += 1,
                Err(error) => {
                    failed += 1;
                    failed_items.push(ScanFailure {
                        title: comic.name.clone(),
                        error: error.to_string(),
                    });
                }
            }
        }
    }

    let _ = on_progress(&ScanProgress {
        scanned: added + updated,
        total,
        current_title: String::new(),
    });

    Ok(ScanLibraryResult {
        scanned: locals.len(),
        added,
        updated,
        unchanged,
        missing,
        failed,
        failed_items,
        database_path: path_display(&database_path),
        bookshelf_root: path_display(&bookshelf_root),
        comics: db.list_comics()?,
    })
}

pub fn scan_chapters_with_progress(
    comic_id: &str,
    comic_directory: impl AsRef<Path>,
    database_path: Option<impl AsRef<Path>>,
) -> Result<Vec<Chapter>> {
    load_or_scan_chapters(comic_id, comic_directory, database_path, true)
}

pub fn load_or_scan_chapters(
    comic_id: &str,
    comic_directory: impl AsRef<Path>,
    database_path: Option<impl AsRef<Path>>,
    force: bool,
) -> Result<Vec<Chapter>> {
    let comic_directory = comic_directory.as_ref();
    let Some(database_path) = database_path else {
        return scan_comic_chapters(comic_id, comic_directory);
    };
    let database_path = database_path.as_ref();
    if database_path.as_os_str().is_empty() {
        return scan_comic_chapters(comic_id, comic_directory);
    }

    let db = LibraryDatabase::open(database_path)?;
    db.migrate()?;
    if !force {
        if let Some(stored_fp) = db.local_fingerprint_for_comic(comic_id)? {
            let current = manga_directory_fingerprint(comic_directory)?;
            if stored_fp == current {
                let stored = db.list_chapters_for_comic(comic_id).unwrap_or_default();
                if !stored.is_empty() {
                    return Ok(stored);
                }
            }
        }
    }

    let stored = db.list_chapters_for_comic(comic_id).unwrap_or_default();
    let reuse = if force { Vec::new() } else { stored.clone() };
    let mut chapters = scan_comic_chapters_reusing(comic_id, comic_directory, &reuse)?;
    merge_chapter_progress(&mut chapters, &stored);
    db.replace_chapters_for_comic(comic_id, &chapters)?;
    if let Ok(fingerprint) = manga_directory_fingerprint(comic_directory) {
        let _ = db.update_local_fingerprint_for_comic(comic_id, Some(&fingerprint));
    }
    Ok(chapters)
}

pub fn save_read_progress(
    database_path: impl AsRef<Path>,
    comic_id: &str,
    chapter_id: &str,
    page: u32,
) -> Result<Option<Comic>> {
    let db = LibraryDatabase::open(database_path)?;
    db.migrate()?;
    db.save_read_progress(comic_id, chapter_id, page)
}

pub fn update_comic_metadata(
    database_path: impl AsRef<Path>,
    comic_id: &str,
    name: Option<String>,
    author: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Option<Comic>> {
    let db = LibraryDatabase::open(database_path)?;
    db.migrate()?;
    db.update_comic_metadata(
        comic_id,
        name.as_deref(),
        author.as_deref(),
        tags.as_deref(),
    )
}

pub fn set_comic_favorite(
    database_path: impl AsRef<Path>,
    comic_id: &str,
    favorited: bool,
) -> Result<Option<Comic>> {
    let db = LibraryDatabase::open(database_path)?;
    db.migrate()?;
    db.set_comic_favorite(comic_id, favorited)
}

pub fn set_reader_prefs(
    database_path: impl AsRef<Path>,
    comic_id: &str,
    reading_direction: ReadingDirection,
    fit_mode: FitMode,
    read_mode: ReadMode,
) -> Result<Option<Comic>> {
    let db = LibraryDatabase::open(database_path)?;
    db.migrate()?;
    db.set_reader_prefs(comic_id, reading_direction, fit_mode, read_mode)
}

pub fn clear_comic_progress(
    database_path: impl AsRef<Path>,
    comic_id: &str,
) -> Result<Option<Comic>> {
    let db = LibraryDatabase::open(database_path)?;
    db.migrate()?;
    db.clear_read_progress(comic_id)
}

pub fn delete_library_comic(
    database_path: impl AsRef<Path>,
    comic_id: &str,
) -> Result<()> {
    let db = LibraryDatabase::open(database_path)?;
    db.migrate()?;
    db.delete_comic(comic_id)
}

pub fn set_comic_cover(
    bookshelf_root: impl AsRef<Path>,
    database_path: impl AsRef<Path>,
    comic_id: &str,
    source: impl AsRef<Path>,
) -> Result<Option<Comic>> {
    let db = LibraryDatabase::open(database_path)?;
    db.migrate()?;
    let Some(mut comic) = db.get_comic(comic_id)? else {
        return Ok(None);
    };
    comic.cover_path = Some(cover_or_source(
        bookshelf_root.as_ref(),
        comic_id,
        source.as_ref().to_path_buf(),
    ));
    db.upsert_comic(&comic)?;
    db.get_comic(comic_id)
}

pub fn cover_candidates(comic_directory: impl AsRef<Path>) -> Result<Vec<String>> {
    list_cover_candidates(comic_directory)
}

pub fn cache_stats(
    bookshelf_root: impl AsRef<Path>,
    extra_roots: &[impl AsRef<Path>],
) -> Result<CacheStats> {
    cache_stats_for_roots(std::iter::once(bookshelf_root.as_ref()).chain(extra_roots.iter().map(|path| path.as_ref())))
}

pub fn clear_extract_cache(
    bookshelf_root: impl AsRef<Path>,
    extra_roots: &[impl AsRef<Path>],
    max_bytes: Option<u64>,
) -> Result<CacheStats> {
    let limit = max_bytes.or(Some(0));
    let mut freed = 0u64;
    for root in std::iter::once(bookshelf_root.as_ref()).chain(extra_roots.iter().map(|path| path.as_ref())) {
        freed += prune_extract_cache(root, limit)?.freed_bytes;
    }
    let mut summed = cache_stats(bookshelf_root, extra_roots)?;
    summed.freed_bytes = freed;
    Ok(summed)
}

fn cache_stats_for_roots<'a>(roots: impl Iterator<Item = &'a Path>) -> Result<CacheStats> {
    let mut bytes = 0u64;
    let mut folders = 0usize;
    for root in roots {
        let stats = extract_cache_stats(root)?;
        bytes += stats.bytes;
        folders += stats.folders;
    }
    Ok(CacheStats {
        bytes,
        folders,
        freed_bytes: 0,
    })
}

fn cache_root_for(path: &Path, main_root: &Path, extra_roots: &[PathBuf]) -> PathBuf {
    extra_roots
        .iter()
        .find(|root| path_is_under(path, root))
        .cloned()
        .filter(|root| !root.as_os_str().is_empty())
        .unwrap_or_else(|| {
            if path_is_under(path, main_root) {
                main_root.to_path_buf()
            } else {
                path.parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| main_root.to_path_buf())
            }
        })
}

fn apply_local_scan(
    db: &LibraryDatabase,
    comic: &mut Comic,
    local: &crate::domain::LocalManga,
    previous_fingerprint: Option<&str>,
    bookshelf_root: &Path,
    extra_roots: &[PathBuf],
) -> Result<LocalScanOutcome> {
    let previous_chapters = comic.chapter_count;
    let was_new = previous_fingerprint.is_none() && previous_chapters == 0;
    let cheap = if local.cheap_signature.is_empty() {
        cheap_signature(&local.directory)?
    } else {
        local.cheap_signature.clone()
    };
    let same_directory = comic
        .local_path
        .as_ref()
        .is_some_and(|path| same_path(path, &local.directory));
    let stored_cheap = db.cheap_signature_for_comic(&comic.id).ok().flatten();
    if same_directory
        && stored_cheap.as_deref() == Some(cheap.as_str())
        && previous_fingerprint.is_some()
        && comic.scan_status == ScanStatus::Matched
        && comic.chapter_count > 0
    {
        return Ok(LocalScanOutcome::Unchanged);
    }

    let fingerprint = manga_directory_fingerprint(&local.directory)?;
    let can_reuse_index = same_directory
        && previous_fingerprint == Some(fingerprint.as_str())
        && comic.scan_status == ScanStatus::Matched
        && comic.chapter_count > 0;

    if can_reuse_index {
        db.update_cheap_signature(&comic.id, Some(&cheap))?;
        if previous_fingerprint.is_some_and(|value| !value.starts_with("v2:")) {
            db.update_local_fingerprint_for_comic(&comic.id, Some(&fingerprint))?;
        }
        return Ok(LocalScanOutcome::Unchanged);
    }

    let stored = db.list_chapters_for_comic(&comic.id).unwrap_or_default();
    let mut chapters =
        scan_comic_chapters_reusing(&comic.id, &local.directory, &stored)?;
    if chapters.is_empty() && previous_fingerprint.is_none() && comic.chapter_count == 0 {
        return Ok(LocalScanOutcome::SkippedEmpty);
    }
    merge_chapter_progress(&mut chapters, &stored);
    comic.chapter_count = chapters.len();
    comic.image_count = chapters.iter().map(|chapter| chapter.page_count).sum();
    comic.latest_chapter_title = latest_chapter_title(&chapters);

    if comic.name.trim().is_empty() || comic.name == comic.location {
        comic.name = local.title.clone();
    }
    comic.location = local.title.clone();
    comic.local_path = Some(local.directory.clone());
    if comic
        .cover_path
        .as_ref()
        .is_none_or(|path| !path.exists())
    {
        let cover_root = cache_root_for(&local.directory, bookshelf_root, extra_roots);
        comic.cover_path = find_first_image_page(&local.directory)?
            .map(|source| cover_or_source(&cover_root, &comic.id, source));
    }
    comic.scan_status = ScanStatus::Matched;
    let added_chapters = comic.chapter_count.saturating_sub(previous_chapters);
    if was_new {
        comic.shelf_updated_at = Some(db.now_stamp());
        comic.shelf_update_note = Some("新书".to_string());
    } else if added_chapters > 0 {
        let latest = comic.latest_chapter_title.clone().unwrap_or_default();
        comic.shelf_updated_at = Some(db.now_stamp());
        comic.shelf_update_note = Some(if latest.is_empty() {
            format!("新增 {added_chapters} 话")
        } else {
            format!("更新至{latest}")
        });
    } else {
        comic.shelf_updated_at = Some(db.now_stamp());
        comic.shelf_update_note = Some("有新内容".to_string());
    }
    db.commit_scanned_comic(comic, Some(&chapters), Some(&fingerprint))?;
    db.update_cheap_signature(&comic.id, Some(&cheap))?;
    Ok(LocalScanOutcome::Changed)
}

fn find_existing_comic(existing: &[Comic], local: &crate::domain::LocalManga) -> Option<usize> {
    existing.iter().position(|comic| {
        comic
            .local_path
            .as_ref()
            .is_some_and(|path| same_path(path, &local.directory))
    }).or_else(|| {
        existing.iter().position(|comic| {
            comic.local_path.is_none()
                && (titles_match(&comic.location, &local.title)
                    || titles_match(&comic.name, &local.title))
        })
    })
}

fn titles_match(left: &str, right: &str) -> bool {
    normalize_title(left) == normalize_title(right) && !left.trim().is_empty()
}

fn normalize_title(title: &str) -> String {
    title
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn merge_chapter_progress(chapters: &mut [Chapter], stored: &[Chapter]) {
    for chapter in chapters.iter_mut() {
        if let Some(previous) = stored
            .iter()
            .find(|item| item.id == chapter.id)
            .or_else(|| stored.iter().find(|item| same_path(&item.path, &chapter.path)))
            .or_else(|| stored.iter().find(|item| item.title == chapter.title))
        {
            chapter.read_progress_page = previous.read_progress_page;
        }
    }
}

fn latest_chapter_title(chapters: &[Chapter]) -> Option<String> {
    chapters
        .iter()
        .rev()
        .find(|chapter| chapter.special_kind == crate::domain::ChapterKind::Regular)
        .or_else(|| chapters.last())
        .map(|chapter| chapter.title.clone())
}

fn path_display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_creates_library_from_local_folders_and_is_idempotent() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("bookshelf");
        let chapter = root.join("孤独摇滚").join("第01话");
        fs::create_dir_all(&chapter).expect("chapter");
        fs::write(chapter.join("001.jpg"), b"image").expect("page");
        let db_path = temp.path().join("library.sqlite");

        let first = scan_library(&root, Some(&db_path)).expect("first scan");
        assert_eq!(first.scanned, 1);
        assert_eq!(first.added, 1);
        assert_eq!(first.updated, 0);
        assert_eq!(first.comics[0].name, "孤独摇滚");
        assert_eq!(first.comics[0].chapter_count, 1);
        assert_eq!(first.comics[0].scan_status, ScanStatus::Matched);
        assert!(first.comics[0].cover_path.is_some());

        let second = scan_library(&root, Some(&db_path)).expect("second scan");
        assert_eq!(second.added, 0);
        assert_eq!(second.updated, 0);
        assert_eq!(second.unchanged, 1);
        assert_eq!(second.comics.len(), 1);
        assert_eq!(second.comics[0].id, first.comics[0].id);
    }

    #[test]
    fn scan_preserves_progress_and_custom_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("bookshelf");
        let chapter = root.join("测试漫画").join("第01话");
        fs::create_dir_all(&chapter).expect("chapter");
        fs::write(chapter.join("001.jpg"), b"image").expect("page");
        let db_path = temp.path().join("library.sqlite");

        let scanned = scan_library(&root, Some(&db_path)).expect("scan");
        let comic_id = scanned.comics[0].id.clone();
        let chapter_id = format!("{comic_id}::第01话");
        save_read_progress(&db_path, &comic_id, &chapter_id, 4).expect("progress");
        update_comic_metadata(
            &db_path,
            &comic_id,
            Some("自定义书名".into()),
            Some("作者A".into()),
            Some(vec!["标签".into()]),
        )
        .expect("meta");
        set_comic_favorite(&db_path, &comic_id, true).expect("fav");

        fs::write(chapter.join("002.jpg"), b"image").expect("new page");
        let again = scan_library(&root, Some(&db_path)).expect("rescan");
        let comic = &again.comics[0];
        assert_eq!(comic.name, "自定义书名");
        assert_eq!(comic.author.as_deref(), Some("作者A"));
        assert_eq!(comic.tags, vec!["标签"]);
        assert!(comic.favorited);
        assert_eq!(comic.read_progress_page, 4);
        assert_eq!(comic.last_read_chapter_id.as_deref(), Some(chapter_id.as_str()));

        let chapters = scan_chapters_with_progress(&comic_id, root.join("测试漫画"), Some(&db_path))
            .expect("chapters");
        assert_eq!(chapters[0].read_progress_page, 4);
    }

    #[test]
    fn scan_marks_missing_without_deleting_user_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("bookshelf");
        let keep = root.join("还在").join("第01话");
        let gone = root.join("已挪走").join("第01话");
        fs::create_dir_all(&keep).expect("keep");
        fs::create_dir_all(&gone).expect("gone");
        fs::write(keep.join("001.jpg"), b"image").expect("keep page");
        fs::write(gone.join("001.jpg"), b"image").expect("gone page");
        let db_path = temp.path().join("library.sqlite");

        scan_library(&root, Some(&db_path)).expect("first");
        fs::remove_dir_all(root.join("已挪走")).expect("move away");
        let after = scan_library(&root, Some(&db_path)).expect("second");

        assert_eq!(after.scanned, 1);
        assert_eq!(after.missing, 1);
        assert!(root.join("还在").join("第01话").join("001.jpg").exists());
        let missing = after
            .comics
            .iter()
            .find(|comic| comic.scan_status == ScanStatus::Missing)
            .expect("missing comic");
        assert_eq!(missing.location, "已挪走");
    }

    #[test]
    fn load_library_reads_index_without_scanning() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("bookshelf");
        let chapter = root.join("已索引").join("第01话");
        fs::create_dir_all(&chapter).expect("chapter");
        fs::write(chapter.join("001.jpg"), b"image").expect("page");
        let db_path = temp.path().join("library.sqlite");
        scan_library(&root, Some(&db_path)).expect("scan");

        fs::write(chapter.join("002.jpg"), b"image").expect("new page not scanned");
        let loaded = load_library(&root, Some(&db_path)).expect("load");
        assert_eq!(loaded.comics.len(), 1);
        assert_eq!(loaded.comics[0].image_count, 1);
    }

    #[test]
    fn matches_legacy_index_entries_by_title() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("bookshelf");
        let chapter = root.join("婚纱之中待到花火散去").join("第01话");
        fs::create_dir_all(&chapter).expect("chapter");
        fs::write(chapter.join("001.jpg"), b"image").expect("page");
        let db_path = temp.path().join("library.sqlite");

        let db = LibraryDatabase::open(&db_path).expect("db");
        db.migrate().expect("migrate");
        let mut legacy = Comic::from_local_directory("占位", temp.path().join("unused"));
        legacy.id = "cp:hzzsddhhshct".to_string();
        legacy.name = "婚纱之中待到花火散去".to_string();
        legacy.location = "婚纱之中待到花火散去".to_string();
        legacy.local_path = None;
        legacy.scan_status = ScanStatus::Imported;
        db.upsert_comic(&legacy).expect("seed");
        drop(db);

        let scanned = scan_library(&root, Some(&db_path)).expect("scan");
        assert_eq!(scanned.comics.len(), 1);
        assert_eq!(scanned.comics[0].id, "cp:hzzsddhhshct");
        assert_eq!(scanned.comics[0].scan_status, ScanStatus::Matched);
        assert_eq!(scanned.added, 0);
        assert_eq!(scanned.updated, 1);
    }

    #[test]
    fn load_chapters_reuses_index_until_forced() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("bookshelf");
        let chapter = root.join("索引书").join("第01话");
        fs::create_dir_all(&chapter).expect("chapter");
        fs::write(chapter.join("001.jpg"), b"image").expect("page");
        let db_path = temp.path().join("library.sqlite");
        let scanned = scan_library(&root, Some(&db_path)).expect("scan");
        let comic_id = scanned.comics[0].id.clone();
        let db = LibraryDatabase::open(&db_path).expect("db");
        db.migrate().expect("migrate");
        let mut stored = db.list_chapters_for_comic(&comic_id).expect("stored");
        stored[0].page_count = 99;
        db.replace_chapters_for_comic(&comic_id, &stored)
            .expect("tamper");
        drop(db);

        let cached = load_or_scan_chapters(
            &comic_id,
            root.join("索引书"),
            Some(&db_path),
            false,
        )
        .expect("cached");
        assert_eq!(cached[0].page_count, 99);

        let forced = load_or_scan_chapters(
            &comic_id,
            root.join("索引书"),
            Some(&db_path),
            true,
        )
        .expect("forced");
        assert_eq!(forced[0].page_count, 1);
    }

    #[test]
    fn deletes_missing_comic_from_index_only() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("bookshelf");
        let chapter = root.join("可删").join("第01话");
        fs::create_dir_all(&chapter).expect("chapter");
        fs::write(chapter.join("001.jpg"), b"image").expect("page");
        let db_path = temp.path().join("library.sqlite");
        let scanned = scan_library(&root, Some(&db_path)).expect("scan");
        let comic_id = scanned.comics[0].id.clone();
        fs::remove_dir_all(root.join("可删")).expect("remove files");
        let after = scan_library(&root, Some(&db_path)).expect("missing");
        assert_eq!(after.missing, 1);
        delete_library_comic(&db_path, &comic_id).expect("delete");
        let loaded = load_library(&root, Some(&db_path)).expect("load");
        assert!(loaded.comics.is_empty());
        assert!(!root.join("可删").exists());
    }

    #[test]
    fn load_does_not_create_missing_bookshelf() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("missing-shelf");
        let loaded = load_library(&root, None::<&Path>).expect("load");
        assert!(!root.exists());
        assert!(loaded.comics.is_empty());
    }

    #[test]
    fn extra_roots_with_same_title_stay_separate() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("main");
        let extra = temp.path().join("extra");
        let main_chapter = root.join("同名").join("第01话");
        let extra_chapter = extra.join("同名").join("第01话");
        fs::create_dir_all(&main_chapter).expect("main");
        fs::create_dir_all(&extra_chapter).expect("extra");
        fs::write(main_chapter.join("001.jpg"), b"image").expect("main page");
        fs::write(extra_chapter.join("001.jpg"), b"image").expect("extra page");
        let db_path = temp.path().join("library.sqlite");

        let scanned = scan_library_with_progress(
            &root,
            Some(&db_path),
            &[extra.as_path()],
            |_| true,
        )
        .expect("scan");
        assert_eq!(scanned.added, 2);
        assert_eq!(scanned.comics.len(), 2);
        let paths: Vec<_> = scanned
            .comics
            .iter()
            .filter_map(|comic| comic.local_path.as_ref())
            .collect();
        assert_eq!(paths.len(), 2);
        assert_ne!(paths[0], paths[1]);
    }

    #[test]
    fn scan_includes_extra_bookshelf_roots() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("main");
        let extra = temp.path().join("extra");
        let main_chapter = root.join("主库漫画").join("第01话");
        let extra_chapter = extra.join("额外漫画").join("第01话");
        fs::create_dir_all(&main_chapter).expect("main");
        fs::create_dir_all(&extra_chapter).expect("extra");
        fs::write(main_chapter.join("001.jpg"), b"image").expect("main page");
        fs::write(extra_chapter.join("001.jpg"), b"image").expect("extra page");
        let db_path = temp.path().join("library.sqlite");

        let scanned = scan_library_with_progress(
            &root,
            Some(&db_path),
            &[extra.as_path()],
            |_| true,
        )
        .expect("scan extras");
        assert_eq!(scanned.added, 2);
        let names: Vec<_> = scanned
            .comics
            .iter()
            .map(|comic| comic.name.as_str())
            .collect();
        assert!(names.contains(&"主库漫画"));
        assert!(names.contains(&"额外漫画"));
    }

    #[test]
    fn rescan_picks_up_new_pages_when_chapter_changes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("bookshelf");
        let chapter = root.join("变页书").join("第01话");
        fs::create_dir_all(&chapter).expect("chapter");
        fs::write(chapter.join("001.jpg"), b"image").expect("page");
        let db_path = temp.path().join("library.sqlite");
        let first = scan_library(&root, Some(&db_path)).expect("first");
        assert_eq!(first.comics[0].image_count, 1);

        fs::write(chapter.join("002.jpg"), b"image").expect("new page");
        let again = scan_library(&root, Some(&db_path)).expect("rescan");
        assert_eq!(again.comics[0].image_count, 2);
        assert_eq!(again.updated, 1);
    }

    #[test]
    fn scan_records_new_book_and_new_chapter_notes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("bookshelf");
        let chapter = root.join("连载书").join("第01话");
        fs::create_dir_all(&chapter).expect("chapter");
        fs::write(chapter.join("001.jpg"), b"image").expect("page");
        let db_path = temp.path().join("library.sqlite");

        let first = scan_library(&root, Some(&db_path)).expect("first");
        assert_eq!(first.comics[0].shelf_update_note.as_deref(), Some("新书"));
        assert!(first.comics[0].shelf_updated_at.is_some());

        let second_chapter = root.join("连载书").join("第02话");
        fs::create_dir_all(&second_chapter).expect("chapter 2");
        fs::write(second_chapter.join("001.jpg"), b"image").expect("page");
        let again = scan_library(&root, Some(&db_path)).expect("rescan");
        assert_eq!(again.updated, 1);
        let note = again.comics[0].shelf_update_note.clone().unwrap_or_default();
        assert!(
            note.contains("更新至") || note.contains("新增"),
            "{note}"
        );
    }

    #[test]
    fn scan_progress_can_stop_early() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("bookshelf");
        for name in ["甲", "乙", "丙"] {
            let chapter = root.join(name).join("第01话");
            fs::create_dir_all(&chapter).expect("chapter");
            fs::write(chapter.join("001.jpg"), b"image").expect("page");
        }
        let db_path = temp.path().join("library.sqlite");
        let result =
            scan_library_with_progress(&root, Some(&db_path), &[] as &[&Path], |progress| progress.scanned < 1)
                .expect("scan");
        assert_eq!(result.added, 1);
        assert!(result.comics.len() <= 2);
    }
}
