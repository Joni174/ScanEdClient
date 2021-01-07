use actix_web::{HttpResponse, HttpRequest, web};
use crate::web_interface::model::{PageForm, NotificationHandle};
use std::fs;
use std::sync::{Arc, Mutex};
use actix_web_actors::ws;
use crate::web_interface::model::ws::{MyWs};
use actix::{Addr};
use crate::photogrammetry::image_handling::{ImageDownloader};
use async_trait::async_trait;
use std::ops::Deref;
use crate::server_com::{com_model};
use actix_web::web::Payload;
use crate::photogrammetry::photogrammetry::start_photogrammetry;
use serde::Serialize;
use std::error::Error;
use crate::server_com;
use log::{warn};

mod constants {
    pub const CONTENT: &'static str = "media_content";
}

#[derive(Serialize)]
struct MasterTemplateContext {
    page_content: String
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
    tt.add_template("master", &master_template).unwrap();
    tt.render("master", &MasterTemplateContext { page_content: html }).unwrap()
}

fn render_page(path: &str) -> HttpResponse {
    let page_html = fs::read_to_string(path).unwrap();
    let rendered_html = render_master_page(page_html);
    HttpResponse::Ok().body(rendered_html)
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
        render_page("html/startup_page.html")
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

        // parse form
        let rounds = match auftrag.clone().into_vec() {
            Ok(rounds) => rounds,
            Err(err) => {
                return (self, HttpResponse::InternalServerError().body(err.to_string()));
            }
        };

        // if initializing folder or post request to server fails return error
        let image_phase = match ImagePhase::new(
            auftrag.get_url().to_string(),
            rounds,
        ).await {
            Ok(image_phase) => image_phase,
            Err(err) => {
                return (self, HttpResponse::InternalServerError().body(err.to_string()));
            }
        };

        (
            Box::new(image_phase),
            redirect_response("/")
        )
    }

    async fn get_content(&self) -> HttpResponse {
        endpoint_not_found_in_phase("media_content", "Configuration")
    }

    async fn get_specific_content(&self, _name: &str) -> HttpResponse {
        endpoint_not_found_in_phase("media_content/{content_name}", "Configuration")
    }

    fn ws_notification(&self, _req: HttpRequest, _stream: web::Payload) -> HttpResponse {
        endpoint_not_found_in_phase("ws_notification", "Configuration")
    }
}

pub struct ImagePhase {
    new_status_notifier: Arc<std::sync::Mutex<Option<Addr<MyWs>>>>,
    image_downloader: Arc<ImageDownloader>,
}

impl ImagePhase {
    async fn new(url: String, rounds: Vec<i32>) -> Result<ImagePhase, Box<dyn Error + Send>> {
        server_com::post_auftrag(com_model::Auftrag::from_vec(rounds.clone()), &url).await?;
        let new_status_notifier = Arc::new(Mutex::new(None));
        let image_downloader = Arc::new(ImageDownloader::new(
            url.clone(),
            com_model::Auftrag::from_vec(rounds.clone()).into_target_status(),
            Arc::clone(&new_status_notifier)).await?);
        Arc::clone(&image_downloader).start().await;
        Ok(ImagePhase {
            new_status_notifier,
            image_downloader,
        })
    }
}

#[async_trait]
impl AppState for ImagePhase {
    async fn index(&self) -> HttpResponse {
        render_page("html/image_phase_page.html")
    }

    async fn status(&self) -> HttpResponse {
        let status = self.image_downloader.get_status().await;
        HttpResponse::Ok().json(status)
    }

