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
use crate::web_interface::model::NotificationHandle;

pub type ConsoleOutput = Arc<Mutex<Vec<String>>>;

#[derive(Serialize)]
#[serde(tag = "type")]
enum Message {
    NewConsoleOutput(MessageBody),
    Error(MessageBody),
    Finished,
}

impl Message {
    fn into_json(self) -> String {
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

pub async fn start_photogrammetry(ws: NotificationHandle,
                                  console_output: ConsoleOutput,
                                  mut shutdown_hook: oneshot::Receiver<()>) -> Result<(), Box<dyn Error + Send>> {
    // clear_local_folders().await?;
    let mut cmd = Command::new("python3");

    cmd.args(&["-u", "run.py", "ph"]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());


    let mut child = cmd.spawn()
        .expect("failed to spawn command");

    let stdout = child.stdout.take()
        .expect("child did not have a handle to stdout");

    let stderr = child.stderr.take()
        .expect("child did not have a handle to stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();
    let (mut shutdown_process_tx, shutdown_process_rx) = oneshot::channel::<()>();
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
        loop {
            if !handle_potential_config_line(stdout_reader
                                                 .next_line()
                                                 .await
                                                 .expect("unable to get next console line (stdout)"),
                                             &ws,
                                             &console_output,
                                             &mut shutdown_process_tx).await
                ||
                !handle_potential_config_line(stderr_reader
                                                  .next_line()
                                                  .await
                                                  .expect("unable to get next console line (stderr)"),
                                              &ws,
                                              &console_output,
                                              &mut shutdown_process_tx).await {
                break;
            }

            if test_for_shutdown_message(&mut shutdown_hook) {
                shutdown(&ws, &mut shutdown_process_tx).await;
                break;
            }
        }
        send_over_ws(Arc::clone(&ws), &Message::Finished.into_json()).await;

        tokio::task::spawn_blocking(zip_3d_model);
    });

    Ok(())
}

async fn handle_potential_config_line(potential_line: Option<String>,
                                      ws: &NotificationHandle,
                                      console_output: &ConsoleOutput,
                                      shutdown_process_tx: &mut oneshot::Sender<()>) -> bool {
    if let Some(line) = potential_line {
        send_console_line(&ws, &console_output, &line).await;
        true
    } else {
        shutdown(&ws, shutdown_process_tx).await;
        false
    }
}

async fn send_console_line(ws: &NotificationHandle, console_output: &ConsoleOutput, line: &str) {
    println!("process output: {}", line);
    console_output.lock().await.push(line.to_string());
    send_over_ws(Arc::clone(&ws), &Message::NewConsoleOutput(line.into())
        .into_json()).await;
}

fn test_for_shutdown_message(shut_rx: &mut oneshot::Receiver<()>) -> bool {
    shut_rx.try_recv() == Err(TryRecvError::Empty)
}

async fn shutdown(ws: &NotificationHandle, shutdown_odm: &mut oneshot::Sender<()>) {
    info!("stopped photogrammetry process early");
    send_over_ws(ws.clone(), &Message::Error("user stopped photogrammetry process".into())
        .into_json())
        .await;
    if let Err(_err) = shutdown_odm.send(()) {
        warn!("photogrammetry process already dead");
    }
}

async fn send_over_ws(ws: NotificationHandle, msg: &str) {
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


