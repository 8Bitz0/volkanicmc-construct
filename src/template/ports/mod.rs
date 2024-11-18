use serde::{Deserialize, Serialize};
use tracing::error;

// "{id}": "{port}/{protocol}"
pub type PortMap = std::collections::HashMap<String, String>;

#[derive(Clone, Debug, PartialEq)]
pub enum PortProtocol {
    Tcp,
    Udp,
}

pub fn parse_port(port: impl std::fmt::Display) -> Option<(u16, PortProtocol)> {
    let port = port.to_string();
    let mut split = port.split('/');
    let port = split.next().unwrap().parse().unwrap();
    let protocol = match split.next().unwrap() {
        "tcp" => PortProtocol::Tcp,
        "udp" => PortProtocol::Udp,
        _ => return None,
    };

    Some((port, protocol))
}

pub async fn parse_port_map(ports: PortMap) -> Vec<(u16, PortProtocol)> {
    let mut parsed_ports = Vec::new();
    for (id, port) in ports {
        if let Some((port, protocol)) = parse_port(port) {
            parsed_ports.push((port, protocol));
        } else {
            error!("Failed to parse port for id: {}", id);
        }
    }

    parsed_ports
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn parse_port_test() {
        let port = "25565/tcp";
        let (port, protocol) = super::parse_port(port).unwrap();
        assert_eq!(port, 25565);
        assert_eq!(protocol, super::PortProtocol::Tcp);

        let port = "19132/udp";
        let (port, protocol) = super::parse_port(port).unwrap();
        assert_eq!(port, 19132);
        assert_eq!(protocol, super::PortProtocol::Udp);
    }

    #[tokio::test]
    async fn parse_port_map_test() {
        let mut ports = std::collections::HashMap::new();
        ports.insert("tcp-port".to_string(), "25565/tcp".to_string());
        ports.insert("udp-port".to_string(), "19132/udp".to_string());

        let parsed_ports = super::parse_port_map(ports).await;
        assert_eq!(parsed_ports.len(), 2);
        assert_eq!(parsed_ports[0], (25565, super::PortProtocol::Tcp));
        assert_eq!(parsed_ports[1], (19132, super::PortProtocol::Udp));
    }
}
