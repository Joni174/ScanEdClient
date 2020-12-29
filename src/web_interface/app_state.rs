use actix_web::{HttpResponse, body::Body, HttpRequest, web, Error};
use crate::web_interface::model::{PageForm, ImageTakingStatus};
use actix_web::dev::HttpResponseBuilder;
use std::fs;
use std::collections::HashSet;
use std::sync::{Mutex, Arc};
use crossbeam_channel::Receiver;
use actix_web_actors::ws;
use crate::web_interface::model::ws::{MyWs, Notifier};
use crate::server_com::start_server_com;
use actix::Actor;

pub trait AppState {
    fn index(&self) -> HttpResponse;
    fn status(&self) -> HttpResponse;
    fn post_page_form(self: Box<Self>, page_form: PageForm) -> (Box<dyn AppState + Sync + Send>, HttpResponse);
    fn get_resulting_content(&self) -> HttpResponse;
    fn ws_notification(&self, req: HttpRequest, stream: web::Payload, notifier: Arc<Mutex<Notifier>>) -> HttpResponse;
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

fn render_not_implemented_for(endpoint: &str) -> HttpResponse {
    HttpResponse::NotFound().body(format!("Status not implemented for the Phase {}", endpoint))
}

fn redirect_response(path: &str) -> HttpResponse {
    HttpResponse::SeeOther().
        header("location", path).finish()
}

#[derive(Clone)]
pub struct Start {}

impl AppState for Start {
    fn index(&self) -> HttpResponse {
        add_html_body_from_file(HttpResponse::Ok(), "html/startup_page.html")
    }

    fn status(&self) -> HttpResponse {
        render_not_implemented_for("Configuration")
    }

    fn post_page_form(self: Box<Self>, page_form: PageForm) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        let auftrag = if let PageForm::Auftrag(auftrag) = page_form { auftrag } else {
            return (
                self,
                HttpResponse::InternalServerError().body("Invalid Form was Submitted")
            );
        };
        let image_phase = ImagePhase::new(auftrag.get_url().to_string(), auftrag.into_vec());
        (
            Box::new(image_phase),
            redirect_response("/")
        )
    }

    fn get_resulting_content(&self) -> HttpResponse {
        render_not_implemented_for("Configuration")
    }

    fn ws_notification(&self, req: HttpRequest, stream: web::Payload, notifier: Arc<Mutex<Notifier>>) -> HttpResponse {
        unimplemented!()
    }
}

#[derive(Clone)]
pub struct ImagePhase {
    url: String,
    latest_aufnahme: Arc<Mutex<Vec<u8>>>,
    server_status: Arc<Mutex<ImageTakingStatus>>,
    notification_receiver: Receiver<String>,
}

impl ImagePhase {
    fn new(url: String, round: Vec<i32>) -> ImagePhase {
        let (tx, rx) = crossbeam_channel::unbounded();
        start_server_com(url.to_string(), round, tx);
        ImagePhase {
            url,
            latest_aufnahme: Arc::new(Mutex::new(vec![])),
            server_status: Arc::new(Mutex::new(ImageTakingStatus::Start)),
            notification_receiver: rx,
        }
    }
}

impl AppState for ImagePhase {
    fn index(&self) -> HttpResponse {
        add_html_body_from_file(HttpResponse::Ok(), "html/image_phase_page.html")
    }

    fn status(&self) -> HttpResponse {
        //crate::server_com::get_status(&self.url)
        HttpResponse::Ok().json(self.server_status.lock().unwrap().clone())
    }

    fn post_page_form(self: Box<Self>, page_form: PageForm) -> (Box<dyn AppState + Sync + Send>, HttpResponse) {
        unimplemented!()
    }

    fn get_resulting_content(&self) -> HttpResponse {
        let latest_aufnahme = self.latest_aufnahme.lock().unwrap();
        HttpResponse::Ok().set_header("Content-Type", "image/jpeg")
            .body(actix_web::web::Bytes::from(latest_aufnahme.clone()))
    }

    fn ws_notification(&self, req: HttpRequest, stream: web::Payload, notifier: Arc<Mutex<Notifier>>) -> HttpResponse {
        match ws::start(MyWs::new(notifier), &req, stream) {
            Ok(res) => res,
            Err(err) => HttpResponse::InternalServerError().body(err.to_string())
        }
        // unimplemented!()
    }
}