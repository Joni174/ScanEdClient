use std::path::{PathBuf};
use std::str::FromStr;
use tokio::fs::{File};
use actix_web::web::{Buf, Bytes};
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::collections::HashSet;
use tokio::sync::Mutex;
use std::ops::Deref;
use crate::server_com::{get_ready_image_list, get_aufnahme};
use std::iter::FromIterator;

pub struct ImageStore {
    image_list: Mutex<HashSet<String>>,
}

impl ImageStore {
    pub async fn new() -> tokio::io::Result<ImageStore> {
        init_dir().await?;
        Ok(ImageStore { image_list: Mutex::new(HashSet::new()) })
    }

    pub async fn store_image(&self, name: String, image: &Vec<u8>) -> Result<(), tokio::io::Error> {
        let mut image_list = self.image_list.lock().await;
        save_image(&name, &image).await?;
        image_list.insert(name);
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

    pub async fn reset(&self) -> tokio::io::Result<()> {
        let mut image_list = self.image_list.lock().await;
        image_list.clear();
        init_dir().await
    }

    pub async fn download_new_images(&self, url: &str) -> bool {
        let available_images = match get_ready_image_list(&url).await {
            Ok(image_list) => image_list,
            Err(err) => {
                log::error!("{}", err.to_string());
                return false;
            }
        };

        let old_images = self.image_list.lock().await;
        let new_images = available_images
            .difference(&old_images)
            .map(|image_name| image_name.clone())
            .collect::<Vec<_>>();
        drop(old_images); // release lock to enable status beeing polled while images are downloading
        let mut downloaded = 0;
        for image_name in new_images {
            //downlaod aufnahme from server
            let image = match get_aufnahme(&url, &image_name).await {
                Ok(image) => image,
                Err(err) => {
                    log::error!("{}", err.to_string());
                    return false;
                }
            };

            // save aufname locally
            if let Err(err) = self.store_image(image_name, &image).await {
                log::error!("{}", err.to_string());
                return false;
            }
            downloaded += 1;
        }
        downloaded != 0
    }
}

async fn save_image(name: &str, img: &Vec<u8>) -> Result<(), tokio::io::Error> {
    let mut file = File::open(image_folder().join(name)).await?;
    file.write_all(img).await?;
    Ok(())
}

async fn read_image(name: &str) -> tokio::io::Result<Vec<u8>> {
    let mut file = File::open(image_folder().join(name)).await?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await?;
    Ok(buf)
}

async fn init_dir() -> tokio::io::Result<()> {
    if image_folder().exists() {
        tokio::fs::remove_dir_all(image_folder()).await?;
    }
    tokio::fs::create_dir(image_folder()).await?;
    Ok(())
}

fn image_folder() -> PathBuf {
    PathBuf::from_str("images").unwrap()
}