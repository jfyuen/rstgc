extern crate slog;
extern crate slog_term;
use slog::{error, info, Logger};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

use error;
use message;

fn connect_remote_server(
    logger: &Logger,
    addr: &Arc<String>,
    tx: &Sender<message::Message>,
    rx: &Receiver<message::Message>,
) -> error::Result<()> {
    loop {
        let stream = match TcpStream::connect(addr.as_ref()) {
            Ok(stream) => Box::new(stream),
            Err(e) => {
                error!(logger, "could not connect: {}", e);
                thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
        };
        let logger1 = logger.clone();
        let tx = tx.clone();
        let addr1 = addr.clone();

        let recv_stream = Box::new(stream.try_clone()?);
        let h = thread::spawn(move || {
            if let Err(e) = message::receive_data(recv_stream, &addr1, &logger1, &tx, true) {
                error!(logger1, "received error from on {}: {}", addr1, e);
            }
        });
        loop {
            let send_stream = Box::new(stream.try_clone()?);
            if let Err(e) = message::send_data(send_stream, &addr, &logger, &rx, true) {
                stream.shutdown(Shutdown::Both)?;
                error!(logger, "received error from on {}: {}", addr, e);
                h.join()?;
                // return Err(e);
                break;
            }
        }
    }
}

fn connect_local_server(
    logger: &Logger,
    addr: &Arc<String>,
    rx: &Receiver<message::Message>,
    tx: &Sender<message::Message>,
) -> error::Result<()> {
    loop {
        let stream = match TcpStream::connect(addr.as_ref()) {
            Ok(stream) => Box::new(stream),
            Err(e) => {
                error!(logger, "could not connect: {}", e);
                thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
        };
        let logger1 = logger.clone();
        let tx = tx.clone();
        let addr1 = addr.clone();

        let recv_stream = Box::new(stream.try_clone()?);
        let h = thread::spawn(move || {
            if let Err(e) = message::receive_data(recv_stream, &addr1, &logger1, &tx, false) {
                error!(logger1, "received error from on {}: {}", addr1, e);
            }
        });
        loop {
            let send_stream = Box::new(stream.try_clone()?);
            if let Err(e) = message::send_data(send_stream, &addr, &logger, &rx, false) {
                stream.shutdown(Shutdown::Both)?;
                // stream.shutdown(Shutdown::Write)?;
                error!(logger, "received error from on {}: {}", addr, e);
                h.join()?;
                // return Err(e);
                break;
            }
        }
    }
}

pub fn run_connect_client(
    logger: &Logger,
    client_addr: &str,
    connect_addr: &str,
) -> error::Result<()> {
    let (tx1, rx1): (Sender<message::Message>, Receiver<message::Message>) = mpsc::channel();
    let (tx2, rx2): (Sender<message::Message>, Receiver<message::Message>) = mpsc::channel();
    let client_addr = Arc::new(String::from(client_addr));
    let logger1 = logger.clone();
    let handle1 = thread::spawn(move || loop {
        match connect_local_server(&logger1, &client_addr.clone(), &rx1, &tx2) {
            Ok(_) => {
                info!(logger1, "ok {}", client_addr);
            }
            Err(e) => {
                error!(logger1, "received connection error: {} {}", client_addr, e);
            }
        }
    });
    let connect_addr = Arc::new(String::from(connect_addr));

    let logger2 = logger.clone();
    let handle2 = thread::spawn(move || loop {
        match connect_remote_server(&logger2, &connect_addr.clone(), &tx1, &rx2) {
            Ok(_) => {
                info!(logger2, "ok {}", connect_addr);
            }
            Err(e) => {
                error!(logger2, "received connection error {}: {}", connect_addr, e);
            }
        }
    });
    handle1.join()?;
    handle2.join()?;
    Ok(())
}
