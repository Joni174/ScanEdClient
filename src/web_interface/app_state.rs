use actix_web::{Responder, HttpResponse};
use actix_web::body::Body;
use std::fs;
use crate::web_interface::model::{Auftrag, ImageTakingStatus};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Mutex;

type WebResult = (Box<dyn AppState>, HttpResponse);

pub trait AppState {
    fn index(&self) -> HttpResponse;
    fn status(&self) -> HttpResponse;
    fn post_auftrag(self: Box<Self>, auftrag: Auftrag) -> (Box<dyn AppState>, HttpResponse);
    fn get_resulting_content(&self) -> HttpResponse;
}

fn render_master_page(html: String) -> String {
    let mut tt = tinytemplate::TinyTemplate::new();
    let master_template = std::fs::read_to_string("html/master_page.html").unwrap();
    tt.add_template(&master_template, "master").unwrap();
    tt.render("master", &html).unwrap()
}

fn render_html(path: &str) -> HttpResponse {
    let start_up_html = fs::read_to_string(path).unwrap();
    HttpResponse::Ok().body(start_up_html)
}

fn render_not_implemented_for(endpoint: &str) -> HttpResponse {
    HttpResponse::NotFound().body(format!("Status not implemented for the Phase {}", endpoint))
}

pub struct Start {}

impl AppState for Start {
    fn index(&self) -> HttpResponse {
        render_html("html/startup_page.html")
    }

    fn status(&self) -> HttpResponse {
        render_not_implemented_for("Configuration")
    }

    fn post_auftrag(self: Box<Self>, auftrag: Auftrag) -> (Box<dyn AppState>, HttpResponse) {
        let image_phase = ImagePhase {
            url: auftrag.get_url().clone(),
            round: auftrag.into_vec(),
            downloaded_images: HashSet::new(),
            server_status: Mutex::new(ImageTakingStatus::Indifferent),
            latest_aufnahme: Mutex::new(vec![])
        };
        (Box::new(image_phase), render_html("image_phase"))
    }

    fn get_resulting_content(&self) -> HttpResponse {
        render_not_implemented_for("Configuration")
    }
}

pub struct ImagePhase {
    url: String,
    round: Vec<i32>,
    downloaded_images: HashSet<String>,
    server_status: Mutex<ImageTakingStatus>,
    latest_aufnahme: Mutex<Vec<u8>>
}

impl AppState for ImagePhase {
    fn index(&self) -> HttpResponse {
        let start_up_html = fs::read_to_string("html/image_phase_page.html").unwrap();
        HttpResponse::Ok().body(start_up_html)
    }

    fn status(&self) -> HttpResponse {
        HttpResponse::Ok().json(self.server_status.lock().unwrap().clone())
    }

    fn post_auftrag(self: Box<Self>, auftrag: Auftrag) -> (Box<dyn AppState>, HttpResponse) {
        unimplemented!()
    }

    fn get_resulting_content(&self) -> HttpResponse {
        let lates_aufnahme = self.latest_aufnahme.lock().unwrap();
        HttpResponse::Ok().set_header("Content-Type", "image/jpeg")
            .body(actix_web::web::Bytes::from(lates_aufnahme.clone()))
    }
}


