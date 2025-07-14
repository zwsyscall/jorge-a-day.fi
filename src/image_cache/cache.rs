use crate::cache::CacheTrait;
use crate::image_cache::image::Image;
use crate::{api_schema, config::AppConfig};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use log::{debug, error, info, trace};
use std::{collections::HashMap, path::PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

pub struct Cache {
    directories: Vec<PathBuf>,
    cache: HashMap<String, Image>,
    max_cache_age_ms: i64,
    newest_image: Option<String>,
    newest_image_time: DateTime<Utc>,
}

// todo: add custom error type
impl CacheTrait for Cache {
    type Error = anyhow::Error;
    type DataSource = PathBuf;
    type Data = Image;
    type Key = String;

    async fn insert_data(&mut self, img: &PathBuf) -> Result<String, anyhow::Error> {
        let image_path = img.canonicalize()?;
        if !image_path.metadata()?.is_file() {
            return Err(anyhow!("Passed a directory."));
        }

        if !self
            .directories
            .iter()
            .any(|dir| image_path.starts_with(dir))
        {
            return Err(anyhow!("Image is outside of scope"));
        }

        let image: Image = {
            let img_str = image_path
                .to_str()
                .ok_or(anyhow!("Image path is not valid."))?;
            img_str.parse()?
        };

        let id = loop {
            let id = Uuid::new_v4().to_string();
            if !self.cache.contains_key(&id) {
                break id;
            }
        };
        debug!("Added to cache: {} => {:#?}", &id, &image_path);

        self.cache.insert(id.to_owned(), image.clone());
        Ok(id)
    }
    async fn remove_data(&mut self, image_path: &PathBuf) -> Option<Image> {
        let image_id = self
            .cache
            .iter()
            .find(|(_, img)| img.path == *image_path)
            .map(|(key, _)| key.to_owned())?;

        self.cache.remove(&image_id)
    }
    async fn get_data(&mut self, key: &String) -> Result<Image, anyhow::Error> {
        if let Some(cached_image) = self.cache.get_mut(key) {
            trace!("Image present in cache, age {}", cached_image.cache_age());

            // if it's not old enough AND the data is actually present
            if cached_image.cache_age() < self.max_cache_age_ms && !cached_image.is_empty() {
                trace!(
                    "Cache has not expired and data is present, age: {}, size: {} bytes‚",
                    &cached_image.cache_age(),
                    cached_image.data.len()
                );
                return Ok(cached_image.clone());
            }

            trace!("Fetching image from disk");
            cached_image.resolve()?;
            return Ok(cached_image.clone());
        }

        Err(anyhow!("no image found"))
    }
    fn clean_cache(&mut self) {
        let mut cleared_images = 0;

        // Cache
        for (_, image) in self.cache.iter_mut() {
            if image.cache_age() > self.max_cache_age_ms && !image.is_empty() {
                image.clear();
                cleared_images += 1;
            }
        }

        // Daily image
        let newest_image_age = (Utc::now() - self.newest_image_time).num_milliseconds();
        if newest_image_age > self.max_cache_age_ms {
            self.newest_image = None
        }

        if cleared_images > 0 {
            debug!("Cleaned {} images from cache.", cleared_images)
        }
    }
    fn directories(&self) -> Vec<PathBuf> {
        self.directories.clone()
    }
}

impl Cache {
    /// Fills up the cache without resolving data.
    pub async fn init(&mut self, config: &AppConfig) {
        let directories = &config.directories;
        self.directories = config
            .directories
            .iter()
            .map(|d| PathBuf::from(d).canonicalize().unwrap())
            .collect();

        let mut files = Vec::new();
        for dir in directories {
            files.extend_from_slice(
                &WalkDir::new(dir)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|e| e.file_type().is_file())
                    .map(|e| e.into_path())
                    .collect::<Vec<PathBuf>>(),
            )
        }

        for file in &files {
            if let Err(e) = &self.insert_data(file).await {
                error!("Error inserting image to cache: {}", e);
            }
        }
    }
    pub async fn get_images(&self, prefix: &str) -> Vec<api_schema::Image> {
        let mut images: Vec<(&str, &Image)> = self
            .cache
            .iter()
            .map(|(key, data)| (key.as_str(), data))
            .collect();

        // Sort and reverse
        images.sort_by_key(|(_key, img)| img.image_age);
        images.reverse();

        // Return transformed images :)
        images
            .into_iter()
            .map(|(key, img)| {
                api_schema::Image::from((format!("{}/{}", prefix, key), img.to_owned()))
            })
            .collect()
    }

    /*.map(|(key, img)| {
        api_schema::Image::from((format!("{}/{}", prefix, key), img.to_owned()))
    })
    .collect() */
    async fn get_newest_image_id(&mut self) -> Option<String> {
        if let Some(id) = &self.newest_image {
            return Some(id.to_owned());
        }
        self.newest_image = self
            .cache
            .iter()
            .max_by_key(|img_tuple| img_tuple.1.image_age)
            .map(|(key, _)| key.to_owned());

        self.newest_image.clone()
    }

    pub async fn get_newest_image(&mut self) -> Option<Image> {
        if let Some(id) = self.get_newest_image_id().await {
            return self.get_data(&id).await.ok();
        }

        None
    }
}
impl From<i64> for Cache {
    fn from(cache_max_age: i64) -> Self {
        info!(
            "Initializing a cache with a maximum TTL of {}ms",
            cache_max_age
        );

        Self {
            directories: Vec::new(),
            cache: HashMap::new(),
            max_cache_age_ms: cache_max_age,
            newest_image: None,
            newest_image_time: Utc::now(),
        }
    }
}
