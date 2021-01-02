use std::sync::{Arc, Mutex};
use crate::web_interface::model::ws::{MyWs, Notification};
use tokio::sync::oneshot::Receiver;
use std::process::Stdio;
use tokio::prelude::*;
use tokio::io::{BufReader, Error};
use serde::Serialize;
use serde_json::json;
use std::ops::Deref;
use actix::Addr;

#[derive(Serialize)]
#[serde(tag = "type")]
enum Message {
    NewConsoleOutput(String),
    Error(String),
    Finished,
}

pub async fn start_photogrammetry(ws: Arc<Mutex<Option<Addr<MyWs>>>>,
                              console_output: Arc<Mutex<String>>,
                              mut shutdown_hook: Receiver<()>) -> Result<(), tokio::io::Error> {
    let mut c = tokio::process::Command::new("python3 run.py")
        .stdout(Stdio::piped())
        .spawn()?;
    tokio::spawn(async move {
        let stdout = c.stdout.as_mut().unwrap();
        let mut buf_reader = BufReader::new(stdout);
        let mut buf = String::new();

        loop {
            if shutdown_hook.try_recv().is_ok() {
                // received shutdown message over channel
                send_over_ws(&ws, &json!(Message::Error("User stopped Process".to_string())).to_string())
            }
            match buf_reader.read_line(&mut buf).await {
                Ok(size) if size == 0 => {
                    //finished
                    send_over_ws(&ws, &json!(Message::Finished).to_string().to_string())
                }
                Ok(size) => {
                    // new console line
                    send_over_ws(&ws, &json!(Message::NewConsoleOutput(buf.clone())).to_string())
                }
                Err(err) => {
                    // Error
                    send_over_ws(&ws, &json!(Message::Error(err.to_string())).to_string())
                }
            }
            let mut console_output = console_output.lock().unwrap();
            console_output.push_str(&buf);
            console_output.push_str("\n");
        }
    });
    Ok(())
}

fn send_over_ws(ws: &Arc<Mutex<Option<Addr<MyWs>>>>, msg: &String) {
    if let Some(ws) = ws.lock().unwrap().as_mut() {
        ws.do_send(Notification(msg.clone()))
    }
}

