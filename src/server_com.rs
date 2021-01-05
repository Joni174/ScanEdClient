use reqwest::{Response, Url};
use std::str::FromStr;
use crate::server_com::com_model::{ServerStatus, Auftrag};
use std::collections::HashSet;
use std::error::Error;
use log::{info};

const AUFTRAG_ENPOINT: &'static str = "auftrag";
const AUFNAHMEN_ENDPOINT: &'static str = "aufnahme";

pub mod com_model {
    use serde::{Serialize, Deserialize};

    #[derive(Deserialize, Serialize, Clone, PartialEq)]
    pub struct ServerStatus {
        runde: i32,
        aufnahme: i32,
    }

    #[derive(Deserialize, Serialize, Debug)]
    pub struct Auftrag {
        pub auftrag: Vec<i32>
    }

    impl Auftrag {
        pub fn from_vec(rounds: Vec<i32>) -> Auftrag {
            Auftrag { auftrag: rounds }
        }
        pub fn into_target_status(self) -> ServerStatus {
            ServerStatus {
                runde: self.auftrag.len() as i32,
                aufnahme: *self.auftrag.last().unwrap()
            }
        }
    }
}

pub async fn get_status(url: &str) -> reqwest::Result<ServerStatus> {
    reqwest::get(str_to_url(&url).join(AUFTRAG_ENPOINT).unwrap()).await?
        .json::<ServerStatus>().await
}

fn str_to_url(str: &str) -> Url {
    reqwest::Url::from_str(&str).unwrap()
}

pub async fn post_auftrag(auftrag: Auftrag, url: &str) -> Result<Response, Box<dyn Error + Send>> {
    info!("post auftrag: {:?}", auftrag);
    let client = reqwest::Client::new();
    client.post(str_to_url(url)
        .join(AUFTRAG_ENPOINT).map_err(|err| -> Box<dyn Error + Send> { Box::new(err) })?)
        .json(&auftrag)
        .send()
        .await
        .map_err(|err| -> Box<dyn Error + Send> { Box::new(err) })
}

pub(crate) async fn get_ready_image_list(url: &str) -> reqwest::Result<HashSet<String>> {
    info!("requesting image index from server");
    Ok(reqwest::get(
        str_to_url(url).join(AUFNAHMEN_ENDPOINT).unwrap()
    ).await?.json::<HashSet<String>>().await.unwrap())
}

pub(crate) async fn get_aufnahme(url: &str, img_path: &String) -> reqwest::Result<Vec<u8>> {
    info!("requesting image from server");
    let response = reqwest::get(str_to_url(url)
        .join(AUFNAHMEN_ENDPOINT).unwrap()
        .join(&img_path).unwrap())
        .await?
        .bytes().await?.to_vec();

    Ok(response)
}