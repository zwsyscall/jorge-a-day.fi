use log::{debug, error, info};
use notify::{
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{ModifyKind, RenameMode},
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

pub trait CacheTrait {
    type Error;
    type DataSource;
    type Data;
    type Key;

    async fn insert_data(&mut self, data: &Self::DataSource) -> Result<Self::Key, Self::Error>;
    async fn remove_data(&mut self, data: &Self::DataSource) -> Option<Self::Data>;
    async fn get_data(&mut self, key: &Self::Key) -> Result<Self::Data, Self::Error>;
    async fn get_data_bytes(
        &mut self,
        key: &Self::Key,
        compress: bool,
    ) -> Result<(String, Vec<u8>), Self::Error>;
    fn len(&self) -> usize;
    fn directories(&self) -> Vec<PathBuf>;
    fn clean_cache(&mut self);
}

// Background processes
pub async fn cache_cleanup<C: CacheTrait>(cache: Arc<Mutex<C>>) {
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

pub async fn directory_watcher<C>(cache: Arc<Mutex<C>>)
where
    C: CacheTrait<DataSource = PathBuf>,
{
    debug!("Starting directory watcher thread");
    let directories = {
        let cache_lock = cache.lock().await;
        cache_lock.directories()
    };

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let mut watcher = match RecommendedWatcher::new(
        move |res| {
            if let Err(e) = tx.blocking_send(res) {
                error!("Failed to send watch event to async channel: {:?}", e);
            }
        },
        notify::Config::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            error!("Error creating watcher: {:?}", e);
            return;
        }
    };

    for directory in &directories {
        info!("Starting to watch {:?}", directory);
        watcher
            .watch(directory, RecursiveMode::Recursive)
            .unwrap_or_else(|err| error!("Error watching directory {:#?} => {}", &directory, err));
    }

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => match event.kind {
                EventKind::Create(_) | EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                    let mut cache_lock = cache.lock().await;
                    for path in event.paths {
                        #[allow(unused)]
                        cache_lock.insert_data(&path).await;
                    }
                }
                EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                    let mut cache_lock = cache.lock().await;
                    for path in event.paths {
                        cache_lock.remove_data(&path).await;
                    }
                }
                _ => {}
            },
            Err(error) => error!("Watcher error: {:?}", error),
        }
    }

    debug!("Directory watcher thread stopped.");
}
