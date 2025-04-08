use std::{error::Error, fs::File, io::Write, path::Path};

use pem::Pem;
use spdlog::prelude::info;

mod config;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let config = config::Config::new();

    let (endpoint, server_cert) = server::make_server_endpoint(config.host)?;

    let pem = Pem::new("CERTIFICATE", server_cert.to_vec());

    let cert_path = Path::new("examples/server_cert.pem");
    let mut file = File::create(&cert_path)?;
    file.write(pem::encode(&pem).as_bytes())?;

    println!("Saved server_cert to {}", cert_path.display());

    info!("Address: {:?}", config.host);
    loop {
        let connection = endpoint.accept().await.unwrap().await.unwrap();

        tokio::spawn(async move {
            println!(
                "[server] incoming connection: addr={}",
                connection.remote_address()
            );
        });
    }
}
