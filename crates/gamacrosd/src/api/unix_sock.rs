use std::fs;
use std::io::{BufWriter, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};

use crossbeam_channel::Sender;
use colored::Colorize;
use bitcode::{Encode, Decode};

use crate::{print_error, print_info};
use super::{Command, ApiTransport, ApiResult};

const SOCKET_FILE_NAME: &str = "api.sock";

#[derive(Encode, Decode)]
pub struct SocketCommand {
    command: Command,
}

pub struct UnixSocket {
    socket_path: PathBuf,
}

impl UnixSocket {
    pub fn new<P: AsRef<Path>>(workspace_path: P) -> Self {
        let socket_path = workspace_path.as_ref().join(SOCKET_FILE_NAME);

        Self { socket_path }
    }
}

impl UnixSocket {
    fn handle_connection(mut stream: UnixStream, tx: &Sender<Command>) {
        let mut length_buffer = [0u8; 4];
        let _ = stream.read_exact(&mut length_buffer);
        if length_buffer == [0u8; 4] {
            let _ = stream.write_all(b"ERR empty\n");
            return;
        }

        let length = u32::from_be_bytes(length_buffer) as usize;
        if length == 0 {
            let _ = stream.write_all(b"ERR empty\n");
            return;
        }

        // Читаем данные
        let mut data_buffer = vec![0u8; length];
        let Ok(_) = stream.read_exact(&mut data_buffer) else {
            let _ = stream.write_all(b"ERR read failed\n");
            return;
        };

        // Десериализуем
        let command = match bitcode::decode(&data_buffer) {
            Ok(cmd) => cmd,
            Err(err) => {
                print_error!("failed to decode command: {err}");
                let _ = stream.write_all(format!("ERR {err}\n").as_bytes());
                return;
            }
        };

        tx.send(command).unwrap();
    }
}

impl ApiTransport for UnixSocket {
    fn listen_events(&self, tx: Sender<Command>) -> ApiResult<JoinHandle<()>> {
        let socket_path = self.socket_path.clone();
        if socket_path.exists() {
            fs::remove_file(&socket_path)?;
        }
        let listener = UnixListener::bind(&socket_path)?;
        print_info!("unix socket api listening at {}", socket_path.display());

        let handle = thread::Builder::new()
            .name("gamacrosd-socket-api".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    match stream {
                        Ok(stream) => {
                            Self::handle_connection(stream, &tx);
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

    fn send_event(&self, event: Command) -> ApiResult<()> {
        let socket_path = self.socket_path.clone();
        let stream = UnixStream::connect(&socket_path)?;
        let mut writer = BufWriter::new(stream);
        let cmd = SocketCommand { command: event };
        let encoded = bitcode::encode(&cmd);
        let length = encoded.len() as u32;
        writer.write_all(&length.to_be_bytes())?;
        writer.write_all(&encoded)?;

        Ok(())
    }
}
