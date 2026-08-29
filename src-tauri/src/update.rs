//! App version and GitHub Release update checks.

use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

pub const GITHUB_OWNER: &str = "qingyou0420";
pub const GITHUB_REPO: &str = "MangaShelf";
const GITHUB_API: &str = "https://api.github.com/repos/qingyou0420/MangaShelf/releases/latest";
const USER_AGENT: &str = concat!("MangaShelf/", env!("CARGO_PKG_VERSION"));

#[cfg(not(windows))]
use std::process::Command;

/// Current app version from Cargo package metadata (kept in sync with tauri.conf.json).
pub fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalInstallerPackage {
    pub path: String,
    pub file_name: String,
    pub version: String,
    pub is_newer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalUpdateCheckResult {
    pub current_version: String,
    pub has_update: bool,
    pub latest: Option<LocalInstallerPackage>,
    pub packages: Vec<LocalInstallerPackage>,
    pub searched_dirs: Vec<String>,
}

/// Returns true when the file name looks like a MangaShelf / Manga Library installer.
pub fn is_manga_library_installer(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    if !lower.ends_with(".exe") {
        return false;
    }
    if !(lower.contains("setup") || lower.contains("installer")) {
        return false;
    }
    lower.contains("mangashelf")
        || lower.contains("manga-shelf")
        || lower.contains("manga shelf")
        || lower.contains("manga library")
        || lower.contains("manga-library")
        || lower.contains("mangalibrary")
}

/// Extract a `major.minor.patch` (optional 4th) token from an installer file name.
/// Prefers underscore-delimited segments (e.g. `Manga Library_1.1.0_x64-setup.exe`).
pub fn extract_version_from_filename(file_name: &str) -> Option<String> {
    let stem = file_name
        .strip_suffix(".exe")
        .or_else(|| file_name.strip_suffix(".EXE"))
        .unwrap_or(file_name);

    for part in stem.split('_') {
        let token = part.trim();
        if is_version_token(token) {
            return Some(token.to_string());
        }
    }

    find_semver_substring(stem)
}

fn is_version_token(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    (2..=4).contains(&parts.len())
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

fn find_semver_substring(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            let mut dots = 0;
            let mut j = i;
            while j < bytes.len() {
                let c = bytes[j];
                if c.is_ascii_digit() {
                    j += 1;
                } else if c == b'.' {
                    if j + 1 >= bytes.len() || !bytes[j + 1].is_ascii_digit() {
                        break;
                    }
                    dots += 1;
                    j += 1;
                } else {
                    break;
                }
            }
            if (2..=3).contains(&dots) {
                let candidate = &s[start..j];
                if is_version_token(candidate) {
                    return Some(candidate.to_string());
                }
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    None
}

/// Compare dotted numeric versions. Missing trailing components are treated as 0.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let pa = version_parts(a);
    let pb = version_parts(b);
    let len = pa.len().max(pb.len());
    for i in 0..len {
        let left = pa.get(i).copied().unwrap_or(0);
        let right = pb.get(i).copied().unwrap_or(0);
        match left.cmp(&right) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .split('.')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect()
}

pub fn is_newer_version(candidate: &str, current: &str) -> bool {
    compare_versions(candidate, current) == Ordering::Greater
}

/// Default directories to scan for installers (dev + installed layouts).
pub fn default_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();

    let mut push = |path: PathBuf| {
        if let Ok(canonical) = path.canonicalize() {
            if seen.insert(canonical.clone()) {
                dirs.push(canonical);
            }
        } else if path.is_dir() && seen.insert(path.clone()) {
            dirs.push(path);
        }
    };

    if let Ok(cwd) = std::env::current_dir() {
        push_release_ancestors(&mut push, cwd, 6);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            push_release_ancestors(&mut push, exe_dir.to_path_buf(), 6);
        }
    }

    // Known drop zones on this machine (source tree + product install).
    for extra in [
        r"D:\Grisia Studio\Manga Library\release",
        r"D:\Grisia Studio\Manga Library\update",
        r"D:\Grisia Product\Manga Library",
        r"D:\Grisia Product\Manga Library\release",
        r"D:\Grisia Product\Manga Library\update",
    ] {
        push(PathBuf::from(extra));
    }

    dirs
}

fn push_release_ancestors(
    push: &mut impl FnMut(PathBuf),
    start: PathBuf,
    max_up: usize,
) {
    let mut current = Some(start);
    for _ in 0..=max_up {
        let Some(dir) = current else {
            break;
        };
        push(dir.join("release"));
        push(dir.join("update"));
        push(dir.clone());
        current = dir.parent().map(Path::to_path_buf);
    }
}

/// Scan directories for Manga Library installers and report packages newer than `current_version`.
pub fn check_local_installer_updates(
    current_version: &str,
    search_dirs: &[PathBuf],
) -> LocalUpdateCheckResult {
    let mut packages = Vec::new();
    let mut seen_paths = HashSet::new();
    let mut searched = Vec::new();

    for dir in search_dirs {
        let dir_display = dir.display().to_string();
        if !dir.is_dir() {
            continue;
        }
        searched.push(dir_display);

        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };
            if !is_manga_library_installer(&file_name) {
                continue;
            }
            let Some(version) = extract_version_from_filename(&file_name) else {
                continue;
            };
            let path_string = path.display().to_string();
            if !seen_paths.insert(path_string.clone()) {
                continue;
            }
            packages.push(LocalInstallerPackage {
                path: path_string,
                file_name,
                is_newer: is_newer_version(&version, current_version),
                version,
            });
        }
    }

    packages.sort_by(|a, b| compare_versions(&b.version, &a.version));

    let has_update = packages.iter().any(|pkg| pkg.is_newer);
    let latest = if has_update {
        packages.iter().find(|pkg| pkg.is_newer).cloned()
    } else {
        None
    };

    LocalUpdateCheckResult {
        current_version: current_version.to_string(),
        has_update,
        latest,
        packages,
        searched_dirs: searched,
    }
}

