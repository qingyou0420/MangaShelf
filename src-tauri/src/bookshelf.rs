use crate::domain::{CacheStats, Chapter, ChapterKind, ExtractProgress, LocalManga};
use anyhow::Result;
use std::{
    cmp::Ordering,
    fs,
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const SKIP_DIR_NAMES: &[&str] = &[".manga-library", ".mangacon-companion"];

pub fn scan_bookshelf(root: impl AsRef<Path>) -> Result<Vec<LocalManga>> {
    let root = root.as_ref();
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut mangas = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let title = entry.file_name().to_string_lossy().trim().to_string();
        if title.is_empty() || should_skip_name(&title) {
            continue;
        }

        if path.is_dir() {
            mangas.push(LocalManga {
                cheap_signature: cheap_signature(&path).unwrap_or_default(),
                title,
                directory: path,
                chapter_count: 0,
                image_count: 0,
            });
            continue;
        }

        if path.is_file() && is_archive_file(&path) {
            mangas.push(LocalManga {
                cheap_signature: cheap_signature(&path).unwrap_or_default(),
                title: file_stem_title(&path).unwrap_or(title),
                directory: path,
                chapter_count: 0,
                image_count: 0,
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

pub fn cheap_signature(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let metadata = fs::metadata(path)?;
    if path.is_file() {
        return Ok(format!(
            "file|{}|{}",
            metadata.len(),
            modified_tick(&metadata)
        ));
    }

    let mut parts = vec![format!(
        "dir|{}|{}",
        metadata.len(),
        modified_tick(&metadata)
    )];
    if path.is_dir() {
        let mut children = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if should_skip_name(&name) {
                continue;
            }
            let child = entry.path();
            let child_meta = match fs::metadata(&child) {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            let kind = if child.is_dir() { "d" } else { "f" };
            children.push(format!(
                "{kind}|{name}|{}|{}",
                child_meta.len(),
                modified_tick(&child_meta)
            ));
            if child.is_dir() {
                if let Ok(nested) = fs::read_dir(&child) {
                    for nested_entry in nested.flatten() {
                        let nested_name = nested_entry.file_name().to_string_lossy().to_string();
                        if should_skip_name(&nested_name) {
                            continue;
                        }
                        let nested_path = nested_entry.path();
                        if !nested_path.is_file() {
                            continue;
                        }
                        let Ok(nested_meta) = fs::metadata(&nested_path) else {
                            continue;
                        };
                        children.push(format!(
                            "f|{name}/{nested_name}|{}|{}",
                            nested_meta.len(),
                            modified_tick(&nested_meta)
                        ));
                    }
                }
            }
        }
        children.sort();
        parts.extend(children);
    }
    Ok(parts.join("\n"))
}

pub fn scan_comic_chapters(comic_id: &str, comic_dir: impl AsRef<Path>) -> Result<Vec<Chapter>> {
    scan_comic_chapters_reusing(comic_id, comic_dir, &[])
}

pub fn scan_comic_chapters_reusing(
    comic_id: &str,
    comic_dir: impl AsRef<Path>,
    stored: &[Chapter],
) -> Result<Vec<Chapter>> {
    let comic_dir = comic_dir.as_ref();
    if !comic_dir.exists() {
        return Ok(Vec::new());
    }

    if comic_dir.is_file() && is_archive_file(comic_dir) {
        let page_count = page_count_reusing(comic_dir, stored, true)?;
        if page_count == 0 {
            return Ok(Vec::new());
        }
        let title = file_stem_title(comic_dir).unwrap_or_else(|| "全一册".to_string());
        return Ok(vec![chapter_record(
            comic_id,
            title,
            comic_dir.to_path_buf(),
            page_count,
            comic_dir,
        )]);
    }

    if !comic_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut chapters = Vec::new();
    for entry in fs::read_dir(comic_dir)? {
        let entry = entry?;
        let path = entry.path();
        let title = entry.file_name().to_string_lossy().trim().to_string();
        if title.is_empty() || should_skip_name(&title) {
            continue;
        }

        if path.is_dir() {
            let page_count = page_count_reusing(&path, stored, false)?;
            if page_count == 0 {
                continue;
            }
            chapters.push(chapter_record(comic_id, title, path, page_count, comic_dir));
        } else if path.is_file() && is_archive_file(&path) {
            let page_count = page_count_reusing(&path, stored, true)?;
            if page_count == 0 {
                continue;
            }
            let archive_title = file_stem_title(&path).unwrap_or(title);
            chapters.push(chapter_record(
                comic_id,
                archive_title,
                path,
                page_count,
                comic_dir,
            ));
        }
    }

    if chapters.is_empty() {
        let page_count = count_images_in_directory(comic_dir)?;
        if page_count > 0 {
            let title = comic_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("全一册")
                .to_string();
            chapters.push(chapter_record(
                comic_id,
                title,
                comic_dir.to_path_buf(),
                page_count,
                comic_dir,
            ));
        }
    }

    chapters.sort_by(|a, b| {
        chapter_kind_rank(a.special_kind)
            .cmp(&chapter_kind_rank(b.special_kind))
            .then_with(|| {
                a.ordinal
                    .unwrap_or(f32::MAX)
                    .partial_cmp(&b.ordinal.unwrap_or(f32::MAX))
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| natural_cmp(&a.title, &b.title))
    });

    Ok(chapters)
}

pub fn list_chapter_pages(chapter_path: impl AsRef<Path>) -> Result<Vec<String>> {
    list_chapter_pages_with_progress(chapter_path, None::<&Path>, |_| {})
}

pub fn list_chapter_pages_with_progress(
    chapter_path: impl AsRef<Path>,
    bookshelf_root: Option<impl AsRef<Path>>,
    on_progress: impl FnMut(crate::domain::ExtractProgress),
) -> Result<Vec<String>> {
    let chapter_path = chapter_path.as_ref();
    if !chapter_path.exists() {
        return Ok(Vec::new());
    }

    if chapter_path.is_file() && is_archive_file(chapter_path) {
        let root = bookshelf_root.as_ref().map(|path| path.as_ref());
        return list_archive_pages(chapter_path, root, on_progress);
    }

    let mut pages = Vec::new();
    collect_image_pages(chapter_path, &mut pages)?;
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

    if path.is_file() && is_archive_file(path) {
        return extract_first_archive_image(path, path.parent());
    }

    let mut pages = Vec::new();
    collect_image_pages(path, &mut pages)?;
    pages.sort_by(|a, b| compare_page_paths(a.as_path(), b.as_path()));
    if let Some(page) = pages.into_iter().next() {
        return Ok(Some(page));
    }

    if path.is_dir() {
        let mut archives = Vec::new();
        for entry in fs::read_dir(path)? {
            let child = entry?.path();
            if child.is_file() && is_archive_file(&child) {
                archives.push(child);
            }
        }
        archives.sort_by(|a, b| compare_page_paths(a, b));
        for archive in archives {
            if let Some(page) = extract_first_archive_image(&archive, Some(path))? {
                return Ok(Some(page));
            }
        }
    }

    Ok(None)
}

pub fn manga_directory_fingerprint(path: impl AsRef<Path>) -> Result<String> {
    Ok(digest_v2(&listing_fingerprint(path)?))
}

pub fn listing_fingerprint(path: impl AsRef<Path>) -> Result<String> {
    let root = path.as_ref();
    if !root.exists() {
        return Ok(String::new());
    }

    if root.is_file() && is_archive_file(root) {
        return archive_fingerprint(root);
    }

    let mut entries = Vec::new();
    collect_fingerprint_entries(root, root, &mut entries)?;
    entries.sort();
    Ok(entries.join("\n"))
}

pub fn digest_v2(listing: &str) -> String {
    format!("v2:{:016x}", fnv1a64(listing.as_bytes()))
}

fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn page_count_reusing(path: &Path, stored: &[Chapter], archive: bool) -> Result<usize> {
    let listing = listing_fingerprint(path)?;
    let digest = digest_v2(&listing);
    if let Some(previous) = stored.iter().find(|chapter| {
        chapter.fingerprint.as_deref() == Some(digest.as_str())
            && same_entry(&chapter.path, path)
    }) {
        return Ok(previous.page_count);
    }
    if archive {
        count_images_in_archive(path)
    } else if path.is_dir() {
        count_images_recursive(path)
    } else {
        Ok(0)
    }
}

fn same_entry(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .to_string_lossy()
            .replace('\\', "/")
            .eq_ignore_ascii_case(&right.to_string_lossy().replace('\\', "/"))
}

fn chapter_record(
    comic_id: &str,
    title: String,
    path: PathBuf,
    page_count: usize,
    comic_dir: &Path,
) -> Chapter {
    let relative = path
        .strip_prefix(comic_dir)
        .ok()
        .map(|value| value.to_string_lossy().replace('\\', "/"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ".".into());
    let fingerprint = listing_fingerprint(&path).ok().map(|listing| digest_v2(&listing));
    Chapter {
        id: format!("{comic_id}::{relative}"),
        comic_id: comic_id.to_string(),
        title: title.clone(),
        path,
        ordinal: chapter_ordinal(&title),
        page_count,
        read_progress_page: 0,
        special_kind: classify_chapter_kind(&title),
        fingerprint,
    }
}

fn collect_image_pages(path: &Path, pages: &mut Vec<PathBuf>) -> Result<()> {
    if !path.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let child = entry?.path();
        let name = child
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if child.is_dir() {
            if should_skip_name(name) {
                continue;
            }
            collect_image_pages(&child, pages)?;
        } else if child.is_file() && is_image_file(&child) {
            pages.push(child);
        }
    }
    Ok(())
}

fn collect_fingerprint_entries(root: &Path, path: &Path, entries: &mut Vec<String>) -> Result<()> {
    if path.is_file() && is_archive_file(path) {
        entries.push(archive_fingerprint(path)?);
        return Ok(());
    }
    if !path.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let child = entry?.path();
        let name = child
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if should_skip_name(name) {
            continue;
        }
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
        } else if child.is_file() && is_archive_file(&child) {
            entries.push(archive_fingerprint(&child)?);
        }
    }
    Ok(())
}

fn archive_fingerprint(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path)?;
    Ok(format!(
        "{}|{}|{}",
        path.to_string_lossy().replace('\\', "/"),
        metadata.len(),
        modified_tick(&metadata)
    ))
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

#[cfg(test)]
fn inspect_manga_directory(path: &Path) -> Result<(usize, usize)> {
    let mut chapter_count = 0;
    let mut image_count = 0;
    let direct_images = count_images_in_directory(path)?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip_name(&name) {
            continue;
        }
        if child.is_dir() {
            let images = count_images_recursive(&child)?;
            if images > 0 {
                chapter_count += 1;
                image_count += images;
            }
        } else if child.is_file() && is_archive_file(&child) {
            let images = count_images_in_archive(&child)?;
            if images > 0 {
                chapter_count += 1;
                image_count += images;
            }
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
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip_name(&name) {
            continue;
        }
        if child.is_dir() {
            count += count_images_recursive(&child)?;
        } else if is_image_file(&child) {
            count += 1;
        } else if is_archive_file(&child) {
            count += count_images_in_archive(&child)?;
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

pub fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "avif"
            )
        })
}

pub fn is_archive_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "zip" | "cbz" | "rar" | "cbr"
            )
        })
}

