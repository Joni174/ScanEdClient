use actix_web::{Responder, HttpResponse};
use actix_web::body::Body;
use std::fs;
use crate::web_interface::model::Auftrag;
use serde_json::{json, Value};
use std::collections::HashSet;

pub trait AppState {
    fn index() -> HttpResponse;
    fn status() -> HttpResponse;
    fn post_auftrag(auftrag: Auftrag) -> (Option<Box<Self>>, HttpResponse);
    fn get_aufnahmen(&self) -> HttpResponse;
    fn get_aufnahme(&self, auftrag_name: &str) -> HttpResponse;
    fn get_3d_model() -> HttpResponse;
    fn upgrade(&self) -> Option<Box<Self>>;
}

fn render_master_page(html: String) -> String {
    let mut tt = tinytemplate::TinyTemplate::new();
    let master_template = std::fs::read_to_string("html/master_page.html").unwrap();
    tt.add_template(&master_template, "master").unwrap();
    tt.render("master", &html).unwrap()
}

pub struct Start {}

impl AppState for Start {
    fn index() -> HttpResponse {
        let start_up_html = fs::read_to_string("html/startup_page.html").unwrap();
        HttpResponse::Ok().body(start_up_html)
    }

    fn status() -> HttpResponse {
        HttpResponse::NotFound().body("Status not implemented for the Phase Startup")
    }

    fn post_auftrag(auftrag: Auftrag) -> (Option<Box<Self>>, HttpResponse) {
        let image_phase = ImagePhase{url: auftrag.downloaded_images: HashSet::new()}
    }

    fn get_aufnahmen(&self) -> HttpResponse {
        unimplemented!()
    }

    fn get_aufnahme(&self, auftrag_name: &str) -> HttpResponse {
        unimplemented!()
    }

    fn get_3d_model() -> HttpResponse {
        unimplemented!()
    }

    fn upgrade(&self) -> Option<Box<Self>> {
        unimplemented!()
    }
}

pub struct ImagePhase {
    url: String,
    round: Vec<i32>,
    downloaded_images: HashSet<String>
}

impl AppState for ImagePhase {
    fn index() -> HttpResponse {
        let start_up_html = fs::read_to_string("html/image_phase_page.html").unwrap();
        HttpResponse::Ok().body(start_up_html)
    }

    fn status() -> HttpResponse {
        unimplemented!()
    }

    fn post_auftrag(auftrag: Auftrag) -> (Option<Box<Self>>, HttpResponse) {
        unimplemented!()
    }

    fn get_aufnahmen(&self) -> HttpResponse {
        unimplemented!()
    }

    fn get_aufnahme(&self, auftrag_name: &str) -> HttpResponse {
        unimplemented!()
    }

    fn get_3d_model() -> HttpResponse {
        unimplemented!()
    }

    fn upgrade(&self) -> Option<Box<Self>> {
        unimplemented!()
    }
}


