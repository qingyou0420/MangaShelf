use crate::domain::{Chapter, ChapterKind, LocalManga};
use anyhow::Result;
use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn scan_bookshelf(root: impl AsRef<Path>) -> Result<Vec<LocalManga>> {
    let root = root.as_ref();
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut mangas = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let title = entry.file_name().to_string_lossy().trim().to_string();
        if title.is_empty() {
            continue;
        }

        let (chapter_count, image_count) = inspect_manga_directory(&path)?;
        if image_count > 0 {
            mangas.push(LocalManga {
                title,
                directory: path,
                chapter_count,
                image_count,
            });
        }
    }

    mangas.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(mangas)
}

pub fn match_local_manga(title: &str, library: &[LocalManga]) -> Option<LocalManga> {
    let target = normalize_title(title);
    library
        .iter()
        .find(|manga| normalize_title(&manga.title) == target)
        .cloned()
}

pub fn scan_comic_chapters(comic_id: &str, comic_dir: impl AsRef<Path>) -> Result<Vec<Chapter>> {
    let comic_dir = comic_dir.as_ref();
    if !comic_dir.exists() {
        return Ok(Vec::new());
    }

    let mut chapters = Vec::new();
    for entry in fs::read_dir(comic_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let title = entry.file_name().to_string_lossy().trim().to_string();
        if title.is_empty() {
            continue;
        }

        let page_count = count_images_recursive(&path)?;
        if page_count == 0 {
            continue;
        }

        let special_kind = classify_chapter_kind(&title);
        chapters.push(Chapter {
            id: format!("{comic_id}::{title}"),
            comic_id: comic_id.to_string(),
            title: title.clone(),
            path,
            ordinal: chapter_ordinal(&title),
            page_count,
            read_progress_page: 0,
            special_kind,
        });
    }

    chapters.sort_by(|a, b| {
        chapter_kind_rank(a.special_kind)
            .cmp(&chapter_kind_rank(b.special_kind))
            .then_with(|| {
                a.ordinal
                    .unwrap_or(f32::MAX)
                    .partial_cmp(&b.ordinal.unwrap_or(f32::MAX))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.title.cmp(&b.title))
    });

    Ok(chapters)
}

pub fn list_chapter_pages(chapter_dir: impl AsRef<Path>) -> Result<Vec<String>> {
    let chapter_dir = chapter_dir.as_ref();
    if !chapter_dir.exists() {
        return Ok(Vec::new());
    }

    let mut pages = Vec::new();
    collect_image_pages(chapter_dir, &mut pages)?;
    pages.sort_by(|a, b| compare_page_paths(a.as_path(), b.as_path()));

    Ok(pages
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect())
}

pub fn find_first_image_page(path: impl AsRef<Path>) -> Result<Option<PathBuf>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(None);
    }

    let mut pages = Vec::new();
    collect_image_pages(path, &mut pages)?;
    pages.sort_by(|a, b| compare_page_paths(a.as_path(), b.as_path()));
    Ok(pages.into_iter().next())
}

pub fn manga_directory_fingerprint(path: impl AsRef<Path>) -> Result<String> {
    let root = path.as_ref();
    if !root.exists() {
        return Ok(String::new());
    }

    let mut entries = Vec::new();
    collect_fingerprint_entries(root, root, &mut entries)?;
    entries.sort();
    Ok(entries.join("\n"))
}

fn collect_image_pages(path: &Path, pages: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let child = entry?.path();
        if child.is_dir() {
            collect_image_pages(&child, pages)?;
        } else if child.is_file() && is_image_file(&child) {
            pages.push(child);
        }
    }

    Ok(())
}

fn collect_fingerprint_entries(root: &Path, path: &Path, entries: &mut Vec<String>) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let child = entry?.path();
        if child.is_dir() {
            collect_fingerprint_entries(root, &child, entries)?;
        } else if child.is_file() && is_image_file(&child) {
            let metadata = fs::metadata(&child)?;
            let relative = child.strip_prefix(root).unwrap_or(child.as_path());
            entries.push(format!(
                "{}|{}|{}",
                relative.to_string_lossy().replace('\\', "/"),
                metadata.len(),
                modified_tick(&metadata)
            ));
        }
    }
    Ok(())
}

