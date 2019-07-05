extern crate slog;
extern crate slog_term;
use slog::{debug, error, info, Logger};
use std::net::{Shutdown, TcpListener};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

use error;
use message;

fn listen_remote_client(
    logger: &Logger,
    addr: &Arc<String>,
    rx: &Receiver<message::Message>,
    tx: &Sender<message::Message>,
) -> error::Result<()> {
    let listener = TcpListener::bind(addr.as_ref())?;
    debug!(logger, "listening on {}", &addr);

    for stream in listener.incoming() {
        let logger1 = logger.clone();
        let tx = tx.clone();
        let mut stream = stream?;
        let addr1 = addr.clone();

        let recv_stream = Box::new(stream.try_clone()?);
        let h = thread::spawn(move || {
            if let Err(e) = message::receive_remote_data(recv_stream, &addr1, &logger1, &tx) {
                error!(logger1, "received error from on {}: {}", addr1, e);
            }
        });
        loop {
            let send_stream = Box::new(stream.try_clone()?);
            if let Err(e) = message::send_remote_data(send_stream, &addr, &logger, &rx) {
                stream.shutdown(Shutdown::Both)?;
                error!(logger, "received error from on {}: {}", addr, e);
                h.join()?;
                break;
            }
        }
    }
    Ok(())
}

fn listen_local_client(
    logger: &Logger,
    addr: &Arc<String>,
    tx: &Sender<message::Message>,
    rx: &Receiver<message::Message>,
) -> error::Result<()> {
    let listener = TcpListener::bind(addr.as_ref())?;
    debug!(logger, "listening started, ready to accept");

    for stream in listener.incoming() {
        let logger1 = logger.clone();
        let mut stream = stream?;
        let tx = tx.clone();
        let addr1 = addr.clone();

        let recv_stream = Box::new(stream.try_clone()?);
        let h = thread::spawn(move || {
            if let Err(e) = message::receive_local_data(recv_stream, &addr1, &logger1, &tx) {
                error!(logger1, "received error from on {}: {}", addr1, e);
            }
        });
        loop {
            let send_stream = Box::new(stream.try_clone()?);
            if let Err(e) = message::send_local_data(send_stream, &addr, &logger, &rx) {
                stream.shutdown(Shutdown::Both)?;
                error!(logger, "received error from on {}: {}", addr, e);
                h.join()?;
                break;
            }
        }
        info!(logger, "reconnecting");
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
        match listen_remote_client(&logger1, &connect_addr.clone(), &rx1, &tx2) {
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
        match listen_local_client(&logger2, &client_addr.clone(), &tx1, &rx2) {
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