pub fn is_zip_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "zip" | "cbz"))
}

pub fn is_rar_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "rar" | "cbr"))
}

enum RarCli {
    SevenZip(PathBuf),
    Unrar(PathBuf),
}

fn find_rar_cli() -> Option<RarCli> {
    const SEVEN_ZIP: &[&str] = &[
        r"C:\Program Files\7-Zip\7z.exe",
        r"C:\Program Files (x86)\7-Zip\7z.exe",
    ];
    for candidate in SEVEN_ZIP {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some(RarCli::SevenZip(path));
        }
    }
    if let Some(path) = lookup_on_path("7z.exe").or_else(|| lookup_on_path("7z")) {
        return Some(RarCli::SevenZip(path));
    }

    const UNRAR: &[&str] = &[
        r"C:\Program Files\WinRAR\UnRAR.exe",
        r"C:\Program Files (x86)\WinRAR\UnRAR.exe",
        r"C:\Program Files\WinRAR\WinRAR.exe",
        r"C:\Program Files (x86)\WinRAR\WinRAR.exe",
    ];
    for candidate in UNRAR {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some(RarCli::Unrar(path));
        }
    }
    lookup_on_path("UnRAR.exe")
        .or_else(|| lookup_on_path("unrar"))
        .map(RarCli::Unrar)
}

