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
    file.write_all(pem::encode(&pem).as_bytes())
        .unwrap_or_else(|e| panic!("Error while writing pem file {e:?}"));

    info!("Saved server_cert to {}", cert_path.display());

    info!("Address: {:?}", config.host);
    loop {
        let connection = endpoint.accept().await.unwrap().await.unwrap();

        tokio::spawn(async move {
            info!(
                "[server] incoming connection: addr={}",
                connection.remote_address()
            );

            while let Ok((mut send, mut recv)) = connection.accept_bi().await {
                // Because it is a bidirectional stream, we can both send and receive.
                let received = recv
                    .read_to_end(50)
                    .await
                    .unwrap_or_else(|e| panic!("Err: {e:?}"));

                info!("request: {:?}", String::from_utf8(received));
                send.write_all(b"response")
                    .await
                    .unwrap_or_else(|e| panic!("Err: {e:?}"));
                send.finish().unwrap_or_else(|e| panic!("Err: {e:?}"))
            }
        });
    }
}
