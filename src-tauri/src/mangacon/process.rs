use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MangaConProcessError {
    #[error("路径必须指向 MangaCon.exe: {0}")]
    InvalidExecutableName(String),
    #[error("MangaCon.exe 不存在: {0}")]
    MissingExecutable(String),
    #[error("路径不是文件: {0}")]
    NotAFile(String),
    #[error("启动 MangaCon.exe 失败: {0}")]
    LaunchFailed(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub pid: u32,
}

pub fn validate_exe(path: impl AsRef<Path>) -> Result<PathBuf, MangaConProcessError> {
    let path = path.as_ref();
    if path.file_name() != Some(OsStr::new("MangaCon.exe")) {
        return Err(MangaConProcessError::InvalidExecutableName(
            path.display().to_string(),
        ));
    }

    if !path.exists() {
        return Err(MangaConProcessError::MissingExecutable(
            path.display().to_string(),
        ));
    }

    if !path.is_file() {
        return Err(MangaConProcessError::NotAFile(path.display().to_string()));
    }

    Ok(path.to_path_buf())
}

pub fn launch_mangacon(path: impl AsRef<Path>) -> Result<LaunchResult, MangaConProcessError> {
    let exe_path = validate_exe(path)?;
    let child = Command::new(exe_path).spawn()?;

    Ok(LaunchResult { pid: child.id() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_exe_reports_missing_mangacon_exe() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("MangaCon.exe");

        let error = validate_exe(&path).expect_err("missing exe should fail");

        assert!(error.to_string().contains("MangaCon.exe 不存在"), "{error}");
    }
}
