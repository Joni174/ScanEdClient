use reqwest::{Response, Url};
use serde::{Serialize, Deserialize};
use std::str::FromStr;
use actix_web::web;

static AUFTRAG_ENPOINT: &'static str = "auftrag";
static AUFNAHMEN_ENDPOINT: &'static str = "aufnahmen";

#[derive(Deserialize, Serialize)]
pub struct ImageTakingStatus {
    runde: i32,
    aufnahme: i32,
}

#[derive(Serialize, PartialEq, Deserialize, Debug)]
struct Round {
    anzahl: i32
}

#[derive(Serialize, PartialEq, Deserialize, Debug)]
struct RoundConfig {
    runde: Round
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct RunConfig {
    rounds: Vec<RoundConfig>,
    url: String,
}

pub async fn get_status(run_config: &RunConfig) -> reqwest::Result<ImageTakingStatus> {
    reqwest::get(str_to_url(&run_config.url).join(AUFTRAG_ENPOINT).unwrap()).await?
        .json::<ImageTakingStatus>().await
}

fn str_to_url(str: &str) -> Url {
    reqwest::Url::from_str(&str).unwrap()
}

pub async fn post_run_config(run_config: &RunConfig) -> reqwest::Result<Response> {
    let client = reqwest::Client::new();
    client.post(str_to_url(&run_config.url).join(AUFTRAG_ENPOINT).unwrap())
        .json(&run_config.rounds)
        .send().await
}

pub(crate) async fn get_ready_image_list(run_config: &RunConfig) -> reqwest::Result<Vec<String>> {
    Ok(reqwest::get(
        str_to_url(&run_config.url).join(AUFNAHMEN_ENDPOINT).unwrap()
    ).await?.json::<Vec<String>>().await.unwrap())
}

pub(crate) async fn get_aufnahme(run_config: &RunConfig, img_path: &String) -> reqwest::Result<web::Bytes> {
    let response = reqwest::get(str_to_url(&run_config.url).join(&img_path).unwrap())
        .await?
        .bytes().await?;

    Ok(response)
}