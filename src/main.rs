use reqwest::{Response, Url};
use std::path::{PathBuf};
use std::str::FromStr;
use std::fs::File;
use simple_logger::SimpleLogger;
use std::io::{copy};
use serde::Deserialize;
use std::fmt::Display;
use serde::export::Formatter;
use std::ops::Add;
use std::error::Error;
use std::process::exit;
use log::{info, error};

static AUFTRAG_ENPOINT: &'static str = "auftrag";
static AUFNAHMEN_ENDPOINT: &'static str = "aufnahmen";

struct Client {
    run_config: RunConfig,
    client: reqwest::Client,
    images_downloaded: Vec<String>,
}

impl Client {
    fn new(run_config: RunConfig) -> Client {
        Client { run_config, client: reqwest::Client::new(), images_downloaded: Vec::new() }
    }

    ///
    /// This starts the scanning process
    /// with the configuration as a parameter
    /// Returns: () if ok reqwest::Error otherwise
    ///
    async fn start_job(&self) -> reqwest::Result<()> {
        self.client.post(self.run_config.url.join(AUFTRAG_ENPOINT).unwrap())
            .json(&self.run_config.rounds)
            .send().await?
            .error_for_status().map(|_| ())
    }

    async fn get_ready_image_list(&self) -> reqwest::Result<Vec<String>> {
        let resp = self.client.get(
            self.run_config.url.join(AUFNAHMEN_ENDPOINT).unwrap()
        )
            .send()
            .await?
            .json::<Vec<String>>()
            .await?;
        Ok(resp)
    }

    async fn get_status(&self) -> reqwest::Result<Status> {
        let incoming_status = self.client.get(self.run_config.url.join(AUFTRAG_ENPOINT).unwrap())
            .send().await?
            .json::<IncomingStatus>().await?;
        let this_round = incoming_status.runde;
        Ok(incoming_status.into_status(
            self.run_config.rounds.len() as i32,
            *self.run_config.rounds.get(this_round as usize)
                .expect("got invalid status from server")))
    }

    fn filter_downloaded(&self, image_paths: Vec<String>) -> Vec<String> {
        image_paths.into_iter()
            .filter(|image_path| self.images_downloaded.contains(image_path))
            .collect()
    }

    async fn download_image(&mut self, image_path: String) -> reqwest::Result<Response> {
        self.images_downloaded.push(image_path.clone());
        self.client.get(self.run_config.url.join(&image_path).unwrap())
            .send().await
    }

    async fn download_downloadable_images(&mut self) -> Result<Vec<String>, Box<dyn Error>> {
        let downloadable_images = self.filter_downloaded(self.get_ready_image_list().await?);
        let mut new_image_messages = Vec::new();
        for image_path in downloadable_images {
            let new_image_name = self.images_downloaded.len().to_string().add(".jpg");
            new_image_messages.push(format!("{} -> {}", image_path, new_image_name));
            write_response_body_to_file(
                File::open(self.run_config.temp_dir.join(&new_image_name))
                    .expect(&format!("unable to open file: {}", new_image_name)),
                self.download_image(image_path).await?,
            ).await?;
        }
        Ok(new_image_messages)
    }
}

async fn write_response_body_to_file(mut file: File, response: Response) -> Result<(),Box<dyn Error>> {
    // copy(response., &mut file)
    //     .expect("unable to write image into local file");
    copy(&mut response
        .bytes().await?.iter().as_slice(), &mut file).map(|_| ()).map_err(|e| e.into())
}

#[derive(PartialEq)]
struct Progress {
    round_now: i32,
    round_max: i32,
    image_now: i32,
    image_max: i32,
}

impl Progress {
    fn new() -> Progress {
        Progress {
            round_now: 0,
            round_max: 0,
            image_now: 0,
            image_max: 0,
        }
    }
}

impl Display for Progress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Runde ({}/{}) | Aufnahme ({:03}/{:03})",
               self.round_now,
               self.round_max,
               self.image_now,
               self.image_max)
    }
}

#[derive(PartialEq)]
enum Status {
    InProgress(Progress),
    Done,
}

#[derive(Deserialize)]
struct IncomingStatus {
    runde: i32,
    aufnahme: i32,
}

impl IncomingStatus {
    fn into_status(self, round_max: i32, image_max: i32) -> Status {
        if self.runde == round_max && self.aufnahme == image_max {
            Status::Done
        } else {
            Status::InProgress(Progress { round_now: self.runde, round_max, image_now: self.aufnahme, image_max })
        }
    }
}

#[derive(Debug)]
struct RunConfig {
    rounds: Vec<i32>,
    temp_dir: PathBuf,
    url: Url,
}

const RUN_CONFIG_SPARSE: &'static [i32] = &[20];
const RUN_CONFIG_MIDDLE: &'static [i32] = &[60, 20];
const RUN_CONFIG_DENSE: &'static [i32] = &[150, 50, 20];

