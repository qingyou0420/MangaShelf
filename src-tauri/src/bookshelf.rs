use crate::domain::LocalManga;
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
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
}
