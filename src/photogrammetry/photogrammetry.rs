use std::sync::{Arc};
use crate::web_interface::model::ws::{Notification};
use tokio::sync::{oneshot, Mutex};
use tokio::process::{Command, ChildStdout, ChildStderr};
use std::process::Stdio;
use tokio::io::{BufReader, AsyncBufReadExt, Lines};
use std::fs::File;
use serde_json::json;
use serde::{Serialize};
use log::{info, warn, error, debug};
use crate::photogrammetry::paths;
use crate::web_interface::model::NotificationHandle;
use tokio::io;
use crate::photogrammetry::photogrammetry::Message::{NewConsoleOutput, Finished};
use tokio::task::JoinHandle;
use std::ops::Deref;

pub type ConsoleOutput = Arc<Mutex<Vec<serde_json::Value>>>;

#[derive(Serialize, Clone)]
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

#[derive(Serialize, Clone)]
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

struct ConsoleReader {
    stdout_reader: Lines<BufReader<ChildStdout>>,
    stderr_reader: Lines<BufReader<ChildStderr>>,
}

impl ConsoleReader {
    fn new(stdout: ChildStdout, stderr: ChildStderr) -> ConsoleReader {
        let stdout_reader = BufReader::new(stdout).lines();
        let stderr_reader = BufReader::new(stderr).lines();
        ConsoleReader { stdout_reader, stderr_reader }
    }

    async fn next_line(&mut self) -> io::Result<Option<String>> {
        tokio::select! {
            stdout_res = self.stdout_reader.next_line() => {
                stdout_res
            },
            stderr_res = self.stderr_reader.next_line() => {
                stderr_res
            }
        }
    }
}

async fn start_process(
    shutdown_hook: oneshot::Receiver<()>
) -> (ConsoleReader, JoinHandle<()>) {
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

    let join_handle = tokio::spawn(async move {
        tokio::select! {
            _ = child => {
                info!("photogrammetry process finished");
            }
            _ = shutdown_hook => {
                warn!("user canceled photogrammetry process");
            }
        }
    });

    (ConsoleReader::new(stdout, stderr), join_handle)
}

pub async fn start_photogrammetry(ws: NotificationHandle,
                                  console_output: ConsoleOutput,
                                  shutdown_process_rx: oneshot::Receiver<()>) -> JoinHandle<()> {
    tokio::spawn(async move {
    let (mut console_reader, process_join_handle) =
        start_process(shutdown_process_rx).await;
    loop {
        let line = match console_reader.next_line().await {
            Ok(line) => { line }
            Err(err) => {
                error!("Error occurred when reading line from console: {}", err);
                break;
            }
        };

        match line {
            None => {
                // console pipe closed
                break;
            }
            Some(line) => {
                let new_console_output = NewConsoleOutput(line.into());
                debug!("console-line: {}", new_console_output.clone().into_json());
                send_over_ws(ws.clone(), &new_console_output.clone().into_json()).await;
                console_output.lock().await.push(json!(new_console_output));
            }
        }
    }
    send_over_ws(ws.clone(), &Finished.into_json()).await;
    console_output.lock().await.push(json!(Finished));

    tokio::spawn(async move {
        if let Err(err) = process_join_handle.await {
            let error_msg = format!("OpenDroneMap Process exited with error: {}", err);
            error!("{}", error_msg);
            send_over_ws(ws.clone(), &Message::Error(error_msg.into()).into_json()).await;
        }
        tokio::task::spawn_blocking(zip_3d_model)
    });
    })
}

async fn send_over_ws(ws: NotificationHandle, msg: &str) {
    let msg = msg.to_string();
    tokio::task::spawn_blocking(move || {
        let notification_handle = ws.lock().unwrap();
        if let Some(ws) = notification_handle.deref() {
            ws.do_send(Notification(msg))
        } else {
            warn!("no websocket addr available");
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