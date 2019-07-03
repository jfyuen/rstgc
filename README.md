# TCP Gender Changer (Rust version)

This is a small port of TGC in Rust to try out the language, it is buggy and not fully functional.
TCP Gender Changer is a small utility to connect/connect on a local service to a remote server using a listen/listen setup. 
More information on [wikipedia](https://en.wikipedia.org/wiki/TCP_Gender_Changer).

This is heavily inspired by http://tgcd.sourceforge.net

## Installation

```bash
$ cargo build --release
```

## Usage

On the node on which to access a server on a LAN, run the connect/connect setup:
```bash
$ rstgc -C -s ${LOCAL_ADDRESS} -c ${REMOTE_ADDRESS}
```

On the remote node on which you can connect to, run the listen/listen setup:
```bash
$ rstgc -L -q ${EXTERNAL_PORT} -p ${LOCAL_PORT}
```

The remote node now has access to the local server on the LAN via `${LOCAL_PORT}`. A connection is initiated to the local server only when data are received from the remote server, unless the `-r` flag is specified to automatically (re)connect to the local server.


### Example

Forward local port 8080 to a remote server (`${REMOTE}`) on port 8000 via port 80. 
On the local server:
```bash
$ rstgc -C -s 127.0.0.1:8080 -c ${REMOTE}:80
```

On the remote server:
```bash
$ rstgc -L -q ${REMOTE}:80 -p 127.0.0.1:8000
```
