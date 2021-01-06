use std::sync::{Arc};
use actix::{Addr};
use crate::web_interface::model::ws::{MyWs, Notification};
use tokio::sync::{oneshot, Mutex};
use tokio::process::Command;
use std::process::Stdio;
use tokio::io::{BufReader, AsyncBufReadExt};
use std::fs::File;
use tokio::sync::oneshot::error::TryRecvError;
use serde_json::json;
use serde::{Serialize};
use log::{info, warn};
use std::error::Error;
use crate::photogrammetry::paths;

#[derive(Serialize)]
#[serde(tag = "type")]
enum Message {
    NewConsoleOutput(MessageBody),
    Error(MessageBody),
    Finished,
}

impl Message {
    fn into_json_string(self) -> String {
        json!(self).to_string()
    }
}

#[derive(Serialize)]
struct MessageBody {
    body: String
}

impl From<String> for MessageBody {
    fn from(string: String) -> Self {
        MessageBody { body: string }
    }
}

impl From<&str> for MessageBody {
    fn from(str: &str) -> Self {
        MessageBody { body: str.to_string() }
    }
}

pub async fn start_photogrammetry(ws: Arc<std::sync::Mutex<Option<Addr<MyWs>>>>,
                                  console_output: Arc<Mutex<Vec<String>>>,
                                  mut shutdown_hook: oneshot::Receiver<()>) -> Result<(), Box<dyn Error + Send>> {
    // clear_local_folders().await?;
    let mut cmd = Command::new("python3");

    cmd.args(&["-u", "run.py", "ph"]);
    cmd.stdout(Stdio::piped());

    let mut child = cmd.spawn()
        .expect("failed to spawn command");

    let stdout = child.stdout.take()
        .expect("child did not have a handle to stdout");

    let mut reader = BufReader::new(stdout).lines();
    let (shutdown_process_tx, shutdown_process_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        tokio::select! {
            _ = child => {
                info!("photogrammetry process finished");
            }
            _ = shutdown_process_rx => {
                warn!("user canceled photogrammetry process");
            }
        }
    });

    tokio::spawn(async move {
        while let Some(line) = match reader.next_line().await {
            Ok(maybe_line) => { maybe_line }
            Err(err) => { panic!(err) }
        } {
            println!("process output: {}", line);
            console_output.lock().await.push(line.clone());
            send_over_ws(Arc::clone(&ws), &Message::NewConsoleOutput(
                line.into()
            ).into_json_string()).await;

            if shutdown_hook.try_recv() == Err(TryRecvError::Empty) {
                // continue
            } else {
                info!("stopped photogrammetry process early");
                send_over_ws(Arc::clone(&ws), &Message::Error("user stopped photogrammetry process".into()).into_json_string()).await;
                if let Err(_err) = shutdown_process_tx.send(()) {
                    warn!("photogrammetry process already dead");
                }
                break;
            }
        }
        send_over_ws(Arc::clone(&ws), &Message::Finished.into_json_string()).await;

        tokio::task::spawn_blocking(zip_3d_model);
    });

    Ok(())
}

async fn send_over_ws(ws: Arc<std::sync::Mutex<Option<Addr<MyWs>>>>, msg: &str) {
    let msg = msg.to_string();
    tokio::task::spawn_blocking(move || {
        if let Some(ws) = ws.lock().unwrap().as_mut() {
            ws.do_send(Notification(msg.to_string()))
        }
    });
}

fn zip_3d_model() {
    use zip::ZipWriter;
    use zip_extensions::write::ZipWriterExtensions;

    let file = File::create(paths::archive_file()).unwrap();
    let mut zip = ZipWriter::new(file);
    zip.create_from_directory(&paths::texture_folder()).unwrap()
}


