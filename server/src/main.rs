use std::{error::Error, fs::File, io::Write, path::Path, time::Duration};

use bytes::Bytes;
use message::{InitializationMessage, MessageType};
use pem::Pem;
use spdlog::prelude::info;
use tokio::time::sleep;

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
                tokio::spawn(async move {
                    loop {
                        match recv.read_chunk(500, true).await {
                            Ok(Some(chunk)) => {
                                let msg = message::Message::decode(&chunk.bytes).unwrap();
                                info!("[server] received: {:?}", msg);

                                match msg.message_type {
                                    MessageType::Initial => {
                                        info!("Message Type - Initial");
                                        let payload = InitializationMessage::decode(&msg.payload);

                                        info!("Message Payload -> {:?}", payload);
                                    }
                                    MessageType::Data => {}
                                    MessageType::Close => {}
                                    MessageType::Ping => {}
                                    _ => panic!("Unreachable"),
                                }
                            }
                            Ok(None) => {
                                continue;
                            }
                            Err(e) => {
                                info!("[server] error reading: {e:?}");
                                break;
                            }
                        }
                    }
                });

                send.write_chunk(Bytes::from_static(b"response"))
                    .await
                    .unwrap_or_else(|e| panic!("Err: {e:?}"));

                sleep(Duration::from_secs(2)).await;
                info!("here");

                send.write_chunk(Bytes::from_static(b"response22"))
                    .await
                    .unwrap_or_else(|e| panic!("Err: {e:?}"));
            }
        });
    }
}
