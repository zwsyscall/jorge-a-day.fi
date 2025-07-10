use actix_web::mime;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use std::{path::PathBuf, str::FromStr};

#[derive(Clone)]
pub struct Image {
    pub path: PathBuf,
    pub data: Vec<u8>,
    pub cache_time: DateTime<Utc>,
    pub image_type: imghdr::Type,
    pub image_age: DateTime<Utc>,
}

impl Image {
    /// Returns cache age in seconds
    pub fn cache_age(&self) -> i64 {
        let now = Utc::now();
        (now - self.cache_time).num_milliseconds()
    }

    /// Loads image data to the cache and update cache age
    pub fn resolve(&mut self) -> Result<(), anyhow::Error> {
        self.data = std::fs::read(&self.path)?;
        self.cache_time = Utc::now();
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn content_type(&self) -> String {
        match self.image_type {
            imghdr::Type::Gif => mime::IMAGE_GIF.to_string(),
            imghdr::Type::Tiff => "image/tiff".to_string(),
            imghdr::Type::Jpeg => mime::IMAGE_JPEG.to_string(),
            imghdr::Type::Bmp => mime::IMAGE_BMP.to_string(),
            imghdr::Type::Png => mime::IMAGE_PNG.to_string(),
            imghdr::Type::Webp => "image/webp".to_string(),
            imghdr::Type::Exr => "image/exr".to_string(),
            imghdr::Type::Ico => "image/vnd.microsoft.icon".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }
}

impl FromStr for Image {
    type Err = anyhow::Error;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(path).canonicalize()?;

        Ok(Image {
            image_type: imghdr::from_file(&path)?.ok_or(anyhow!("File type is not supported"))?,
            image_age: DateTime::from(path.metadata()?.created()?),
            path: path,
            cache_time: Utc::now(),
            data: Vec::new(),
        })
    }
}
