use serde::{Deserialize, Serialize};
use crate::server_com::Status;

#[derive(Deserialize)]
pub struct Auftrag {
    input_runde1: String,
    input_runde2: String,
    input_runde3: String,
    input_hostname: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum PageForm {
    Auftrag(Auftrag),
    None,
}

impl Auftrag {
    pub fn into_vec(self) -> Vec<i32> {
        vec![self.input_runde1.parse::<i32>().unwrap(),
             self.input_runde2.parse::<i32>().unwrap(),
             self.input_runde3.parse::<i32>().unwrap()]
    }

    pub fn get_url(&self) -> &String {
        &self.input_hostname
    }
}

#[derive(Serialize, Clone)]
pub enum ImageTakingStatus {
    Start,
    TakingImages(Status),
    Finished,
}

pub mod ws {
    use actix::{Actor, Message, StreamHandler, AsyncContext, Context, Addr, ActorContext, Handler, WrapFuture};
    use actix_web_actors::ws;
    use serde_json::json;
    use serde::Serialize;
    use crossbeam_channel::Receiver;
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