fn lookup_on_path(name: &str) -> Option<PathBuf> {
    let output = run_hidden(Command::new("where").arg(name)).ok()?;
    if !output.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .trim()
        .to_string();
    let path = PathBuf::from(line);
    path.is_file().then_some(path)
}

fn run_hidden(command: &mut Command) -> io::Result<std::process::Output> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.output()
}

fn rar_unavailable() -> anyhow::Error {
    anyhow::anyhow!("无法解压 rar/cbr：未找到 7-Zip 或 WinRAR。安装其中之一后即可阅读。")
}

fn list_rar_image_names(archive_path: &Path) -> Result<Vec<String>> {
    let Some(cli) = find_rar_cli() else {
        return Err(rar_unavailable());
    };
    let output = match &cli {
        RarCli::SevenZip(tool) => run_hidden(
            Command::new(tool)
                .arg("l")
                .arg("-slt")
                .arg("-ba")
                .arg(archive_path),
        )?,
        RarCli::Unrar(tool) => run_hidden(Command::new(tool).arg("lb").arg(archive_path))?,
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "列出 rar 内容失败：{}",
            stderr.trim().chars().take(180).collect::<String>()
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let names = match cli {
        RarCli::SevenZip(_) => parse_seven_zip_list(&stdout),
        RarCli::Unrar(_) => stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && is_image_name(line))
            .map(str::to_string)
            .collect(),
    };
    Ok(names)
}

