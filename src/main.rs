#[macro_use]
extern crate futures;
extern crate tokio_tls;

use getopts::Options;
use native_tls::TlsConnector;
use std::env;
use std::net::ToSocketAddrs;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::{TcpListener, TcpStream};

type Error = Box<dyn std::error::Error + Sync + Send + 'static>;
static DEBUG: AtomicBool = AtomicBool::new(false);

fn print_usage(program: &str, opts: Options) {
    let program_path = std::path::PathBuf::from(program);
    let program_name = program_path.file_stem().unwrap().to_str().unwrap();
    let brief = format!(
        "Usage: {} [-b BIND_ADDR] -l LOCAL_PORT -s STRATUM_HOST",
        program_name
    );
    print!("{}", opts.usage(&brief));
}

mod config {
    #[derive(Debug, Clone)]
    pub struct Stratum {
        pub host: String,
        pub port: i32,
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.reqopt(
        "l",
        "local-port",
        "The local port to which stratum-proxy should bind.",
        "LOCAL_PORT",
    );
    opts.optopt(
        "b",
        "bind",
        "The address on which to listen for incoming requests.",
        "BIND_ADDR",
    );
    opts.reqopt(
        "s",
        "stratum",
        "The remote stratum server to which mining work will be forwarded.",
        "STRATUM_HOST",
    );

    opts.optflag("d", "debug", "Enable debug mode");

    let matches = match opts.parse(&args[1..]) {
        Ok(opts) => opts,
        Err(_) => {
            print_usage(&program, opts);
            std::process::exit(-1);
        }
    };

    DEBUG.store(matches.opt_present("d"), Ordering::Relaxed);
    let stratum_host = match matches.opt_str("s") {
        Some(host) => host,
        None => "miningforce.org".to_owned(),
    };

    let local_port: i32 = matches.opt_str("l").unwrap().parse()?;
    let bind_addr = match matches.opt_str("b") {
        Some(addr) => addr,
        None => "127.0.0.1".to_owned(),
    };

    forward(&bind_addr, local_port, &stratum_host).await
}

async fn forward(bind_ip: &str, local_port: i32, stratum_host: &str) -> Result<(), Error> {
    // Listen on the specified IP and port
    let bind_addr = format!("{}:{}", bind_ip, local_port);
    let bind_sock = bind_addr.parse::<std::net::SocketAddr>()?;
    let mut listener = TcpListener::bind(&bind_sock).await?;
    println!("Listening on {}", listener.local_addr().unwrap());
    let stratum_port: i32 = 443;

    let stratum = config::Stratum {
        host: stratum_host.to_owned(),
        port: stratum_port.to_owned(),
    };

    // We have either been provided an IP address or a host name.
    // Instead of trying to check its format, just trying creating a SocketAddr from it.
    let parse_result =
        format!("{}:{}", &stratum_host, stratum_port).parse::<std::net::SocketAddr>();
    let remote_addr = match parse_result {
        Ok(s) => s,
        Err(_) => {
            // It's a hostname; we're going to need to resolve it.
            let domain = format!("{}:{}", &stratum_host, stratum_port);
            let resolution = domain
                .to_socket_addrs()?
                .next()
                .ok_or("Failed to resolve DNS.")?;

            println!("Successfully resolved {} to {}", domain, resolution);

            resolution
        }
    };

    loop {
        let (mut client, client_addr) = listener.accept().await?;
        let stratum = stratum.clone();

        tokio::spawn(async move {
            println!("New connection from {}", client_addr);

            // Establish connection to upstream for each incoming client connection
            let remote = TcpStream::connect(&remote_addr).await?;
            let cx = TlsConnector::builder().build()?;
            let cx = tokio_tls::TlsConnector::from(cx);
            let remote = cx.connect(&stratum.host, remote).await?;

            let (mut client_recv, mut client_send) = client.split();
            let (mut remote_recv, mut remote_send) = tokio::io::split(remote);

            let (remote_bytes_copied, client_bytes_copied) = join!(
                tokio::io::copy(&mut remote_recv, &mut client_send),
                tokio::io::copy(&mut client_recv, &mut remote_send),
            );

            match remote_bytes_copied {
                Ok(count) => {
                    if DEBUG.load(Ordering::Relaxed) {
                        eprintln!(
                            "Transferred {} bytes from remote client {} to upstream server",
                            count, client_addr
                        );
                    }
                }
                Err(err) => {
                    eprintln!(
                        "Error writing from remote client {} to upstream server!",
                        client_addr
                    );
                    eprintln!("{:?}", err);
                }
            };

            match client_bytes_copied {
                Ok(count) => {
                    if DEBUG.load(Ordering::Relaxed) {
                        eprintln!(
                            "Transferred {} bytes from upstream server to remote client {}",
                            count, client_addr
                        );
                    }
                }
                Err(err) => {
                    eprintln!(
                        "Error writing bytes from upstream server to remote client {}",
                        client_addr
                    );
                    eprintln!("{:?}", err);
                }
            };

            let r: Result<(), Error> = Ok(());
            r
        });
    }
}
