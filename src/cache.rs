use crate::image::Image;
use crate::{api_schema, config::AppConfig};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;
use walkdir::WalkDir;

pub struct ImageCache {
    directories: Vec<PathBuf>,
    cache: HashMap<String, Image>,
    max_cache_age_ms: i64,
    newest_image: Option<String>,
    newest_image_time: DateTime<Utc>,
}

// todo: add custom error type
impl ImageCache {
    /// Fills up the cache without resolving data.
    pub async fn init(&mut self, config: &AppConfig) {
        let directories = &config.directories;
        self.directories = config
            .directories
            .iter()
            .map(|d| PathBuf::from(d).canonicalize().unwrap())
            .collect();
        debug!("{:?}", self.directories);

        let mut files = Vec::new();
        for dir in directories {
            files.extend_from_slice(
                &WalkDir::new(dir)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|e| e.file_type().is_file())
                    .map(|e| e.into_path())
                    // This is possibly slow. Might have to change this later on
                    .filter(|e| imghdr::from_file(e).ok().is_some())
                    .collect::<Vec<PathBuf>>(),
            )
        }

        for file in &files {
            if let Err(e) = &self.insert_image(file).await {
                error!("Error inserting image to cache: {}", e);
            }
        }
    }

    pub async fn insert_image(&mut self, img: &PathBuf) -> Result<String, anyhow::Error> {
        let image_path = img.canonicalize()?;
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

    pub async fn fetch_image(&mut self, key: &str) -> Result<Image, anyhow::Error> {
        if let Some(cached_image) = self.cache.get_mut(key) {
            debug!("Image present in cache, age {}", cached_image.cache_age());

            // if it's not old enough AND the data is actually present
            if cached_image.cache_age() < self.max_cache_age_ms && !cached_image.is_empty() {
                debug!(
                    "Cache has not expired and data is present, age: {}, size: {} bytes‚",
                    &cached_image.cache_age(),
                    cached_image.data.len()
                );
                return Ok(cached_image.clone());
            }

            debug!("Fetching image from disk");
            cached_image.resolve()?;
            return Ok(cached_image.clone());
        }

        Err(anyhow!("no image found"))
    }

    pub async fn get_images(&self) -> Vec<api_schema::Image> {
        self.cache
            .iter()
            .map(|(key, img)| api_schema::Image::from((key, img.to_owned())))
            .collect()
    }

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
            return self.fetch_image(&id).await.ok();
        }

        None
    }

    pub fn clean_cache(&mut self) {
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

        debug!("Cleaned {} images from cache.", cleared_images)
    }
}

impl From<i64> for ImageCache {
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

pub async fn cache_cleanup(cache: Arc<Mutex<ImageCache>>) {
    use tokio::time::{Duration, sleep};
    debug!("Beginning cache clean up thread");
    loop {
        sleep(Duration::from_secs(60)).await;
        {
            let mut cache_lock = cache.lock().await;
            cache_lock.clean_cache();
        }
    }
}

pub async fn directory_watcher(cache: Arc<Mutex<ImageCache>>) {
    debug!("Starting directory watcher thread");
    let directories = {
        let cache_lock = cache.lock().await;
        cache_lock.directories.clone()
    };

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let mut watcher = match RecommendedWatcher::new(
        move |res| {
            if let Err(e) = tx.blocking_send(res) {
                error!("Failed to send watch event to async channel: {:?}", e);
            }
        },
        Config::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            error!("Error creating watcher: {:?}", e);
            return;
        }
    };

    for directory in &directories {
        debug!("Starting to watch {:?}", directory);
        watcher
            .watch(directory, RecursiveMode::Recursive)
            .unwrap_or_else(|err| error!("Error watching directory {:#?} => {}", &directory, err));
    }

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => match event.kind {
                EventKind::Create(_)
                | EventKind::Modify(notify::event::ModifyKind::Name(
                    notify::event::RenameMode::To,
                )) => {
                    debug!("Modified name!");
                    let mut cache_lock = cache.lock().await;
                    for path in event.paths {
                        #[allow(unused)]
                        cache_lock.insert_image(&path).await;
                    }
                }
                _ => {}
            },
            Err(error) => error!("Watcher error: {:?}", error),
        }
    }

    debug!("Directory watcher thread stopped.");
}