fn parse_seven_zip_list(stdout: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut current_path: Option<String> = None;
    let mut is_folder = false;
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            if let Some(path) = current_path.take() {
                if !is_folder && is_image_name(&path) {
                    names.push(path);
                }
            }
            is_folder = false;
            continue;
        }
        if let Some(value) = line.strip_prefix("Path = ") {
            current_path = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("Folder = ") {
            is_folder = value != "-";
        }
    }
    if let Some(path) = current_path {
        if !is_folder && is_image_name(&path) {
            names.push(path);
        }
    }
    names
}

fn extract_rar_archive(archive_path: &Path, dest: &Path) -> Result<()> {
    let Some(cli) = find_rar_cli() else {
        return Err(rar_unavailable());
    };
    fs::create_dir_all(dest)?;
    let output = match cli {
        RarCli::SevenZip(tool) => run_hidden(
            Command::new(tool)
                .arg("x")
                .arg("-y")
                .arg(format!("-o{}", dest.display()))
                .arg(archive_path),
        )?,
        RarCli::Unrar(tool) => run_hidden(
            Command::new(tool)
                .arg("x")
                .arg("-y")
                .arg(archive_path)
                .arg(dest),
        )?,
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "解压 rar/cbr 失败：{}",
            stderr.trim().chars().take(180).collect::<String>()
        );
    }
    Ok(())
}

fn extract_first_rar_image(
    archive_path: &Path,
    dest_parent: Option<&Path>,
) -> Result<Option<PathBuf>> {
    let mut names = list_rar_image_names(archive_path)?;
    names.sort_by(|a, b| natural_cmp(a, b));
    let Some(name) = names.into_iter().next() else {
        return Ok(None);
    };
    let parent = dest_parent.unwrap_or_else(|| Path::new("."));
    let out_dir = parent.join(".manga-library").join("cover-preview");
    fs::create_dir_all(&out_dir)?;
    let file_name = Path::new(&name)
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cover.jpg"));
    let out_path = out_dir.join(format!(
        "{:016x}-{}",
        fnv1a64(archive_path.to_string_lossy().as_bytes()),
        sanitize_component(&file_name.to_string_lossy())
    ));
    if out_path.is_file() {
        return Ok(Some(out_path));
    }
    let staging = out_dir.join(format!(
        "tmp-{:016x}",
        fnv1a64(archive_path.to_string_lossy().as_bytes())
    ));
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging)?;
    extract_rar_named(archive_path, &staging, &name)?;
    let extracted = find_extracted_file(&staging, &file_name.to_string_lossy())
        .or_else(|| find_first_file(&staging));
    if let Some(extracted) = extracted {
        if extracted != out_path {
            let _ = fs::copy(&extracted, &out_path);
        }
    }
    let _ = fs::remove_dir_all(&staging);
    if out_path.is_file() {
        Ok(Some(out_path))
    } else {
        Ok(None)
    }
}

fn extract_rar_named(archive_path: &Path, dest: &Path, name: &str) -> Result<()> {
    let Some(cli) = find_rar_cli() else {
        return Err(rar_unavailable());
    };
    let output = match cli {
        RarCli::SevenZip(tool) => run_hidden(
            Command::new(tool)
                .arg("e")
                .arg("-y")
                .arg(format!("-o{}", dest.display()))
                .arg(archive_path)
                .arg(name),
        )?,
        RarCli::Unrar(tool) => run_hidden(
            Command::new(tool)
                .arg("e")
                .arg("-y")
                .arg(archive_path)
                .arg(name)
                .arg(dest),
        )?,
    };
    if !output.status.success() {
        extract_rar_archive(archive_path, dest)?;
    }
    Ok(())
}

