use actix_web::{HttpResponse, body::Body, HttpRequest, web, Error};
use crate::web_interface::model::{PageForm, ImageTakingStatus};
use actix_web::dev::HttpResponseBuilder;
use std::fs;
use std::collections::HashSet;
use std::sync::{Arc};
use tokio::sync::Mutex;
use crossbeam_channel::Receiver;
use actix_web_actors::ws;
use crate::web_interface::model::ws::{MyWs, Notification};
use actix::{Actor, Addr};
use crate::server_com;
use crate::photogrammetry::image_handling::ImageStore;
use async_trait::async_trait;
use std::ops::Deref;
use actix_web::rt::time::delay_for;
use tokio::time::Duration;
use crate::web_interface::app_state::constants::POLL_DELAY;
use crate::server_com::{get_ready_image_list, get_aufnahme};

mod constants {
    pub const CONTENT: &'static str = "media_content";
    pub const POLL_DELAY: u64 = 3; // in seconds
}


#[async_trait]
pub trait AppState {
    async fn index(&self) -> HttpResponse;
    async fn status(&self) -> HttpResponse;
    async fn reset(self: Box<Self>) -> (Box<dyn AppState + Sync + Send>, HttpResponse);
    async fn post_page_form(self: Box<Self>, page_form: PageForm) -> (Box<dyn AppState + Sync + Send>, HttpResponse);
    async fn get_content(&self) -> HttpResponse;
    async fn get_specific_content(&self, name: &str) -> HttpResponse;
    fn ws_notification(&self, req: HttpRequest, stream: web::Payload) -> HttpResponse;
}

fn render_master_page(html: String) -> String {
    let mut tt = tinytemplate::TinyTemplate::new();
    let master_template = std::fs::read_to_string("html/master.html").unwrap();
    tt.add_template(&master_template, "master").unwrap();
    tt.render("master", &html).unwrap()
}

fn add_html_body_from_file(mut http_response: HttpResponseBuilder, path: &str) -> HttpResponse {
    let start_up_html = fs::read_to_string(path).unwrap();
    http_response.body(Body::from(start_up_html))
}

fn endpoint_not_found_in_phase(endpoint: &str, phase: &str) -> HttpResponse {
    HttpResponse::NotFound()
        .body(format!("{} not implemented for the Phase {}", endpoint, phase))
}

fn redirect_response(path: &str) -> HttpResponse {
    HttpResponse::SeeOther().
        header("location", path).finish()
}

#[derive(Clone)]
pub struct Start {}

#[async_trait]
impl AppState for Start {
    async fn index(&self) -> HttpResponse {
        add_html_body_from_file(HttpResponse::Ok(), "html/startup_page.html")
    }

    async fn status(&self) -> HttpResponse {
        endpoint_not_found_in_phase("/status(get)", "Configuration")
    }

    async fn reset(self: Box<Self>) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        (self, endpoint_not_found_in_phase("/(delete)", "Configuration"))
    }

    async fn post_page_form(self: Box<Self>, page_form: PageForm) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        let auftrag = if let PageForm::Auftrag(auftrag) = page_form {
            auftrag
        } else {
            return (
                self,
                HttpResponse::InternalServerError().body("Invalid Form was Submitted")
            );
        };

        // if initializing folder fails return error
        let image_phase = match ImagePhase::new(
            auftrag.get_url().to_string(),
            auftrag.into_vec()).await {
            Ok(image_phase) => image_phase,
            Err(err) => {
                return (self, HttpResponse::InternalServerError().body(err.to_string()));
            }
        };
        // start downloading images
        image_phase.start_server_com().await;

        (
            Box::new(image_phase),
            redirect_response("/")
        )
    }

    async fn get_content(&self) -> HttpResponse {
        endpoint_not_found_in_phase("media_content", "Configuration")
    }

    async fn get_specific_content(&self, name: &str) -> HttpResponse {
        endpoint_not_found_in_phase("media_content/{content_name}", "Configuration")
    }

    fn ws_notification(&self, req: HttpRequest, stream: web::Payload) -> HttpResponse {
        endpoint_not_found_in_phase("ws_notification", "Configuration")
    }
}

pub struct ImagePhase {
    url: String,
    rounds: Vec<i32>,
    server_status: Arc<std::sync::Mutex<ImageTakingStatus>>,
    new_status_notifier: Arc<std::sync::Mutex<Option<Addr<MyWs>>>>,
    image_store: Arc<ImageStore>,
}

impl ImagePhase {
    async fn new(url: String, rounds: Vec<i32>) -> tokio::io::Result<ImagePhase> {
        Ok(ImagePhase {
            url,
            rounds,
            server_status: Arc::new(std::sync::Mutex::new(ImageTakingStatus::Start)),
            new_status_notifier: Arc::new(std::sync::Mutex::new(None)),
            image_store: Arc::new(ImageStore::new().await?),
        })
    }

    async fn start_server_com(&self) {
        let url = self.url.clone();
        let rounds = self.rounds.clone();
        let notification_handle = Arc::clone(&self.new_status_notifier);
        let image_store = Arc::clone(&self.image_store);
        tokio::spawn(async move {
            if let Err(err) = server_com::post_auftrag(
                server_com::com_model::RunConfig::from_vec(rounds),
                &url,
            ).await {
                    log::error!("{}", err.to_string());
                    return;
            }

            loop {
                if image_store.download_new_images(&url).await {
                    if let Some(handle) = notification_handle.lock().unwrap().as_ref() {
                        handle.do_send(Notification(String::from("new image")));
                    }
                    delay_for(tokio::time::Duration::from_secs(POLL_DELAY)).await;
                }
            }
        });
    }
}

#[async_trait]
impl AppState for ImagePhase {
    async fn index(&self) -> HttpResponse {
        add_html_body_from_file(HttpResponse::Ok(), "html/image_phase_page.html")
    }

    async fn status(&self) -> HttpResponse {
        //crate::server_com::get_status(&self.url)
        HttpResponse::Ok().json(self.server_status.lock().unwrap().clone())
    }

    async fn reset(self: Box<Self>) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        if let Err(err) = self.image_store.reset().await {
            (self, HttpResponse::InternalServerError().body(err.to_string()))
        } else {
            (Box::new(Start {}), HttpResponse::Ok().finish())
        }
    }

    async fn post_page_form(self: Box<Self>, page_form: PageForm) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        unimplemented!()
    }

    async fn get_content(&self) -> HttpResponse {
        let image_list = self.image_store.get_image_list().await
            .iter()
            .map(|image_name| format!("/{}/{}", constants::CONTENT, image_name))
            .collect::<Vec<_>>();

        HttpResponse::Ok().set_header("Content-Type", "text/json")
            .json(&image_list)
    }

    async fn get_specific_content(&self, name: &str) -> HttpResponse {
        match self.image_store.get_image(&name.to_string()).await {
            Ok(img) => {
                HttpResponse::Ok().body(actix_web::web::Bytes::from(img))
            }
            Err(err) => {
                match err {
                    None => {
                        // image not found in image store
                        HttpResponse::NotFound().finish()
                    }
                    Some(io_error) => {
                        // error accessing image
                        HttpResponse::InternalServerError().body(io_error.to_string())
                    }
                }
            }
        }
    }

    fn ws_notification(&self, req: HttpRequest, stream: web::Payload) -> HttpResponse {
        let notifier = Arc::clone(&self.new_status_notifier);
        match ws::start(MyWs::new(notifier), &req, stream)
        {
            Ok(res) => res,
            Err(err) => HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}