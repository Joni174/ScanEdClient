use serde::{Deserialize, Serialize};
use crate::server_com::com_model::ServerStatus;
use std::error::Error;
use std::sync::Arc;
use actix::Addr;
use crate::web_interface::model::ws::MyWs;

pub type NotificationHandle = Arc<std::sync::Mutex<Option<Addr<MyWs>>>>;

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum PageForm {
    Auftrag(Auftrag),
    None,
}

#[derive(Deserialize, Clone)]
pub struct Auftrag {
    input_runde1: String,
    input_runde2: String,
    input_runde3: String,
    input_hostname: String,
}

impl Auftrag {
    pub fn into_vec(self) -> Result<Vec<i32>, Box<dyn Error>> {
        Ok(vec![
            self.input_runde1.parse::<i32>().map_err(|err| -> Box<dyn Error> {err.into()})?,
            self.input_runde2.parse::<i32>().map_err(|err| -> Box<dyn Error> {err.into()})?,
            self.input_runde3.parse::<i32>().map_err(|err| -> Box<dyn Error> {err.into()})?])
    }

    pub fn get_url(&self) -> &String {
        &self.input_hostname
    }
}

#[derive(Serialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum ImageAppStatus {
    Start,
    TakingImages(ServerStatus),
    Finished,
}

pub mod ws {
    use actix::{Actor, Message, StreamHandler, AsyncContext, Addr, Handler};
    use actix_web_actors::ws;
    use std::sync::{Arc, Mutex};

    #[derive(Message, Clone)]
    #[rtype(result = "()")]
    struct Msg(String);

    pub struct MyWs {
        addr: Arc<Mutex<Option<Addr<MyWs>>>>
    }

    impl MyWs {
        pub(crate) fn new(addr: Arc<Mutex<Option<Addr<MyWs>>>>) -> Self {
            MyWs { addr }
        }
    }

    impl Actor for MyWs {
        type Context = ws::WebsocketContext<Self>;
    }

    /// Handler for ws::Message message
    impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWs {
        fn handle(
            &mut self,
            msg: Result<ws::Message, ws::ProtocolError>,
            _ctx: &mut Self::Context,
        ) {
            match msg {
                Ok(ws::Message::Ping(msg)) => {
                    println!("ping: {:?}", &msg);
                    _ctx.pong(&*msg);
                }
                Ok(ws::Message::Text(text)) => {
                    println!("text: {:?}", text);
                }
                Ok(ws::Message::Binary(bin)) => {
                    println!("binary: {:?}", bin);
                }
                _ => (),
            }
        }
        fn started(&mut self, ctx: &mut Self::Context) {
            let mut addr = self.addr.lock().unwrap();
            *addr = Some(ctx.address())
        }
    }

    impl Handler<Notification> for MyWs {
        type Result = ();

        fn handle(&mut self, msg: Notification, ctx: &mut Self::Context) -> Self::Result {
            ctx.text(msg.0)
        }
    }

    #[derive(Message)]
    #[rtype(result = "()")]
    pub struct Notification(pub String);

    // #[derive(Clone)]
    // pub struct Notifier {
    //     addr: Option<Addr<MyWs>>
    // }
    //
    // impl Notifier {
    //     pub fn new() -> Notifier {
    //         Notifier{addr: None}
    //     }
    //
    //     pub fn get_addr(&self) -> &Option<Addr<MyWs>> {
    //         &self.addr
    //     }
    // }
    //
    // impl Actor for Notifier {
    //     type Context = Context<Self>;
    // }
    //
    // impl Handler<Notification> for Notifier {
    //     type Result = ();
    //
    //     fn handle(&mut self, msg: Notification, ctx: &mut Context<Self>) -> Self::Result {
    //         self.addr.as_ref().unwrap().send(msg);
    //     }
    // }
}