impl RunConfig {
    fn new(rounds: Vec<i32>, temp_dir: &str, hostname: &str, port: &str) -> RunConfig {
        let path = PathBuf::from(temp_dir);

        let port = port.parse::<u16>().expect(
            &format!("invalid port number: {}", port)
        );

        let url = Url::from_str(&format!("http://{}:{}", hostname, port)).expect("invalid hostname");

        RunConfig { rounds, temp_dir: path, url }
    }
    fn from_cli_arguments() -> RunConfig {
        use clap::{App, Arg};

        let matches = App::new("Scan3d Client for image capturing and photogrammetry.")
            .about("Must be used in combination with the Scan3d Platform. Open Drone Map is currently used as Photogrammetry software.")
            .version("v0.1")
            .author("Jonas Gastecker jonas.gastecker@sz-ybbs.ac.at")
            .arg(Arg::with_name("density")
                .possible_values(&["sparse", "middle", "dense"])
                .short("d")
                .long("density")
                .required_unless("configuration")
                .takes_value(true)
                .help("Selection of a standard configuration."))
            .arg(Arg::with_name("configuration")
                .short("c")
                .long("configuration")
                .value_name("config")
                .help("Is used to provide the number of images per round e.g. 80,40,10 (round 1: 80 images, round 2: 40 images, round 3: 10 images).")
                .conflicts_with("density")
                .takes_value(true))
            .arg(Arg::with_name("temp-dir")
                .short("t")
                .long("temp")
                .value_name("path")
                .help("Provide a folder to store images temporarily.")
                .default_value(".img_scan3d")
                .takes_value(true))
            .arg(Arg::with_name("server-address")
                .short("h")
                .long("hostname")
                .value_name("hostname")
                .default_value("10.0.0.1")
                .help("Hostname of the server"))
            .arg(Arg::with_name("port")
                .short("p")
                .long("path")
                .value_name("port")
                .default_value("8080")
                .help("Port number which is used on the server."))
            .get_matches();

        let number_images = if let Some(density) = matches.value_of("density") {
            match density {
                "sparse" => { RUN_CONFIG_SPARSE.to_vec() }
                "middle" => { RUN_CONFIG_MIDDLE.to_vec() }
                "dense" => { RUN_CONFIG_DENSE.to_vec() }
                _ => { panic!("invalid density value provided") }
            }
        } else {
            // when no density is provided a custom value for images per round must be given
            let configuration_string = matches.value_of("configuration")
                .expect("some configuration for the number of images must be provided");
            configuration_string
                .split(",")
                .map(|number_of_images|
                    number_of_images.parse::<i32>()
                        .expect(&*format!("provided configuration is invalid: {}",
                                          configuration_string))).collect()
        };

        let hostname = matches.value_of("server-address").expect("no hostname provided");
        let port = matches.value_of("port").expect("no port provided");
        let temp_dir = matches.value_of("temp-dir").expect("no temp dir provided");

        RunConfig::new(number_images, temp_dir, hostname, port)
    }
}

#[tokio::main]
async fn main() -> reqwest::Result<()> {
    SimpleLogger::new().with_level(log::LevelFilter::Info).init().unwrap();
    let run_config = RunConfig::from_cli_arguments();
    if let Err(err) = initialize_directory(&run_config.temp_dir) {
        error!("{}", err);
        exit(1);
    };

    let mut client = Client::new(run_config);

    info!("Sende Auftrag...");
    if let Err(err) = client.start_job().await {
        error!("{}", err);
        exit(1);
    } else {
        info!("Auftrag OK");
    };

    let mut old_progress = Progress::new();

    loop {
        // delay between polls
        tokio::time::delay_for(tokio::time::Duration::new(2,0)).await;

        // check status
        info!("Sende Statusabfrage...");
        let progress = if let Status::InProgress(progress) = client.get_status().await? {
            info!("Vortschritt: {}", progress);
            progress
        } else {
            info!("Auftrag Abgeschlossen");
            break;
        };

        if old_progress.ne(&progress) {
            info!("Neue Aufnahmen werden herunter geladen...");
            old_progress = progress;
            match client.download_downloadable_images().await {
                Ok(downloaded_images) => {
                    info!("Folgende Aufnahmen wurden heruntergeladen: ---\n{}\n---", downloaded_images.join("\n"));
                }
                Err(err) => {
                    error!("{}", err);
                    break;
                }
            };
        }
    }
    Ok(())
}

fn initialize_directory(directory: &PathBuf) -> std::io::Result<()> {
    use std::fs;
    if directory.exists() {
        fs::remove_dir_all(directory)?;
    }
    fs::create_dir(directory)?;

    Ok(())
}