fn find_extracted_file(root: &Path, file_name: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(file_name))
            {
                return Some(path);
            }
        }
    }
    None
}

fn find_first_file(root: &Path) -> Option<PathBuf> {
    let mut pages = Vec::new();
    collect_image_pages(root, &mut pages).ok()?;
    pages.sort_by(|a, b| compare_page_paths(a, b));
    pages.into_iter().next()
}

fn should_skip_name(name: &str) -> bool {
    name.starts_with('.')
        || SKIP_DIR_NAMES
            .iter()
            .any(|skip| name.eq_ignore_ascii_case(skip))
}

fn file_stem_title(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string)
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
    } else if title.starts_with('第') && (title.contains('话') || title.contains('話')) {
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

fn count_images_in_archive(path: &Path) -> Result<usize> {
    if is_rar_archive(path) {
        return Ok(list_rar_image_names(path).map(|names| names.len()).unwrap_or(1));
    }
    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut count = 0;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_file() && is_image_name(entry.name()) {
            count += 1;
        }
    }
    Ok(count)
}

fn list_archive_pages(
    archive_path: &Path,
    bookshelf_root: Option<&Path>,
    on_progress: impl FnMut(ExtractProgress),
) -> Result<Vec<String>> {
    let cache = archive_extract_dir(archive_path, bookshelf_root)?;
    extract_archive_images(archive_path, &cache, on_progress)?;
    if let Some(root) = bookshelf_root {
        let _ = prune_extract_cache(root, None);
    }
    let mut pages = Vec::new();
    collect_image_pages(&cache, &mut pages)?;
    pages.sort_by(|a, b| compare_page_paths(a.as_path(), b.as_path()));
    Ok(pages
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect())
}

fn archive_extract_dir(archive_path: &Path, bookshelf_root: Option<&Path>) -> Result<PathBuf> {
    let metadata = fs::metadata(archive_path)?;
    let root = bookshelf_root
        .map(Path::to_path_buf)
        .or_else(|| archive_path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    Ok(root.join(".manga-library").join("extract").join(format!(
        "{:016x}",
        fnv1a64(
            format!(
                "{}|{}|{}",
                archive_path.to_string_lossy(),
                metadata.len(),
                modified_tick(&metadata)
            )
            .as_bytes(),
        ),
    )))
}

fn extract_first_archive_image(
    archive_path: &Path,
    dest_parent: Option<&Path>,
) -> Result<Option<PathBuf>> {
    if is_rar_archive(archive_path) {
        return extract_first_rar_image(archive_path, dest_parent);
    }
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut names: Vec<(usize, String)> = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_file() && is_image_name(entry.name()) {
            names.push((index, entry.name().to_string()));
        }
    }
    names.sort_by(|a, b| natural_cmp(&a.1, &b.1));
    let Some((index, name)) = names.into_iter().next() else {
        return Ok(None);
    };
    let parent = dest_parent.unwrap_or_else(|| Path::new("."));
    let out_dir = parent.join(".manga-library").join("cover-preview");
    fs::create_dir_all(&out_dir)?;
    let Some(relative) = sanitized_archive_relative(&name) else {
        return Ok(None);
    };
    let file_name = relative
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cover.jpg"));
    let out_path = out_dir.join(format!(
        "{:016x}-{}",
        fnv1a64(archive_path.to_string_lossy().as_bytes()),
        sanitize_component(&file_name.to_string_lossy())
    ));
    if !out_path.is_file() {
        let mut entry = archive.by_index(index)?;
        let mut output = fs::File::create(&out_path)?;
        io::copy(&mut entry, &mut output)?;
    }
    Ok(Some(out_path))
}

