mod web_interface;
mod server_com;

use std::str::FromStr;
use actix_web::{HttpServer, App, web};
use std::net::SocketAddr;
use crate::web_interface::app_state::AppState;
use crate::web_interface::app_state;
use crate::web_interface::model::ws::{Notification, MyWs};
use std::sync::{Arc, Mutex};
use std::thread;
use std::ops::Deref;
use actix::Actor;
//
// #[derive(Debug, PartialEq)]
// enum AppState {
//     Start,
//     ImageTaking(server_com::RunConfig, DownloadState),
//     Photogrammetry,
//     Finished,
// }
//
// #[derive(PartialEq, Debug)]
// struct DownloadState {
//     downloaded_images: HashSet<String>
// }
//
// #[get("/status")]
// async fn get_status(data: web::Data<Mutex<AppState>>) -> impl Responder {
//     let app_state = data.lock().await;
//     match app_state.deref() {
//         ImageTaking(run_config, _) =>
//             {
//                 HttpResponse::Ok().json(
//                     json!({
//                         "phase": "Photoaufnahmeprozess",
//                         "status": match server_com::get_status(&run_config).await {
//                             Ok(status) => json!( status ),
//                             Err(err) => json!( err.to_string() )
//                         }
//                 }))
//             }
//         Start => {
//             HttpResponse::Ok().json(json!({
//                         "phase": "Konfigurationsmodus",
//                         "status": "Zurzeit läuft kein Prozess"}))
//         }
//         AppState::Photogrammetry => {
//             HttpResponse::Ok().json(json!({
//                         "phase": "Photogrammetrie",
//                         "status": ""}))
//         }
//         AppState::Finished => {
//             HttpResponse::Ok().json(json!({
//                         "phase": "Abgeschlossen",
//                         "status": "Das fertige 3D Modell kann jetzt heruntergeladen werden!"}))
//         }
//     }
// }
//
// #[post("/auftrag")]
// async fn post_configuration(config: web::Json<server_com::RunConfig>,
//                             data: web::Data<Mutex<AppState>>)
//                             -> impl Responder {
//     let app_state = data.lock().await;
//     let res = if app_state.deref() == &AppState::Start {
//         match server_com::post_run_config(&config.0).await {
//             Ok(_response) => { HttpResponse::Ok().body("Auftrag erfolgreich abgegeben!") }
//             Err(err) => {
//                 HttpResponse::InternalServerError()
//                     .body(format!("Auftrag konnte nicht abgegeben werden: {}", err.to_string()))
//             }
//         }
//     } else {
//         wrong_mode_response(app_state.deref())
//     };
//     res
// }
//
// fn wrong_mode_response(app_state: &AppState) -> HttpResponse<actix_web::body::Body> {
//     HttpResponse::InternalServerError()
//         .body(format!("Wrong mode! Currently in mode: {:?}", app_state.deref()))
// }
//
// #[delete("/auftrag")]
// async fn reset_app_state(data: web::Data<Mutex<AppState>>) -> impl Responder {
//     let mut app_state = data.lock().await;
//     clear_images();
//     *app_state = AppState::Start;
//     HttpResponse::Ok().body("Prozess kann nun von neuem begonnen werden.").await
// }
//
// #[get("/aufnahmen")]
// async fn get_ready_images(data: web::Data<Mutex<AppState>>) -> impl Responder {
//     let app_state = data.lock().await;
//     if let AppState::ImageTaking(run_config, download_state) = app_state.deref() {
//         match server_com::get_ready_image_list(run_config).await {
//             Ok(path_list) => {
//                 let available: HashSet<String> = path_list.iter().map(|str| str.to_string())
//                     .collect();
//                 let to_download: Vec<String> = available
//                     .difference(&download_state.downloaded_images)
//                     .map(|str| str.to_string()).collect();
//                 HttpResponse::Ok().json(to_download).await
//             }
//             Err(err) => {
//                 HttpResponse::InternalServerError().body(err.to_string()).await
//             }
//         }
//     } else {
//         wrong_mode_response(app_state.deref()).await
//     }
// }
//
// #[get("/aufnahme/{name}")]
// async fn get_and_save_aufnahme(data: web::Data<Mutex<AppState>>, path: web::Path<String>) -> impl Responder {
//     let mut app_state = data.lock().await;
//     if let AppState::ImageTaking(run_config, download_state) = app_state.deref_mut() {
//         let image_path = &path.0;
//         download_state.downloaded_images.insert(image_path.to_string());
//
//         match server_com::get_aufnahme(run_config, image_path).await {
//             Ok(img) => {
//                 if let Err(err) = save_image(image_path, &img).await {
//                     HttpResponse::InternalServerError().body(err.to_string()).await
//                 } else {
//                     HttpResponse::Ok()
//                         .set_header("Content-Type", "image/jpeg")
//                         .body(img.bytes().to_vec()).await
//                 }
//             }
//             Err(err) => {
//                 HttpResponse::InternalServerError().body(err.to_string()).await
//             }
//         }
//     } else {
//         wrong_mode_response(app_state.deref()).await
//     }
// }
//
// #[get("/")]
// async fn index_file () -> actix_web::Result<actix_files::NamedFile> {
//     Ok(actix_files::NamedFile::open("html/index.html")?)
// }
//
// #[get("/test")]
// async fn test () -> impl Responder {
//     let app_state = web_interface::app_state::Start{};
//     web_interface::app_state::Start::status()
// }

