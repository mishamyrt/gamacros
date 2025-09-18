use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::thread;

use colored::Colorize;
use crossbeam_channel::Sender;

use crate::{print_error, print_info};

#[derive(Debug)]
pub(crate) enum ControlCommand {
    Rumble { id: Option<u32>, ms: u32 },
}

pub(crate) const SOCKET_FILE_NAME: &str = "control.sock";

pub const RUMBLE_COMMAND: &str = "rumble";

pub(crate) fn start_control_server<P: AsRef<Path>>(
    workspace_path: P,
    tx: Sender<ControlCommand>,
) -> std::io::Result<thread::JoinHandle<()>> {
    let socket_path = workspace_path.as_ref().join(SOCKET_FILE_NAME);
    if socket_path.exists() {
        let _ = fs::remove_file(&socket_path);
    }
    let listener = UnixListener::bind(&socket_path)?;
    print_info!("control socket listening at {}", socket_path.display());

    let handle = thread::Builder::new()
        .name("gamacrosd-control".into())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        handle_connection(stream, &tx);
                    }
                    Err(e) => {
                        print_error!("control socket accept error: {}", e);
                        break;
                    }
                }
            }
        })?;
    Ok(handle)
}

fn handle_connection(mut stream: UnixStream, tx: &Sender<ControlCommand>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => {
            let _ = stream.write_all(b"ERR empty\n");
        }
        Ok(_) => match parse_command(line.trim()) {
            Ok(cmd) => {
                let _ = tx.send(cmd);
                let _ = stream.write_all(b"OK\n");
            }
            Err(err) => {
                let _ = stream.write_all(format!("ERR {err}\n").as_bytes());
            }
        },
        Err(e) => {
            let _ = stream.write_all(format!("ERR read failed: {e}\n").as_bytes());
        }
    }
}

fn parse_command(s: &str) -> Result<ControlCommand, String> {
    let mut parts = s.split_whitespace();
    let Some(cmd) = parts.next() else {
        return Err("missing command".into());
    };
    match cmd {
        RUMBLE_COMMAND => {
            let mut ms: Option<u32> = None;
            let mut id: Option<u32> = None;
            for part in parts {
                let mut kv = part.splitn(2, '=');
                let key = kv.next().unwrap_or("");
                let value = kv.next().unwrap_or("");
                match key {
                    "ms" => {
                        ms = value.parse::<u32>().ok();
                    }
                    "id" => {
                        id = value.parse::<u32>().ok();
                    }
                    _ => {
                        print_error!(
                            "unknown argument: {}\nrumble supports ms and id",
                            key
                        );
                    }
                }
            }
            let ms = ms.ok_or_else(|| "missing ms".to_string())?;
            Ok(ControlCommand::Rumble { id, ms })
        }
        other => Err(format!("unknown command: {other}")),
    }
}