fn extract_archive_images(
    archive_path: &Path,
    cache: &Path,
    mut on_progress: impl FnMut(ExtractProgress),
) -> Result<()> {
    if cache.is_dir() && count_images_recursive(cache).unwrap_or(0) > 0 {
        return Ok(());
    }
    if cache.exists() {
        let _ = fs::remove_dir_all(cache);
    }
    fs::create_dir_all(cache)?;

    if is_rar_archive(archive_path) {
        extract_rar_archive(archive_path, cache)?;
        let total = count_images_recursive(cache).unwrap_or(0);
        on_progress(ExtractProgress {
            current: total,
            total,
        });
        return Ok(());
    }

    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let total = archive.len();
    let mut current = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if !entry.is_file() || !is_image_name(entry.name()) {
            continue;
        }
        let Some(relative) = sanitized_archive_relative(entry.name()) else {
            continue;
        };
        let out_path = cache.join(relative);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = fs::File::create(&out_path)?;
        io::copy(&mut entry, &mut output)?;
        current += 1;
        on_progress(ExtractProgress { current, total });
    }
    Ok(())
}

fn is_image_name(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "avif"
            )
        })
}

fn sanitized_archive_relative(name: &str) -> Option<PathBuf> {
    let mut parts = Vec::new();
    for part in name.replace('\\', "/").split('/') {
        if part.is_empty() || part == "." || part == ".." {
            continue;
        }
        parts.push(sanitize_component(part));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.into_iter().collect())
    }
}

fn sanitize_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ' ' | '[' | ']') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.trim().is_empty() {
        "item".to_string()
    } else {
        sanitized
    }
}

pub fn list_cover_candidates(comic_dir: impl AsRef<Path>) -> Result<Vec<String>> {
    let comic_dir = comic_dir.as_ref();
    let mut candidates = Vec::new();
    if let Some(first) = find_first_image_page(comic_dir)? {
        candidates.push(first.to_string_lossy().into_owned());
    }
    if comic_dir.is_dir() {
        let mut children = Vec::new();
        for entry in fs::read_dir(comic_dir)? {
            children.push(entry?.path());
        }
        children.sort_by(|a, b| compare_page_paths(a, b));
        for child in children {
            if candidates.len() >= 48 {
                break;
            }
            if let Some(page) = find_first_image_page(&child)? {
                let display = page.to_string_lossy().into_owned();
                if !candidates.iter().any(|existing| existing == &display) {
                    candidates.push(display);
                }
            }
        }
    }
    Ok(candidates)
}

const DEFAULT_CACHE_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

pub fn extract_cache_stats(root: impl AsRef<Path>) -> Result<CacheStats> {
    let folders = collect_extract_dirs(root.as_ref())?;
    let bytes = folders.iter().map(|item| item.1).sum();
    Ok(CacheStats {
        bytes,
        folders: folders.len(),
        freed_bytes: 0,
    })
}

pub fn prune_extract_cache(
    root: impl AsRef<Path>,
    max_bytes: Option<u64>,
) -> Result<CacheStats> {
    let limit = max_bytes.unwrap_or(DEFAULT_CACHE_LIMIT);
    let mut folders = collect_extract_dirs(root.as_ref())?;
    folders.sort_by_key(|item| item.2);
    let mut bytes: u64 = folders.iter().map(|item| item.1).sum();
    let mut freed = 0u64;
    for (path, size, _) in folders {
        if bytes <= limit {
            break;
        }
        if fs::remove_dir_all(&path).is_ok() {
            bytes = bytes.saturating_sub(size);
            freed += size;
        }
    }
    let remaining = collect_extract_dirs(root.as_ref())?;
    Ok(CacheStats {
        bytes: remaining.iter().map(|item| item.1).sum(),
        folders: remaining.len(),
        freed_bytes: freed,
    })
}