/// Open a file or directory with the OS shell (Explorer on Windows).
pub fn open_path(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    if !path.exists() {
        return Err("路径不存在".to_string());
    }
    launch_path(path)
}

pub fn check_github_updates(current_version: &str) -> Result<LocalUpdateCheckResult, String> {
    let response = github_agent()
        .get(GITHUB_API)
        .call()
        .map_err(map_ureq_error)?;
    let status = response.status();
    if status == 404 {
        return Ok(empty_update_result(current_version));
    }
    if !(200..300).contains(&status) {
        return Err(format!("GitHub 返回 HTTP {status}"));
    }
    let body = response
        .into_string()
        .map_err(|err| format!("无法读取 GitHub 响应: {err}"))?;
    parse_github_release(&body, current_version)
}

pub fn parse_github_release(
    body: &str,
    current_version: &str,
) -> Result<LocalUpdateCheckResult, String> {
    let payload: GitHubRelease = serde_json::from_str(body)
        .map_err(|err| format!("无法解析 GitHub 发布信息: {err}"))?;
    let version = payload
        .tag_name
        .trim()
        .trim_start_matches('v')
        .trim()
        .to_string();
    let Some(asset) = payload
        .assets
        .into_iter()
        .find(|item| is_manga_library_installer(&item.name))
    else {
        return Ok(empty_update_result(current_version));
    };
    let asset_version =
        extract_version_from_filename(&asset.name).unwrap_or_else(|| version.clone());
    let is_newer = is_newer_version(&asset_version, current_version);
    let latest = LocalInstallerPackage {
        path: asset.browser_download_url,
        file_name: asset.name,
        version: asset_version,
        is_newer,
    };
    Ok(LocalUpdateCheckResult {
        current_version: current_version.to_string(),
        has_update: is_newer,
        latest: is_newer.then(|| latest.clone()),
        packages: vec![latest],
        searched_dirs: vec![format!(
            "https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases"
        )],
    })
}

pub fn download_and_install_update(download_url: &str, file_name: &str) -> Result<(), String> {
    if !is_manga_library_installer(file_name) {
        return Err("不是 MangaShelf 安装包".to_string());
    }
    let dest = update_download_path(file_name)?;
    if dest.is_file() {
        let _ = fs::remove_file(&dest);
    }
    let response = github_agent()
        .get(download_url)
        .call()
        .map_err(map_ureq_error)?;
    if !(200..300).contains(&response.status()) {
        return Err(format!("下载失败：HTTP {}", response.status()));
    }
    let mut reader = response.into_reader();
    let mut output = fs::File::create(&dest).map_err(|err| format!("无法保存安装包: {err}"))?;
    std::io::copy(&mut reader, &mut output).map_err(|err| format!("写入安装包失败: {err}"))?;
    output.flush().ok();
    drop(output);
    open_local_installer(&dest)
}

