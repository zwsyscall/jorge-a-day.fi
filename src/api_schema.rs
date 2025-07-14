use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Return code for GET /daily
#[derive(Deserialize, Serialize)]
pub struct DailyImage {
    pub image: Image,
}

#[derive(Deserialize, Serialize)]
pub struct Images {
    images: Vec<Image>,
}

#[derive(Deserialize, Serialize)]
pub struct Image {
    pub date: DateTime<Utc>,
    pub url: String,
}

impl From<(String, crate::image_cache::image::Image)> for Image {
    fn from((key, img): (String, crate::image_cache::image::Image)) -> Self {
        Self {
            url: key,
            date: img.image_age,
        }
    }
}