fn collect_extract_dirs(root: &Path) -> Result<Vec<(PathBuf, u64, SystemTime)>> {
    let extract = root.join(".manga-library").join("extract");
    let mut found = Vec::new();
    if !extract.is_dir() {
        return Ok(found);
    }
    for entry in fs::read_dir(&extract)? {
        let child = entry?.path();
        if child.is_dir() {
            let size = dir_size(&child).unwrap_or(0);
            let modified = fs::metadata(&child)
                .and_then(|meta| meta.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            found.push((child, size, modified));
        }
    }
    Ok(found)
}

fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0u64;
    if path.is_file() {
        return Ok(fs::metadata(path)?.len());
    }
    if !path.is_dir() {
        return Ok(0);
    }
    for entry in fs::read_dir(path)? {
        let child = entry?.path();
        if child.is_dir() {
            total += dir_size(&child)?;
        } else if child.is_file() {
            total += fs::metadata(&child)?.len();
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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
        let (chapter_count, image_count) =
            inspect_manga_directory(&found.directory).expect("inspect");
        assert_eq!(chapter_count, 1);
        assert_eq!(image_count, 2);
    }

    #[test]
    fn treats_flat_image_folder_as_single_chapter() {
        let temp = tempfile::tempdir().expect("tempdir");
        let comic_dir = temp.path().join("单册");
        fs::create_dir_all(&comic_dir).expect("comic dir");
        fs::write(comic_dir.join("02.png"), b"image").expect("page");
        fs::write(comic_dir.join("01.jpg"), b"image").expect("page");

        let chapters = scan_comic_chapters("local:flat", &comic_dir).expect("scan");
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].page_count, 2);
        assert_eq!(chapters[0].title, "单册");
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

        let chapters = scan_comic_chapters("local:test", &comic_dir).expect("chapter scan");

        assert_eq!(chapters.len(), 6);
        assert_eq!(chapters[0].title, "第01话");
        assert_eq!(chapters[0].special_kind, ChapterKind::Regular);
        assert_eq!(chapters[1].title, "第6话");
        assert_eq!(chapters[2].title, "第10话");
        assert_eq!(chapters[3].special_kind, ChapterKind::MachineTranslation);
        assert_eq!(chapters[4].special_kind, ChapterKind::Volume);
        assert_eq!(chapters[5].special_kind, ChapterKind::Other);
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

    #[test]
    fn cheap_signature_includes_immediate_children() {
        let temp = tempfile::tempdir().expect("tempdir");
        let comic = temp.path().join("漫画");
        let chapter = comic.join("第01话");
        fs::create_dir_all(&chapter).expect("chapter");
        fs::write(chapter.join("001.jpg"), b"image").expect("page");

        let before = cheap_signature(&comic).expect("before");
        let extra = comic.join("第02话");
        fs::create_dir_all(&extra).expect("new chapter");
        fs::write(extra.join("001.jpg"), b"image").expect("page");
        let after = cheap_signature(&comic).expect("after");
        assert_ne!(before, after);
        assert!(after.contains("第02话"));
    }

    #[test]
    fn cheap_signature_changes_when_chapter_file_is_replaced() {
        let temp = tempfile::tempdir().expect("tempdir");
        let comic = temp.path().join("漫画");
        let chapter = comic.join("第01话");
        fs::create_dir_all(&chapter).expect("chapter");
        fs::write(chapter.join("001.jpg"), b"image").expect("page");
        let before = cheap_signature(&comic).expect("before");
        fs::write(chapter.join("001.jpg"), b"image-replaced").expect("replace");
        let after = cheap_signature(&comic).expect("after");
        assert_ne!(before, after);
    }

    #[test]
    fn skips_hidden_cache_directories_during_scan() {
        let temp = tempfile::tempdir().expect("tempdir");
        let manga = temp.path().join("可见漫画");
        let chapter = manga.join("第01话");
        fs::create_dir_all(&chapter).expect("chapter");
        fs::write(chapter.join("001.jpg"), b"image").expect("page");
        let cache = temp.path().join(".manga-library").join("extract");
        fs::create_dir_all(&cache).expect("cache");
        fs::write(cache.join("001.jpg"), b"image").expect("cached page");

        let library = scan_bookshelf(temp.path()).expect("scan");
        assert_eq!(library.len(), 1);
        assert_eq!(library[0].title, "可见漫画");
    }

    #[test]
    fn treats_cbz_file_as_chapter_and_lists_extracted_pages() {
        let temp = tempfile::tempdir().expect("tempdir");
        let comic_dir = temp.path().join("压缩漫画");
        fs::create_dir_all(&comic_dir).expect("comic dir");
        let archive_path = comic_dir.join("第01话.cbz");
        write_zip_with_images(&archive_path, &["02.png", "01.jpg"]).expect("zip");

        let chapters = scan_comic_chapters("local:zip", &comic_dir).expect("scan");
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, "第01话");
        assert_eq!(chapters[0].page_count, 2);

        let pages = list_chapter_pages(&archive_path).expect("pages");
        assert_eq!(pages.len(), 2);
        assert!(pages[0].ends_with("01.jpg"));
        assert!(pages[1].ends_with("02.png"));
    }

    fn write_zip_with_images(path: &Path, names: &[&str]) -> Result<()> {
        let file = fs::File::create(path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for name in names {
            zip.start_file(*name, options)?;
            zip.write_all(b"image")?;
        }
        zip.finish()?;
        Ok(())
    }
}
