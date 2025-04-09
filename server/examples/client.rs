use bytes::Bytes;
use message::{Message, msg_utils};
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

    tokio::join!(run_client(&client, server_addr));

    client.wait_idle().await;

    Ok(())
}

async fn run_client(endpoint: &Endpoint, server_addr: SocketAddr) {
    let connect = endpoint.connect(server_addr, "localhost").unwrap();
    let connection = connect.await.unwrap();
    let msg = Message::new(
        message::MessageType::Initial,
        msg_utils::generate_uuid(),
        Bytes::from_static(b"Hello world!!"),
    );

    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));

    info!("[client] connected: addr={}", connection.remote_address());

    send.write_chunk(msg.encode())
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));

    send.finish().unwrap_or_else(|e| panic!("{e:?}"));
    let received = recv
        .read_to_end(10)
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));

    info!("Received: {:?}", String::from_utf8(received));
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