fn modified_tick(metadata: &fs::Metadata) -> u128 {
    metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn compare_page_paths(a: &Path, b: &Path) -> Ordering {
    let a_name = a
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let b_name = b
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    natural_cmp(a_name, b_name).then_with(|| a.to_string_lossy().cmp(&b.to_string_lossy()))
}

fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_index = 0;
    let mut b_index = 0;

    loop {
        let a_token = next_natural_token(a, a_index);
        let b_token = next_natural_token(b, b_index);

        let (a_digits, a_part, next_a) = match a_token {
            Some(token) => token,
            None => {
                return if b_token.is_none() {
                    Ordering::Equal
                } else {
                    Ordering::Less
                }
            }
        };
        let (b_digits, b_part, next_b) = match b_token {
            Some(token) => token,
            None => return Ordering::Greater,
        };

        let ordering = if a_digits && b_digits {
            compare_number_tokens(a_part, b_part)
        } else {
            a_part
                .to_ascii_lowercase()
                .cmp(&b_part.to_ascii_lowercase())
        };

        if ordering != Ordering::Equal {
            return ordering;
        }

        a_index = next_a;
        b_index = next_b;
    }
}

fn next_natural_token(value: &str, start: usize) -> Option<(bool, &str, usize)> {
    let bytes = value.as_bytes();
    if start >= bytes.len() {
        return None;
    }

    let is_digit = bytes[start].is_ascii_digit();
    let mut end = start + 1;
    while end < bytes.len() && bytes[end].is_ascii_digit() == is_digit {
        end += 1;
    }

    Some((is_digit, &value[start..end], end))
}

fn compare_number_tokens(a: &str, b: &str) -> Ordering {
    let a_trimmed = a.trim_start_matches('0');
    let b_trimmed = b.trim_start_matches('0');
    let a_number = if a_trimmed.is_empty() { "0" } else { a_trimmed };
    let b_number = if b_trimmed.is_empty() { "0" } else { b_trimmed };

    a_number
        .len()
        .cmp(&b_number.len())
        .then_with(|| a_number.cmp(b_number))
        .then_with(|| a.len().cmp(&b.len()))
}

fn inspect_manga_directory(path: &Path) -> Result<(usize, usize)> {
    let mut chapter_count = 0;
    let mut image_count = 0;
    let direct_images = count_images_in_directory(path)?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let chapter_path = entry.path();
        if !chapter_path.is_dir() {
            continue;
        }

        let images = count_images_recursive(&chapter_path)?;
        if images > 0 {
            chapter_count += 1;
            image_count += images;
        }
    }

    if image_count == 0 && direct_images > 0 {
        chapter_count = 1;
        image_count = direct_images;
    }

    Ok((chapter_count, image_count))
}

fn count_images_recursive(path: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            count += count_images_recursive(&child)?;
        } else if is_image_file(&child) {
            count += 1;
        }
    }
    Ok(count)
}

fn count_images_in_directory(path: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(path)? {
        let child: PathBuf = entry?.path();
        if child.is_file() && is_image_file(&child) {
            count += 1;
        }
    }
    Ok(count)
}

