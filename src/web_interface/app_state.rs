use actix_web::{Responder, HttpResponse};
use actix_web::body::Body;

trait AppState {
    fn index() -> HttpResponse;
    fn status() -> HttpResponse;
    fn post_auftrag(auftrag: Auftrag) -> HttpResponse;
    fn get_aufnahmen(&self) -> HttpResponse;
    fn get_aufnahme(&self, auftrag_name: &str) -> HttpResponse;
    fn get_3d_model() -> HttpResponse;
    fn upgrade(&self) -> Option<Self>;
}

struct Start {}

impl AppState for Start {
    fn index() -> HttpResponse {
        unimplemented!()
    }

    fn status() -> HttpResponse {
        unimplemented!()
    }

    fn post_auftrag(auftrag: _) -> HttpResponse {
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

    fn upgrade(&self) -> Option<Self> {
        unimplemented!()
    }
}


