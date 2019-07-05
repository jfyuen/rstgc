extern crate slog;
extern crate slog_term;
use slog::{error, info, Logger};
use std::net::TcpStream;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

use error;
use message;

fn connect_server(
    logger: &Logger,
    addr: &Arc<String>,
    tx: &Sender<message::Message>,
    rx: &Receiver<message::Message>,
    is_remote: bool,
) -> error::Result<()> {
    loop {
        let stream = match TcpStream::connect(addr.as_ref()) {
            Ok(stream) => stream,
            Err(e) => {
                error!(logger, "could not connect: {}", e);
                thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
        };
        if is_remote {
            message::handle_remote_stream(logger, addr, tx, rx, stream)?;
        } else {
            message::handle_local_stream(logger, addr, tx, rx, stream)?;
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
        match connect_server(&logger1, &client_addr.clone(), &tx2, &rx1, false) {
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
        match connect_server(&logger2, &connect_addr.clone(), &tx1, &rx2, true) {
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