fn is_image_file(path: &Path) -> bool {
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

fn normalize_title(title: &str) -> String {
    title
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn classify_chapter_kind(title: &str) -> ChapterKind {
    if title.contains("机翻") || title.contains("機翻") {
        ChapterKind::MachineTranslation
    } else if title.contains('卷') {
        ChapterKind::Volume
    } else if title.starts_with('第') && title.contains('话') {
        ChapterKind::Regular
    } else {
        ChapterKind::Other
    }
}

fn chapter_kind_rank(kind: ChapterKind) -> u8 {
    match kind {
        ChapterKind::Regular => 0,
        ChapterKind::MachineTranslation => 1,
        ChapterKind::Volume => 2,
        ChapterKind::Other => 3,
    }
}

fn chapter_ordinal(title: &str) -> Option<f32> {
    let digits: String = title
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit())
        .collect();

    if digits.is_empty() {
        None
    } else {
        digits.parse::<f32>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_chapter_folders_counts_images_and_matches_local_manga() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manga = temp.path().join("孤独摇滚");
        let chapter = manga.join("第001话");
        fs::create_dir_all(&chapter).expect("chapter dir");
        fs::write(chapter.join("001.jpg"), b"image").expect("jpg");
        fs::write(chapter.join("002.png"), b"image").expect("png");
        fs::write(chapter.join("readme.txt"), b"skip").expect("txt");

        let library = scan_bookshelf(temp.path()).expect("bookshelf scan");
        let found = match_local_manga("孤独摇滚", &library).expect("local match");

        assert_eq!(found.title, "孤独摇滚");
        assert_eq!(found.chapter_count, 1);
        assert_eq!(found.image_count, 2);
    }

    #[test]
    fn scans_chapter_records_and_classifies_special_kinds() {
        let temp = tempfile::tempdir().expect("tempdir");
        let comic_dir = temp.path().join("测试漫画");
        for name in ["第01话", "第6话", "第10话", "第01卷机翻", "第02卷", "番外"] {
            let chapter = comic_dir.join(name);
            fs::create_dir_all(&chapter).expect("chapter dir");
            fs::write(chapter.join("001.jpg"), b"image").expect("image");
        }

        let chapters = scan_comic_chapters("cp:test", &comic_dir).expect("chapter scan");

        assert_eq!(chapters.len(), 6);
        assert_eq!(chapters[0].title, "第01话");
        assert_eq!(chapters[0].special_kind, ChapterKind::Regular);
        assert_eq!(chapters[1].title, "第6话");
        assert_eq!(chapters[1].special_kind, ChapterKind::Regular);
        assert_eq!(chapters[2].title, "第10话");
        assert_eq!(chapters[2].special_kind, ChapterKind::Regular);
        assert_eq!(chapters[3].special_kind, ChapterKind::MachineTranslation);
        assert_eq!(chapters[4].special_kind, ChapterKind::Volume);
        assert_eq!(chapters[5].special_kind, ChapterKind::Other);
        assert_eq!(chapters[0].read_progress_page, 0);
    }

    #[test]
    fn lists_chapter_pages_in_reading_order() {
        let temp = tempfile::tempdir().expect("tempdir");
        let chapter = temp.path().join("第01话");
        let nested = chapter.join("extra");
        fs::create_dir_all(&nested).expect("chapter dirs");
        fs::write(chapter.join("10.png"), b"image").expect("image");
        fs::write(chapter.join("2.png"), b"image").expect("image");
        fs::write(chapter.join("1.jpg"), b"image").expect("image");
        fs::write(chapter.join("notes.txt"), b"skip").expect("text");
        fs::write(nested.join("11.webp"), b"image").expect("image");

        let pages = list_chapter_pages(&chapter).expect("list pages");

        assert_eq!(pages.len(), 4);
        assert!(pages[0].ends_with("1.jpg"));
        assert!(pages[1].ends_with("2.png"));
        assert!(pages[2].ends_with("10.png"));
        assert!(pages[3].ends_with("11.webp"));
    }

    #[test]
    fn finds_first_image_page_for_local_cover_fallback() {
        let temp = tempfile::tempdir().expect("tempdir");
        let chapter = temp.path().join("chapter-01");
        fs::create_dir_all(&chapter).expect("chapter dir");
        fs::write(chapter.join("010.jpg"), b"image").expect("late page");
        fs::write(chapter.join("001.png"), b"image").expect("first page");

        let cover = find_first_image_page(temp.path())
            .expect("find cover")
            .expect("cover page");

        assert_eq!(
            cover.file_name().and_then(|name| name.to_str()),
            Some("001.png")
        );
    }

    #[test]
    fn manga_directory_fingerprint_changes_when_chapter_contents_change() {
        let temp = tempfile::tempdir().expect("tempdir");
        let chapter = temp.path().join("chapter-01");
        fs::create_dir_all(&chapter).expect("chapter dir");
        fs::write(chapter.join("001.jpg"), b"image").expect("first page");

        let before = manga_directory_fingerprint(temp.path()).expect("initial fingerprint");
        fs::write(chapter.join("002.jpg"), b"image").expect("second page");
        let after = manga_directory_fingerprint(temp.path()).expect("changed fingerprint");

        assert_ne!(before, after);
    }
}
