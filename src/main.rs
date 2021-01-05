mod web_interface;
mod server_com;
mod photogrammetry;

use std::str::FromStr;
use actix_web::{HttpServer, App, web};
use std::net::SocketAddr;
use crate::web_interface::app_state::AppState;
use crate::web_interface::app_state;
use tokio::sync::{Mutex};
use log::{info};

mod endpoints {
    use actix_web::{Responder, web, get, post, delete, HttpRequest, HttpResponse};
    use crate::AppData;
    use crate::web_interface::model::{PageForm};
    use log::{info};

    #[get("/")]
    pub(crate) async fn index(data: web::Data<AppData>) -> impl Responder {
        info!("serving index request");
        let app_state = data.app_state.lock().await;
        app_state.as_ref().unwrap().index().await
    }

    #[delete("/")]
    pub(crate) async fn reset(data: web::Data<AppData>) -> impl Responder {
        info!("serving reset request");
        let mut app_state = data.app_state.lock().await;
        let (new_app_state, res) = app_state.take().unwrap().reset().await;
        *app_state = Some(new_app_state);
        res
    }

    #[get("/status")]
    pub(crate) async fn status(data: web::Data<AppData>) -> impl Responder {
        info!("serving status request");
        let app_state = data.app_state.lock().await;
        app_state.as_ref().unwrap().status().await
    }

    #[post("/page_form")]
    pub(crate) async fn post_page_form(page_form: web::Form<PageForm>, data: web::Data<AppData>) -> impl Responder {
        info!("serving page_form post request");
        let mut app_state = data.app_state.lock().await;
        let (new_app_state, res) = app_state.take().unwrap()
            .post_page_form(page_form.0).await;
        *app_state = Some(new_app_state);
        res
    }

    #[get("/media_content")]
    pub(crate) async fn get_media_content(data: web::Data<AppData>) -> impl Responder {
        info!("serving media_content index");
        let app_state = data.app_state.lock().await;
        app_state.as_ref().unwrap().get_content().await
    }

    #[get("/media_content/{image_name}")]
    pub(crate) async fn get_specific_media_content(data: web::Data<AppData>, req: HttpRequest) -> impl Responder {
        info!("serving specific media_content");
        let app_state = data.app_state.lock().await;
        match req.match_info().get("image_name"){
            None => {HttpResponse::BadRequest().finish()}
            Some(image_name) => {
                app_state.as_ref().unwrap().get_specific_content(image_name).await
            }
        }
    }

    #[get("/ws_notification")]
    pub(crate) async fn ws_notification(req: HttpRequest, stream: web::Payload, data: web::Data<AppData>) -> impl Responder {
        info!("serving ws_notification");
        let app_state = data.app_state.lock().await;
        app_state.as_ref().unwrap().ws_notification(req, stream).await
    }

    #[get("/0_0.jpeg")]
    pub async fn image() -> impl Responder {
        let img = tokio::fs::read("0_0.jpg").await.unwrap();
        HttpResponse::Ok()
            .header("Content-Type", "image/jpeg")
            .header("Content-Length", img.len().to_string())
            .body(img)
    }
}

struct AppData {
    app_state: Mutex<Option<Box<dyn AppState + Send>>>,
}

#[actix_web::main]
async fn main() {
    let app_data = web::Data::new(AppData {
        app_state: Mutex::new(Some(Box::new(app_state::Start {}))),
    });

    env_logger::Builder::from_env(env_logger::Env::default()
        .default_filter_or("info")).init();

    info!("starting client");

    HttpServer::new(move || {
        App::new()
            .service(endpoints::index)
            .service(endpoints::status)
            .service(endpoints::post_page_form)
            .service(endpoints::get_media_content)
            .service(endpoints::get_specific_media_content)
            .service(endpoints::image)
            .service(endpoints::ws_notification)
            .app_data(app_data.clone())
    }).bind(SocketAddr::from_str("0.0.0.0:8080").unwrap())
        .unwrap()
        .run().await.unwrap();
}