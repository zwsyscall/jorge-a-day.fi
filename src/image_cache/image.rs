use actix_web::mime;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use exif::{Reader, Tag};
use std::{path::PathBuf, str::FromStr};

const COMPRESSION_LEVEL: f32 = 0.82;

#[derive(Clone, Debug)]
pub struct Image {
    pub path: PathBuf,
    pub data: Vec<u8>,
    pub compressed_data: Vec<u8>,
    pub image_age: DateTime<Utc>,
    cache_time: DateTime<Utc>,
    image_type: imghdr::Type,
}

impl Image {
    fn apply_exif_orientation(img: image::DynamicImage, orientation: u32) -> image::DynamicImage {
        use image::imageops::{flip_horizontal, flip_vertical, rotate90, rotate180, rotate270};

        match orientation {
            1 => img,
            2 => image::DynamicImage::ImageRgba8(flip_horizontal(&img)),
            3 => image::DynamicImage::ImageRgba8(rotate180(&img)),
            4 => image::DynamicImage::ImageRgba8(flip_vertical(&img)),
            5 => image::DynamicImage::ImageRgba8(rotate90(&flip_horizontal(&img))),
            6 => image::DynamicImage::ImageRgba8(rotate90(&img)),
            7 => image::DynamicImage::ImageRgba8(rotate270(&flip_horizontal(&img))),
            8 => image::DynamicImage::ImageRgba8(rotate270(&img)),
            _ => img,
        }
    }

    /// Extract EXIF orientation if present
    fn get_exif_orientation(data: &[u8]) -> Result<u32, anyhow::Error> {
        let exif = Reader::new()
            .read_from_container(&mut std::io::Cursor::new(data))
            .map_err(|e| anyhow!("EXIF parse error: {}", e))?;

        let field = exif
            .get_field(Tag::Orientation, exif::In::PRIMARY)
            .ok_or_else(|| anyhow!("No Orientation tag found"))?;

        field
            .value
            .get_uint(0)
            .ok_or_else(|| anyhow!("Invalid orientation value"))
    }

    fn compress_image(data: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        let img = image::load_from_memory(data)?;

        let orientation = Self::get_exif_orientation(data).unwrap_or(1);
        let rotated = Self::apply_exif_orientation(img, orientation);

        let encoder = webp::Encoder::from_image(&rotated)
            .map_err(|err| anyhow!("Error parsing file: {}", err))?;

        encoder
            .encode_simple(false, COMPRESSION_LEVEL)
            .map_err(|err| anyhow!("Error encoding data: {:#?}", err))
            .map(|mem| mem.to_vec())
    }

    fn resolve_compressed(data: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        Self::compress_image(&data)
    }
}

impl Image {
    /// Returns cache age in ms
    pub fn cache_age(&self) -> i64 {
        let now = Utc::now();
        (now - self.cache_time).num_milliseconds()
    }

    /// Loads image data to the cache and update cache age
    pub fn resolve(&mut self) -> Result<(), anyhow::Error> {
        if let Ok(data) = std::fs::read(&self.path) {
            self.compressed_data = Self::resolve_compressed(&data)?;
            self.data = data;
            self.cache_time = Utc::now();
            return Ok(());
        }
        Err(anyhow!("Unable to read image data"))
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn compressed_is_empty(&self) -> bool {
        self.compressed_data.is_empty()
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

        Ok(Self {
            image_type: imghdr::from_file(&path)?.ok_or(anyhow!("File type is not supported"))?,
            image_age: DateTime::from(path.metadata()?.created()?),
            path: path,
            cache_time: Utc::now(),
            data: Vec::new(),
            compressed_data: Vec::new(),
        })
    }
}
