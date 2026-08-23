use anyhow::Result;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

const COVER_MAX_WIDTH: u32 = 320;

pub fn write_cover_thumbnail(
    bookshelf_root: impl AsRef<Path>,
    comic_id: &str,
    source: impl AsRef<Path>,
) -> Result<PathBuf> {
    let source = source.as_ref();
    let dir = bookshelf_root
        .as_ref()
        .join(".manga-library")
        .join("covers");
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(format!("{}.jpg", cover_stem(comic_id)));

    if dest.is_file() && source.is_file() {
        if let (Ok(src_meta), Ok(dst_meta)) = (std::fs::metadata(source), std::fs::metadata(&dest)) {
            if dst_meta.len() > 0
                && dst_meta.modified().ok() >= src_meta.modified().ok()
            {
                return Ok(dest);
            }
        }
    }

    let image = image::open(source)?;
    let resized = image.resize(
        COVER_MAX_WIDTH,
        COVER_MAX_WIDTH.saturating_mul(4),
        image::imageops::FilterType::Triangle,
    );
    let rgb = image::DynamicImage::ImageRgb8(resized.to_rgb8());
    let tmp = dest.with_extension("tmp");
    rgb.save_with_format(&tmp, image::ImageFormat::Jpeg)?;
    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}

pub fn cover_or_source(
    bookshelf_root: &Path,
    comic_id: &str,
    source: PathBuf,
) -> PathBuf {
    write_cover_thumbnail(bookshelf_root, comic_id, &source).unwrap_or(source)
}

fn cover_stem(comic_id: &str) -> String {
    let mut hasher = DefaultHasher::new();
    comic_id.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbImage;

    #[test]
    fn writes_jpeg_thumbnail_and_reuses_unchanged_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("page.png");
        RgbImage::from_pixel(800, 1200, image::Rgb([30, 50, 80]))
            .save(&source)
            .expect("png");

        let first = write_cover_thumbnail(temp.path(), "local:test", &source).expect("thumb");
        assert_eq!(first.extension().and_then(|ext| ext.to_str()), Some("jpg"));
        let thumb = image::open(&first).expect("open thumb");
        assert!(thumb.width() <= COVER_MAX_WIDTH);
        assert!(thumb.height() > 0);

        let first_len = std::fs::metadata(&first).expect("meta").len();
        let second = write_cover_thumbnail(temp.path(), "local:test", &source).expect("reuse");
        assert_eq!(first, second);
        assert_eq!(std::fs::metadata(&second).expect("meta").len(), first_len);
    }

    #[test]
    fn falls_back_when_source_is_not_an_image() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("page.jpg");
        std::fs::write(&source, b"not-an-image").expect("write");
        let result = cover_or_source(temp.path(), "local:bad", source.clone());
        assert_eq!(result, source);
    }
}
