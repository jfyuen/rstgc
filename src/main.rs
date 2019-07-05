extern crate bincode;
extern crate clap;
extern crate slog;
extern crate slog_term;
#[macro_use]
extern crate serde_derive;
extern crate serde;
use clap::App;
use clap::Arg;
use clap::SubCommand;
use slog::{Drain, Logger};

mod client;
mod error;
mod message;
mod server;

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
        server::run_listen_server(&logger, &p, &q).unwrap();
    } else if let Some(matches) = matches.subcommand_matches("-C") {
        let s = matches.value_of("local-server").unwrap();
        let c = matches.value_of("listen-server").unwrap();
        client::run_connect_client(&logger, &s, &c).unwrap();
    }
}
