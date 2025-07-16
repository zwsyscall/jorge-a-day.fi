use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Return code for GET /daily
#[derive(Deserialize, Serialize)]
pub struct DailyImage {
    pub image: ImageJson,
}

#[derive(Deserialize, Serialize)]
pub struct Images {
    images: Vec<ImageJson>,
}

#[derive(Deserialize, Serialize)]
pub struct ImageJson {
    pub date: DateTime<Utc>,
    pub url: String,
}

impl From<(String, crate::image_cache::image::Image)> for ImageJson {
    fn from((key, img): (String, crate::image_cache::image::Image)) -> Self {
        Self {
            url: key,
            date: img.image_age,
        }
    }
}

#[derive(Deserialize)]
pub struct CompressQuery {
    pub compress: Option<String>,
}