fn empty_update_result(current_version: &str) -> LocalUpdateCheckResult {
    LocalUpdateCheckResult {
        current_version: current_version.to_string(),
        has_update: false,
        latest: None,
        packages: Vec::new(),
        searched_dirs: vec![format!(
            "https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases"
        )],
    }
}

fn github_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(45))
        .user_agent(USER_AGENT)
        .build()
}

fn map_ureq_error(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(404, _) => "GitHub 上还没有发布版本".to_string(),
        ureq::Error::Status(code, _) => format!("GitHub 返回 HTTP {code}"),
        other => format!("无法连接 GitHub: {other}"),
    }
}

fn update_download_path(file_name: &str) -> Result<PathBuf, String> {
    let safe_name = Path::new(file_name)
        .file_name()
        .ok_or_else(|| "无效的安装包文件名".to_string())?;
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("MangaShelf").join("updates");
    fs::create_dir_all(&dir).map_err(|err| format!("无法创建更新目录: {err}"))?;
    Ok(dir.join(safe_name))
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Launch a local installer executable. Only allows `*.exe` setup packages with a parseable version.
pub fn open_local_installer(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    if !path.is_file() {
        return Err("安装包不存在".to_string());
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "无效的安装包路径".to_string())?;
    if !is_manga_library_installer(file_name) {
        return Err("不是 MangaShelf 安装包".to_string());
    }
    if extract_version_from_filename(file_name).is_none() {
        return Err("无法从文件名解析版本号".to_string());
    }

    launch_path(path)
}

fn launch_path(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        launch_with_shell(path)
    }
    #[cfg(not(windows))]
    {
        #[cfg(target_os = "macos")]
        let mut command = Command::new("open");
        #[cfg(not(target_os = "macos"))]
        let mut command = Command::new("xdg-open");
        command
            .arg(path)
            .spawn()
            .map_err(|err| format!("无法打开路径: {err}"))?;
        Ok(())
    }
}