    async fn reset(self: Box<Self>) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        self.image_downloader.reset().await;
        (Box::new(Start {}), redirect_response("/"))
    }

    async fn post_page_form(self: Box<Self>, _page_form: PageForm) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        let (sender, receiver) = tokio::sync::oneshot::channel::<()>();
        let photogrammetry_phase = PhotogrammetryPhase::new(sender);
        start_photogrammetry(
            Arc::clone(&photogrammetry_phase.new_status),
            Arc::clone(&photogrammetry_phase.console_output),
            receiver,
        ).await;
        (Box::new(photogrammetry_phase), redirect_response("/"))
        // {
        //     Ok(_) => {(Box::new(photogrammetry_phase), redirect_response("/"))},
        //     Err(err) => {(self, HttpResponse::InternalServerError().body(err.to_string()))}
    }

    async fn get_content(&self) -> HttpResponse {
        let image_list = self.image_downloader.get_image_list().await
            .iter()
            .map(|image_name| format!("/{}/{}", constants::CONTENT, image_name))
            .collect::<Vec<_>>();

        HttpResponse::Ok().set_header("Content-Type", "text/json")
            .json(&image_list)
    }

    async fn get_specific_content(&self, name: &str) -> HttpResponse {
        match self.image_downloader.get_image(&name.to_string()).await {
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

pub struct PhotogrammetryPhase {
    console_output: Arc<tokio::sync::Mutex<Vec<serde_json::Value>>>,
    new_status: NotificationHandle,
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl PhotogrammetryPhase {
    fn new(sender: tokio::sync::oneshot::Sender<()>) -> PhotogrammetryPhase {
        PhotogrammetryPhase {
            console_output: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            new_status: Arc::new(Mutex::new(None)),
            shutdown_tx: sender,
        }
    }
}

#[async_trait]
impl AppState for PhotogrammetryPhase {
    async fn index(&self) -> HttpResponse {
        render_page("html/photogrammetry_page.html")
    }

    async fn status(&self) -> HttpResponse {
        endpoint_not_found_in_phase("status", "PhotogrammetryPhase")
    }

    #[allow(unused_must_use)]
    async fn reset(self: Box<Self>) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        if let Err(_err) = self.shutdown_tx.send(()) {
            warn!("photogrammetry process already dead");
        }
        (Box::new(Start {}), redirect_response("/"))
    }

    async fn post_page_form(self: Box<Self>, _page_form: PageForm) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        (Box::new(ModelPhase {}), redirect_response("/"))
    }

    async fn get_content(&self) -> HttpResponse {
        log::error!("get_content_lock");
        let body = self.console_output.lock().await.deref().clone();
        log::error!("release_content_lock");

        HttpResponse::Ok().json(body)
    }

    async fn get_specific_content(&self, _name: &str) -> HttpResponse {
        endpoint_not_found_in_phase("/media_content/{content_name}", "PhotogrammetryPhase")
    }

    fn ws_notification(&self, req: HttpRequest, stream: Payload) -> HttpResponse {
        let notifier = Arc::clone(&self.new_status);
        match ws::start(MyWs::new(notifier), &req, stream)
        {
            Ok(res) => res,
            Err(err) => HttpResponse::InternalServerError().body(err.to_string())
        }
    }
}

pub struct ModelPhase {}

#[async_trait]
impl AppState for ModelPhase {
    async fn index(&self) -> HttpResponse {
        render_page("html/model_page.html")
    }

    async fn status(&self) -> HttpResponse {
        endpoint_not_found_in_phase("/status", "Model")
    }

    async fn reset(self: Box<Self>) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        (Box::new(Start {}), redirect_response("/"))
    }

    async fn post_page_form(self: Box<Self>, _page_form: PageForm) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        (self, endpoint_not_found_in_phase("/page_form(post)", "Model"))
    }

    async fn get_content(&self) -> HttpResponse {
        //return 3d model as zip
        match tokio::fs::read("/model.zip").await {
            Ok(file) => {
                HttpResponse::Ok()
                    .header("Content-Type", "application/octet-stream")
                    .body(file)
            }
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
        }
    }

    async fn get_specific_content(&self, _name: &str) -> HttpResponse {
        endpoint_not_found_in_phase("/media_content/{content_id}", "Model")
    }

    fn ws_notification(&self, _req: HttpRequest, _stream: Payload) -> HttpResponse {
        endpoint_not_found_in_phase("/ws_notification", "Model")
    }
}