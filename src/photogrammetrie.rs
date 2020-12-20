use std::path::{PathBuf};
use std::str::FromStr;
use tokio::fs::{File};
use actix_web::web::Buf;
use tokio::io::AsyncWriteExt;


pub fn clear_images() {
    if image_folder().is_dir() {
        std::fs::remove_dir_all(image_folder()).unwrap()
    }
}

pub async fn save_image(name: &str, img: &actix_web::web::Bytes) -> Result<(), std::io::Error> {
    let mut file = File::open(image_folder().join(name)).await.unwrap();
    file.write_all(img.bytes()).await.unwrap();
    Ok(())
}

fn image_folder() -> PathBuf {
    PathBuf::from_str("abc").unwrap()
}