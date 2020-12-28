use actix_web::{HttpResponse, body::Body};
use crate::web_interface::model::{PageForm, ImageTakingStatus};
use actix_web::dev::HttpResponseBuilder;
use std::fs;
use std::collections::HashSet;
use std::sync::Mutex;

pub trait AppState {
    fn index(&self) -> HttpResponse;
    fn status(&self) -> HttpResponse;
    fn post_page_form(self: Box<Self>, page_form: PageForm) -> (Box<dyn AppState>, HttpResponse);
    fn get_resulting_content(&self) -> HttpResponse;
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

pub struct Start {}

impl AppState for Start {
    fn index(&self) -> HttpResponse {
        add_html_body_from_file(HttpResponse::Ok(), "html/startup_page.html")
    }

    fn status(&self) -> HttpResponse {
        render_not_implemented_for("Configuration")
    }

    fn post_page_form(self: Box<Self>, page_form: PageForm) -> (Box<dyn AppState>, HttpResponse) {
        let auftrag = if let PageForm::Auftrag(auftrag) = page_form { auftrag } else {
            return (
                self,
                HttpResponse::InternalServerError().body("Post Request not allowed in this state")
            );
        };
        let image_phase = ImagePhase {
            url: auftrag.get_url().clone(),
            round: auftrag.into_vec(),
            downloaded_images: HashSet::new(),
            server_status: Mutex::new(ImageTakingStatus::Indifferent),
            latest_aufnahme: Mutex::new(vec![]),
        };
        (
            Box::new(image_phase),
            redirect_response("/")
        )
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
    latest_aufnahme: Mutex<Vec<u8>>,
}

impl AppState for ImagePhase {
    fn index(&self) -> HttpResponse {
        add_html_body_from_file(HttpResponse::Ok(), "html/image_phase_page.html")
    }

    fn status(&self) -> HttpResponse {
        HttpResponse::Ok().json(self.server_status.lock().unwrap().clone())
    }

    fn post_page_form(self: Box<Self>, page_form: PageForm) -> (Box<dyn AppState>, HttpResponse) {
        unimplemented!()
    }

    fn get_resulting_content(&self) -> HttpResponse {
        let latest_aufnahme = self.latest_aufnahme.lock().unwrap();
        HttpResponse::Ok().set_header("Content-Type", "image/jpeg")
            .body(actix_web::web::Bytes::from(latest_aufnahme.clone()))
    }
}