use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug, Clone)]
pub struct Config {
    _ipv4: IpAddr,
    _port: u16,

    pub host: SocketAddr,
}

impl Config {
    pub fn new() -> Config {
        let ipv4 = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let port = 9003;

        let host = SocketAddr::new(ipv4, port);

        Config {
            _ipv4: ipv4,
            _port: port,
            host,
        }
    }
}
