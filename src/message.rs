extern crate bincode;
extern crate serde;
extern crate slog;
extern crate slog_term;
use slog::{info, Logger};
use std::io::Read;
use std::io::Write;
use std::sync::mpsc::{Receiver, Sender};

use error;

#[derive(Serialize, Deserialize)]
pub struct Message {
    content: Vec<u8>,
    eof: bool,
}

pub fn send_data(
    mut writer: Box<std::io::Write>,
    addr: &str,
    logger: &Logger,
    rx: &Receiver<Message>,
    is_serialized: bool,
) -> error::Result<()> {
    loop {
        let msg = rx.recv()?;
        info!(
            logger,
            "from channel {}: {}",
            addr,
            String::from_utf8_lossy(&msg.content)
        );
        let data = if is_serialized {
            bincode::serialize(&msg)?
        } else {
            msg.content
        };
        let n = writer.write(&data)?;
        info!(logger, "wrote to stream {}: {} bytes", addr, n);
        writer.flush()?;
        if msg.eof {
            info!(logger, "received eof from message {}", addr);
            break;
        }
    }
    info!(logger, "leaving send data loop {}", addr);
    Ok(())
}

pub fn receive_data(
    mut reader: Box<std::io::Read>,
    addr: &str,
    logger: &Logger,
    tx: &Sender<Message>,
    is_serialized: bool,
) -> error::Result<()> {
    let mut buffer = [0u8; 4096];
    loop {
        let n = reader.read(&mut buffer)?;
        let eof = n == 0;
        let msg: Message = if is_serialized {
            bincode::deserialize(&buffer[..n])?
        } else {
            Message {
                content: buffer[..n].to_vec(),
                eof,
            }
        };
        info!(
            logger,
            "received from network on {}: {}",
            addr,
            String::from_utf8_lossy(&msg.content)
        );
        tx.send(msg).unwrap();
        if eof {
            break;
        }
    }
    Ok(())
}
