use std::sync::{Arc, Mutex};
use crate::web_interface::model::ws::{MyWs, Notification};
use tokio::sync::oneshot::Receiver;
use std::process::Stdio;
use tokio::prelude::*;
use tokio::io::{BufReader};
use serde::Serialize;
use serde_json::json;
use actix::Addr;
use std::error::Error;
use std::fs::File;

#[derive(Serialize)]
#[serde(tag = "type")]
enum Message {
    NewConsoleOutput(MessageBody),
    Error(MessageBody),
    Finished,
}

#[derive(Serialize)]
struct MessageBody {
    body: String
}

impl From<String> for MessageBody {
    fn from(string: String) -> Self {
        MessageBody{body: string}
    }
}

pub async fn start_photogrammetry(ws: Arc<Mutex<Option<Addr<MyWs>>>>,
                              console_output: Arc<Mutex<Vec<String>>>,
                              mut shutdown_hook: Receiver<()>) -> Result<(), Box<dyn Error + Send>> {
    // clear_local_folders().await?;

    let mut c = tokio::process::Command::new("python3")
        .args(&["-u", "run.py", "ph"])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|err| -> Box<dyn Error + Send>{Box::new(err)})?;

    tokio::spawn(async move {
        let stdout = c.stdout.as_mut().unwrap();
        let mut buf_reader = BufReader::new(stdout);
        let mut buf = String::new();

        loop {
            if shutdown_hook.try_recv().is_ok() {
                // received shutdown message over channel
                c.kill().expect("unable to kill process");
                send_over_ws(&ws, &json!(Message::Error("User stopped Process".to_string().into())).to_string());
                return;
            }

            let mut finished = false;

            match buf_reader.read_line(&mut buf).await {
                Ok(size) if size == 0 => {
                    //finished
                    let to_send = json!(Message::Finished).to_string().to_string();
                    send_over_ws(&ws, &to_send);
                    finished = true;
                }
                Ok(_size) => {
                    // new console line
                    let to_send = json!(Message::NewConsoleOutput(buf.clone().into())).to_string();
                    send_over_ws(&ws, &to_send)
                }
                Err(err) => {
                    // Error
                    let to_send = json!(Message::Error(err.to_string().into())).to_string();
                    send_over_ws(&ws, &to_send);
                    finished = true;
                }
            }

            let mut console_output = console_output.lock().unwrap();
            console_output.push(buf.clone());
            buf.clear();
            if finished {break}
        }
    });
    Ok(())
}

// pub async fn clear_local_folders() -> Result<(), Box<dyn Error + Send>> {
//     let photogrammetry_dirs = vec![
//         PathBuf::from_str("texuring").unwrap(),
//         PathBuf::from_str("texuring").unwrap(),
//         PathBuf::from_str("texuring").unwrap(),
//     ];
//     for dir_path in photogrammetry_dirs {
//         if dir_path.exists() {
//             fs::remove_dir_all(dir_path).await
//                 .map_err(|err| -> Box<dyn Error + Send> {Box::new(err)})?;
//         }
//     }
//     Ok(())
// }

fn send_over_ws(ws: &Arc<Mutex<Option<Addr<MyWs>>>>, msg: &String) {
    if let Some(ws) = ws.lock().unwrap().as_mut() {
        ws.do_send(Notification(msg.clone()))
    }
}

fn zip_3d_model() {
    use zip::ZipWriter;
    use zip_extensions::write::ZipWriterExtensions;


    let file = File::create(archive_file).unwrap();
    let mut zip = ZipWriter::new(file);
    zip.create_from_directory(&source_path).unwrap()
}