mod endpoints {
    use actix_web::{Responder, web, get, post, HttpRequest, HttpResponse};
    use crate::AppData;
    use crate::web_interface::model::{PageForm};
    use actix_web_actors::ws;
    use std::sync::Arc;

    #[get("/")]
    pub(crate) async fn index(data: web::Data<AppData>) -> impl Responder {
        let app_state = data.app_state.lock().unwrap();
        app_state.as_ref().unwrap().index()
    }

    #[get("/status")]
    pub(crate) async fn status(data: web::Data<AppData>) -> impl Responder {
        let app_state = data.app_state.lock().unwrap();
        app_state.as_ref().unwrap().status()
    }

    #[post("/page_form")]
    pub(crate) async fn post_page_form(page_form: web::Form<PageForm>, data: web::Data<AppData>) -> impl Responder {
        let mut app_state = data.app_state.lock().unwrap();
        let (new_app_state, res) = app_state.take().unwrap()
            .post_page_form(page_form.0);
        *app_state = Some(new_app_state);
        res
    }

    #[get("/media_content")]
    pub(crate) async fn get_media_content(data: web::Data<AppData>) -> impl Responder {
        let mut app_state = data.app_state.lock().unwrap();
        app_state.take().unwrap().get_resulting_content()
    }

    #[get("/ws_notification")]
    pub(crate) async fn ws_notification(req: HttpRequest, stream: web::Payload, data: web::Data<AppData>) -> impl Responder {
        let notifier = Arc::clone(&data.addr);
        let app_state = data.app_state.lock().unwrap();
        app_state.as_ref().unwrap().ws_notification(req, stream, notifier)
    }
}

struct AppData {
    app_state: Mutex<Option<Box<dyn AppState + Send>>>,
    addr: Arc<Mutex<Option<actix::Addr<MyWs>>>>,
}

#[actix_web::main]
async fn main() {
    let addr = Arc::new(Mutex::new(None));
    let addr2 = Arc::clone(&addr);
    let app_data = web::Data::new(AppData {
        app_state: Mutex::new(Some(Box::new(app_state::Start {}))),
        addr,
    });

    HttpServer::new(move || {
        App::new()
            .service(endpoints::index)
            .service(endpoints::status)
            .service(endpoints::post_page_form)
            .service(endpoints::get_media_content)
            .service(endpoints::ws_notification)
// .service(test)
// .service(post_configuration)
// .service(get_status)
// .service(index_file)
// .service(actix_files::Files::new("/", "html"))
            .app_data(app_data.clone())
    }).bind(SocketAddr::from_str("0.0.0.0:8080").unwrap())
        .unwrap()
        .run().await.unwrap();
}