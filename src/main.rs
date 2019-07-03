extern crate clap;
extern crate slog;
extern crate slog_term;
extern crate bincode;
#[macro_use]
extern crate serde_derive;
extern crate serde;
use clap::App;
use clap::Arg;
use clap::SubCommand;
use slog::{debug, error, info, Drain, Logger};
use std::io::Read;
use std::io::Write;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

mod error;

#[derive(Serialize, Deserialize)]
struct Message {
    content: Vec<u8>,
    eof: bool,
}

fn receive_local_data(
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

fn receive_remote_data(
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

fn send_remote_data(
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


fn listen_remote_client(
    logger: &Logger,
    addr: &Arc<String>,
    rx: &Receiver<Message>,
    tx: &Sender<Message>
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
            if let Err(e) = receive_remote_data(recv_stream, &addr1, &logger1, &tx) {
                error!(logger1, "received error from on {}: {}", addr1, e);
            }
        });
        loop {
            let send_stream = Box::new(stream.try_clone()?);
            if let Err(e) = send_remote_data(send_stream, &addr, &logger, &rx) {
                stream.shutdown(Shutdown::Both)?;
                error!(logger, "received error from on {}: {}", addr, e);
                h.join()?;
                break;
            }
        }
    }
    Ok(())
}


fn send_local_data(
    mut writer: Box<std::io::Write>,
    addr: &str,
    logger: &Logger,
    rx: &Receiver<Message>
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

fn listen_local_client(
    logger: &Logger,
    addr: &Arc<String>,
    tx: &Sender<Message>,
    rx: &Receiver<Message>,
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
            if let Err(e) = receive_local_data(recv_stream, &addr1, &logger1, &tx) {
                error!(logger1, "received error from on {}: {}", addr1, e);
            }
        });
        loop {
            let send_stream = Box::new(stream.try_clone()?);
            if let Err(e) = send_local_data(send_stream, &addr, &logger, &rx) {
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

fn run_listen_server(logger: &Logger, client_addr: &str, connect_addr: &str) -> error::Result<()> {
    let (tx1, rx1): (Sender<Message>, Receiver<Message>) = mpsc::channel();
    let (tx2, rx2): (Sender<Message>, Receiver<Message>) = mpsc::channel();
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

fn connect_remote_server(
    logger: &Logger,
    addr: &Arc<String>,
    tx: &Sender<Message>,
    rx: &Receiver<Message>,
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
            if let Err(e) = receive_remote_data(recv_stream, &addr1, &logger1, &tx) {
                error!(logger1, "received error from on {}: {}", addr1, e);
            }
        });
        loop {
            let send_stream = Box::new(stream.try_clone()?);
            if let Err(e) = send_remote_data(send_stream, &addr, &logger, &rx) {
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
    rx: &Receiver<Message>,
    tx: &Sender<Message>,
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
            if let Err(e) = receive_local_data(recv_stream, &addr1, &logger1, &tx) {
                error!(logger1, "received error from on {}: {}", addr1, e);
            }
        });
        loop {
            let send_stream = Box::new(stream.try_clone()?);
            if let Err(e) = send_local_data(send_stream, &addr, &logger, &rx) {
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

fn run_connect_client(logger: &Logger, client_addr: &str, connect_addr: &str) -> error::Result<()> {
    let (tx1, rx1): (Sender<Message>, Receiver<Message>) = mpsc::channel();
    let (tx2, rx2): (Sender<Message>, Receiver<Message>) = mpsc::channel();
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

fn main() {
    let matches = App::new("TGC")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Jean-François YUEN <jfyuen@gmail.com>")
        .about("TCP Gender Changer, rust version")
        .subcommand(SubCommand::with_name("-C")
            .arg(Arg::with_name("local-server").short("s").help("the host and port of the local server").takes_value(true))
            .arg(Arg::with_name("listen-server").short("c").help("the host and port of the Listen/Listen server").takes_value(true))
            .arg(Arg::with_name("interval").short("i").help("interval when (re)connecting to either host in seconds, must be positive").takes_value(true))
            .arg(Arg::with_name("reconnect").short("r").help("automatically reconnect to the local server, if not specified a connection is made only when data are received from the remote node").takes_value(true))
            .arg(Arg::with_name("debug").short("d").help("activate debug logs"))
        )
        .subcommand(SubCommand::with_name("-L")
            .arg(Arg::with_name("client-port").short("p").help("the port to listen on for actual client connection").takes_value(true))
            .arg(Arg::with_name("connect-port").short("q").help("the port to listen on for connection from the other Connect/Connect node").takes_value(true))
            .arg(Arg::with_name("debug").short("d").help("activate debug logs"))
        )
        .get_matches();

    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let logger = Logger::root(slog_term::FullFormat::new(plain).build().fuse(), slog::o!());

    if let Some(matches) = matches.subcommand_matches("-L") {
        let p = matches.value_of("client-port").unwrap();
        let q = matches.value_of("connect-port").unwrap();
        run_listen_server(&logger, &p, &q).unwrap();
    } else if let Some(matches) = matches.subcommand_matches("-C") {
        let s = matches.value_of("local-server").unwrap();
        let c = matches.value_of("listen-server").unwrap();
        run_connect_client(&logger, &s, &c).unwrap();
    }
}
