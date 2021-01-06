use tokio::fs::{File};
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::collections::HashSet;
use tokio::sync::Mutex;
use std::ops::Deref;
use crate::server_com;
use std::iter::FromIterator;
use std::error::Error;
use std::sync::Arc;
use crate::web_interface::model::ImageAppStatus;
use crate::web_interface::model::ws::{MyWs, Notification};
use actix::Addr;
use crate::server_com::com_model;
use actix_web::rt::time::delay_for;
use log::{info, error};
use crate::photogrammetry::paths;

pub struct ImageStore {
    image_list: Mutex<HashSet<String>>,
}

impl ImageStore {
    pub async fn new() -> tokio::io::Result<ImageStore> {
        init_dir().await?;
        Ok(ImageStore { image_list: Mutex::new(HashSet::new()) })
    }

    pub async fn store_image(&self, image_path: &str, image: &Vec<u8>) -> Result<(), tokio::io::Error> {
        let mut image_list = self.image_list.lock().await;
        let image_name = image_path.split("/").last().unwrap();
        save_image(image_name, &image).await?;
        image_list.insert(image_name.to_string());
        Ok(())
    }

    pub async fn get_image_list(&self) -> Vec<String> {
        let image_list = self.image_list.lock().await;
        Vec::from_iter(image_list.deref().clone().into_iter())
    }

    pub async fn get_image(&self, name: &String) -> Result<Vec<u8>, Option<tokio::io::Error>> {
        let image_list = self.image_list.lock().await;
        if image_list.contains(name) {
            Ok(read_image(name).await.map_err(|err| Some(err))?)
        } else {
            Err(None)
        }
    }
}

async fn save_image(name: &str, img: &Vec<u8>) -> Result<(), tokio::io::Error> {
    let mut file = File::create(paths::image_folder().join(name)).await?;
    file.write_all(img).await?;
    Ok(())
}

async fn read_image(name: &str) -> tokio::io::Result<Vec<u8>> {
    let mut file = File::open(paths::image_folder().join(name)).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;
    Ok(buf)
}

async fn init_dir() -> tokio::io::Result<()> {
    if paths::parent_folder().exists() {
        tokio::fs::remove_dir_all(paths::parent_folder()).await?;
    }
    tokio::fs::create_dir_all(paths::image_folder()).await?;
    Ok(())
}

pub const POLL_DELAY: u64 = 3; // in seconds

pub struct ImageDownloader {
    url: String,
    target_server_status: com_model::ServerStatus,
    notification_handle: Arc<std::sync::Mutex<Option<Addr<MyWs>>>>,
    image_store: Arc<ImageStore>,
    app_image_status: Arc<Mutex<ImageAppStatus>>,
    reset: Mutex<bool>,
}

impl ImageDownloader {
    pub async fn new(url: String,
                     target_server_status: com_model::ServerStatus,
                     notification_handle: Arc<std::sync::Mutex<Option<Addr<MyWs>>>>) -> Result<ImageDownloader, Box<dyn Error + Send>> {
        Ok(ImageDownloader {
            url,
            target_server_status,
            notification_handle,
            image_store: Arc::new(
                ImageStore::new()
                    .await
                    .map_err(|err| -> Box<dyn Error + Send> { Box::new(err)})?),
            app_image_status: Arc::new(Mutex::new(ImageAppStatus::Start)),
            reset: Mutex::new(false),
        })
    }

    pub async fn get_status(&self) -> ImageAppStatus {
        self.app_image_status.lock().await.deref().clone()
    }

    pub async fn start(self: Arc<Self>) {
        let app_image_status = Arc::clone(&self.app_image_status);
        tokio::spawn(async move {
            while app_image_status.lock().await.deref().ne(&ImageAppStatus::Finished)
                || *self.reset.lock().await {
                if let Some(new_status) = self.get_new_status().await.expect("failed to get status") {
                    *self.app_image_status.lock().await = new_status;
                    self.download_images().await.expect("failed to download images");
                    self.notifie_ws().await;
                }
                delay_for(tokio::time::Duration::from_secs(POLL_DELAY)).await;
            };
        });
    }

    async fn get_new_image_paths(&self) -> Result<Vec<String>, Box<dyn Error + Send>> {
        let available_images = server_com::get_ready_image_list(&self.url).await
            .map_err(|err| -> Box<dyn Error + Send> { Box::new(err) })?;
        let old_images = self.image_store.image_list.lock().await;
        Ok(available_images
            .difference(&HashSet::from_iter(old_images.iter().map(|image_name| format!("/aufnahme/{}", image_name))))
            .map(|image_name| image_name.clone())
            .collect::<Vec<_>>())
    }

    async fn download_images(&self) -> Result<(), Box<dyn Error + Send>> {
        let new_images = self.get_new_image_paths().await?;
        for image_path in new_images {
            let url = self.url.clone();
            let image_store = Arc::clone(&self.image_store);
            tokio::spawn(async move {
                //downlaod aufnahme from server
                let image = match server_com::get_aufnahme(&url, &image_path).await {
                    Ok(image) => image,
                    Err(err) => {
                        error!("Error downloading image: {}", err);
                        return;
                    },
                };


                // save aufname locally
                if let Err(err) = image_store.store_image(&image_path, &image).await {
                    error!("Error downloading image: {}", err);
                };
            });
        }
        Ok(())
    }

    async fn get_new_status(&self) -> Result<Option<ImageAppStatus>, Box<dyn Error + Send>> {
        let server_status = server_com::get_status(&self.url).await
            .map_err(|err| -> Box<dyn Error + Send> { Box::new(err) })?;
        let new_server_status = if server_status.eq(&self.target_server_status) {
            ImageAppStatus::Finished
        } else {
            ImageAppStatus::TakingImages(server_status)
        };

        if self.app_image_status.lock().await.eq(&new_server_status) {
            Ok(None)
        } else {
            Ok(Some(new_server_status))
        }
    }

    async fn notifie_ws(&self) {
        let notification_handle = Arc::clone(&self.notification_handle);
        tokio::task::spawn_blocking(move || {
            let notification_handle_option = notification_handle
                .lock()
                .unwrap();
            if let Some(notification_handle) = notification_handle_option.deref() {
                notification_handle.do_send(Notification("new image".to_string()));
            } else {
                info!("websocket not available update message was therefore not send")
            }
        });
    }

    pub(crate) async fn reset(&self) {
        *self.reset.lock().await = true;
    }

    pub async fn get_image_list(&self) -> Vec<String> {
        self.image_store.get_image_list().await
    }

    pub async fn get_image(&self, image_name: &String) -> Result<Vec<u8>, Option<tokio::io::Error>> {
        self.image_store.get_image(image_name).await
    }
}

