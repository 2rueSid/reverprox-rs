use message::{InitializationMessage, Message, MessageType, msg_utils};
use spdlog::info;
use std::{
    error::Error,
    fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use quinn::{ClientConfig, Endpoint};
use rustls::pki_types::CertificateDer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9003);
    let client_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

    // Read the PEM file
    let pem_data =
        fs::read_to_string("examples/server_cert.pem").expect("Failed to read certificate file");

    let pem_file = pem::parse(pem_data).expect("Failed to parse PEM");

    let server_cert = CertificateDer::from(pem_file.contents());

    info!(
        "Loaded server certificate with {} bytes",
        server_cert.as_ref().len()
    );

    let client = make_client_endpoint(client_addr, &[&server_cert])?;

    run_client(&client, server_addr).await;

    client.wait_idle().await;

    Ok(())
}

async fn run_client(endpoint: &Endpoint, server_addr: SocketAddr) {
    let connect = endpoint.connect(server_addr, "localhost").unwrap();
    let connection = connect.await.unwrap();
    let connection_id = msg_utils::generate_uuid();

    let initialization_payload = InitializationMessage::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 20000),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3000),
    )
    .unwrap_or_else(|e| panic!("{e:?}"));

    let init_msg = Message::new(
        MessageType::Initial,
        connection_id,
        initialization_payload.encode(),
    );

    let (mut send, mut recv) = connection.open_bi().await.unwrap();
    info!("[client] connected: addr={}", connection.remote_address());

    tokio::spawn(async move {
        loop {
            match recv.read_chunk(500, true).await {
                Ok(Some(chunk)) => {
                    info!(
                        "[client] received: {:?}",
                        String::from_utf8_lossy(&chunk.bytes)
                    );
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
    send.write_chunk(init_msg.encode()).await.unwrap();
}

fn make_client_endpoint(
    bind_addr: SocketAddr,
    server_certs: &[&[u8]],
) -> Result<Endpoint, Box<dyn Error + Send + Sync + 'static>> {
    let client_cfg = configure_client(server_certs)?;
    let mut endpoint = Endpoint::client(bind_addr)?;
    endpoint.set_default_client_config(client_cfg);
    Ok(endpoint)
}

fn configure_client(
    server_certs: &[&[u8]],
) -> Result<ClientConfig, Box<dyn Error + Send + Sync + 'static>> {
    let mut certs = rustls::RootCertStore::empty();
    for cert in server_certs {
        certs.add(CertificateDer::from(*cert))?;
    }

    Ok(ClientConfig::with_root_certificates(Arc::new(certs))?)
}

// async fn send_tcp_request() -> Result<(), Box<dyn Error>> {
//     let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
//
//     let raw_http = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
//     stream.write_all(raw_http).await?;
//     Ok(())
// }
