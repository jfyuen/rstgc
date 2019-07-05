extern crate slog;
extern crate slog_term;
use slog::{debug, error, info, Logger};
use std::net::TcpListener;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

use error;
use message;

fn listen_client(
    logger: &Logger,
    addr: &Arc<String>,
    rx: &Receiver<message::Message>,
    tx: &Sender<message::Message>,
    is_remote: bool,
) -> error::Result<()> {
    let listener = TcpListener::bind(addr.as_ref())?;
    debug!(logger, "listening on {}", &addr);

    for stream in listener.incoming() {
        let mut stream = stream?;
        if is_remote {
            message::handle_remote_stream(logger, addr, tx, rx, stream)?;
        } else {
            message::handle_local_stream(logger, addr, tx, rx, stream)?;
            info!(logger, "reconnecting");
        }
    }
    Ok(())
}

pub fn run_listen_server(
    logger: &Logger,
    client_addr: &str,
    connect_addr: &str,
) -> error::Result<()> {
    let (tx1, rx1): (Sender<message::Message>, Receiver<message::Message>) = mpsc::channel();
    let (tx2, rx2): (Sender<message::Message>, Receiver<message::Message>) = mpsc::channel();
    let connect_addr = Arc::new(String::from(connect_addr));
    let logger1 = logger.clone();
    let handle1 = thread::spawn(move || loop {
        match listen_client(&logger1, &connect_addr.clone(), &rx1, &tx2, true) {
            Ok(_) => {
                info!(logger1, "ok {}", connect_addr);
            }
            Err(e) => {
                error!(logger1, "received connection error: {} {}", connect_addr, e);
            }
        }
    });
    let client_addr = Arc::new(String::from(client_addr));

    let logger2 = logger.clone();
    let handle2 = thread::spawn(move || loop {
        match listen_client(&logger2, &client_addr.clone(), &rx2, &tx1, false) {
            Ok(_) => {
                info!(logger2, "ok {}", client_addr);
            }
            Err(e) => {
                error!(logger2, "received connection error {}: {}", client_addr, e);
            }
        }
    });
    handle1.join()?;
    handle2.join()?;
    Ok(())
}