/// Use ShellExecute so Windows can show the UAC prompt.
/// `CreateProcess` (`Command::spawn`) fails with OS error 740 on elevated installers.
#[cfg(windows)]
fn launch_with_shell(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn to_wide(value: &std::ffi::OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    let file = to_wide(path.as_os_str());
    let operation = to_wide(std::ffi::OsStr::new("open"));
    let code = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;

    if code <= 32 {
        return Err(shell_execute_error(code));
    }
    Ok(())
}

fn shell_execute_error(code: isize) -> String {
    match code {
        0 | 8 => "无法启动安装包：内存不足".to_string(),
        2 => "无法启动安装包：找不到文件".to_string(),
        5 => "无法启动安装包：需要管理员权限。请在 UAC 提示中选择“是”，或右键安装包选择“以管理员身份运行”。".to_string(),
        31 => "无法启动安装包：系统无法打开该文件".to_string(),
        _ => format!("无法启动安装包（错误码 {code}）"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn extracts_version_from_tauri_nsis_name() {
        assert_eq!(
            extract_version_from_filename("Manga Library_1.1.0_x64-setup.exe"),
            Some("1.1.0".to_string())
        );
        assert_eq!(
            extract_version_from_filename("Manga Library_1.2.3_x64-setup.exe"),
            Some("1.2.3".to_string())
        );
        assert_eq!(
            extract_version_from_filename("MangaShelf_2.5.0_x64-setup.exe"),
            Some("2.5.0".to_string())
        );
    }

    #[test]
    fn rejects_legacy_companion_installer_name() {
        assert!(!is_manga_library_installer("婕敾鎺т即渚1.0.17_x64-setup.exe"));
    }

    #[test]
    fn accepts_manga_library_installer_names() {
        assert!(is_manga_library_installer("Manga Library_1.1.0_x64-setup.exe"));
        assert!(is_manga_library_installer("manga-library_2.0.0_x64-setup.exe"));
        assert!(is_manga_library_installer("MangaShelf_2.5.0_x64-setup.exe"));
    }

    #[test]
    fn parses_github_release_payload() {
        let json = r#"{
            "tag_name": "v2.5.0",
            "html_url": "https://github.com/qingyou0420/MangaShelf/releases/tag/v2.5.0",
            "assets": [
                {
                    "name": "MangaShelf_2.5.0_x64-setup.exe",
                    "browser_download_url": "https://example.invalid/MangaShelf_2.5.0_x64-setup.exe"
                }
            ]
        }"#;
        let result = parse_github_release(json, "2.4.2").expect("parse");
        assert!(result.has_update);
        let latest = result.latest.expect("latest");
        assert_eq!(latest.version, "2.5.0");
        assert!(latest.is_newer);
        assert!(latest.path.contains("MangaShelf_2.5.0"));
    }

    #[test]
    fn compares_versions_numerically() {
        assert_eq!(compare_versions("1.1.0", "1.0.17"), Ordering::Greater);
        assert_eq!(compare_versions("1.1.0", "1.1.0"), Ordering::Equal);
        assert_eq!(compare_versions("1.1.0", "1.2.0"), Ordering::Less);
        assert_eq!(compare_versions("1.10.0", "1.9.0"), Ordering::Greater);
        assert!(is_newer_version("1.2.0", "1.1.0"));
        assert!(!is_newer_version("1.1.0", "1.1.0"));
        assert!(!is_newer_version("1.0.9", "1.1.0"));
    }

    #[test]
    fn finds_newer_installer_in_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let release = temp.path().join("release");
        fs::create_dir_all(&release).expect("mkdir");

        fs::write(
            release.join("Manga Library_1.0.0_x64-setup.exe"),
            b"old",
        )
        .expect("write old");
        fs::write(
            release.join("Manga Library_1.2.0_x64-setup.exe"),
            b"new",
        )
        .expect("write new");
        fs::write(
            release.join("婕敾鎺т即渚9.9.9_x64-setup.exe"),
            b"legacy",
        )
        .expect("write legacy");
        fs::write(release.join("readme.txt"), b"ignore").expect("write txt");

        let result = check_local_installer_updates("1.1.0", std::slice::from_ref(&release));
        assert!(result.has_update);
        assert_eq!(result.current_version, "1.1.0");
        assert_eq!(result.packages.len(), 2);
        let latest = result.latest.expect("latest");
        assert_eq!(latest.version, "1.2.0");
        assert!(latest.is_newer);
        assert!(latest.file_name.contains("1.2.0"));
        assert!(result
            .packages
            .iter()
            .any(|p| p.version == "1.0.0" && !p.is_newer));
    }

    #[test]
    fn no_update_when_only_same_or_older() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("Manga Library_1.1.0_x64-setup.exe"),
            b"same",
        )
        .expect("write");
        fs::write(
            temp.path().join("Manga Library_1.0.5_x64-setup.exe"),
            b"older",
        )
        .expect("write older");

        let result = check_local_installer_updates("1.1.0", &[temp.path().to_path_buf()]);
        assert!(!result.has_update);
        assert!(result.latest.is_none());
        assert_eq!(result.packages.len(), 2);
    }

    #[test]
    fn ancestor_walk_finds_release_folder_above_exe_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let release = temp.path().join("release");
        let nested = temp.path().join("src-tauri").join("target").join("release");
        fs::create_dir_all(&release).expect("release");
        fs::create_dir_all(&nested).expect("nested");
        fs::write(
            release.join("Manga Library_2.0.0_x64-setup.exe"),
            b"pkg",
        )
        .expect("write");

        let mut dirs = Vec::new();
        let mut seen = HashSet::new();
        let mut push = |path: PathBuf| {
            if path.is_dir() && seen.insert(path.clone()) {
                dirs.push(path);
            }
        };
        push_release_ancestors(&mut push, nested, 6);

        let result = check_local_installer_updates("1.1.2", &dirs);
        assert!(result.has_update);
        assert_eq!(result.latest.expect("latest").version, "2.0.0");
    }

    #[test]
    fn shell_execute_error_explains_access_denied() {
        assert!(shell_execute_error(5).contains("管理员权限"));
        assert!(shell_execute_error(2).contains("找不到文件"));
    }

    #[test]
    fn open_rejects_non_installer() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("notes.txt");
        fs::write(&path, b"hi").expect("write");
        assert!(open_local_installer(&path).is_err());
    }
}
