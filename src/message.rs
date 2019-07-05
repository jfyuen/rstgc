extern crate bincode;
extern crate serde;
extern crate slog;
extern crate slog_term;
use error;
use slog::error;
use slog::{info, Logger};
use std::io::Read;
use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

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

pub fn handle_local_stream(
    logger: &Logger,
    addr: &Arc<String>,
    tx: &Sender<Message>,
    rx: &Receiver<Message>,
    stream: TcpStream,
) -> error::Result<()> {
    let logger1 = logger.clone();

    let tx = tx.clone();
    let addr1 = addr.clone();

    let recv_stream = Box::new(stream.try_clone()?);
    let h = thread::spawn(move || {
        if let Err(e) = receive_data(recv_stream, &addr1, &logger1, &tx, false) {
            error!(logger1, "received error from on {}: {}", addr1, e);
        }
    });
    loop {
        let send_stream = Box::new(stream.try_clone()?);
        if let Err(e) = send_data(send_stream, &addr, &logger, &rx, false) {
            stream.shutdown(Shutdown::Both)?;
            error!(logger, "received error from on {}: {}", addr, e);
            h.join()?;
            break;
        }
    }
    Ok(())
}

pub fn handle_remote_stream(
    logger: &Logger,
    addr: &Arc<String>,
    tx: &Sender<Message>,
    rx: &Receiver<Message>,
    stream: TcpStream,
) -> error::Result<()> {
    let logger1 = logger.clone();
    let tx = tx.clone();
    let addr1 = addr.clone();

    let recv_stream = Box::new(stream.try_clone()?);
    let h = thread::spawn(move || {
        if let Err(e) = receive_data(recv_stream, &addr1, &logger1, &tx, true) {
            error!(logger1, "received error from on {}: {}", addr1, e);
        }
    });
    loop {
        let send_stream = Box::new(stream.try_clone()?);
        if let Err(e) = send_data(send_stream, &addr, &logger, &rx, true) {
            stream.shutdown(Shutdown::Both)?;
            error!(logger, "received error from on {}: {}", addr, e);
            h.join()?;
            break;
        }
    }
    Ok(())
}
