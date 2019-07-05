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

pub fn send_local_data(
    mut writer: Box<std::io::Write>,
    addr: &str,
    logger: &Logger,
    rx: &Receiver<Message>,
) -> error::Result<()> {
    loop {
        let msg = rx.recv()?;
        info!(
            logger,
            "from channel {}: {}",
            addr,
            String::from_utf8_lossy(&msg.content)
        );
        let n = writer.write(&msg.content[..])?;
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

pub fn receive_local_data(
    mut reader: Box<std::io::Read>,
    addr: &str,
    logger: &Logger,
    tx: &Sender<Message>,
) -> error::Result<()> {
    let mut buffer = [0u8; 4096];
    loop {
        let n = reader.read(&mut buffer)?;
        let eof = n == 0;
        let content = &buffer[..n];
        let msg = Message {
            content: content.to_vec(),
            eof,
        };
        tx.send(msg).unwrap();
        info!(
            logger,
            "received from network on {}: {}",
            addr,
            String::from_utf8_lossy(content)
        );
        if eof {
            break;
        }
    }
    Ok(())
}

pub fn receive_remote_data(
    mut reader: Box<std::io::Read>,
    addr: &str,
    logger: &Logger,
    tx: &Sender<Message>,
) -> error::Result<()> {
    let mut buffer = [0u8; 4096];
    loop {
        let n = reader.read(&mut buffer)?;
        let eof = n == 0;
        let msg: Message = bincode::deserialize(&buffer[..n])?;
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

pub fn send_remote_data(
    mut writer: Box<std::io::Write>,
    addr: &str,
    logger: &Logger,
    rx: &Receiver<Message>,
) -> error::Result<()> {
    loop {
        let msg = rx.recv()?;
        info!(
            logger,
            "from channel {}: {}",
            addr,
            String::from_utf8_lossy(&msg.content)
        );
        let serialized = bincode::serialize(&msg)?;
        let n = writer.write(&serialized)?;
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
