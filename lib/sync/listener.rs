use std::net::{ToSocketAddrs, UdpSocket};
use std::time::Duration;

use crate::Result;

const EXPECTED_PACKET: &[u8] = &[2u8, b'm', b'u', b's', b'o', b's', b'y', b'n', b'c'];
const RESPONSE_PACKET: &[u8] = &[2u8, b's', b'y', b'n', b'c', b'm', b'u', b's', b'o'];

#[derive(Debug)]
pub struct Listener {
    socket: UdpSocket,
}

impl Listener {
    pub fn bind<A: ToSocketAddrs>(address: A) -> Result<Self> {
        let socket = UdpSocket::bind(address)?;
        Ok(Self { socket })
    }

    pub fn listen(self) -> Result<()> {
        let mut buf = vec![0u8; EXPECTED_PACKET.len()];

        loop {
            let (no_bytes, src) = self.socket.recv_from(&mut buf)?;
            if &buf[..no_bytes] == EXPECTED_PACKET {
                self.socket.send_to(RESPONSE_PACKET, src)?;
            }

            std::thread::sleep(Duration::from_secs(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn listen() -> Result<()> {
        let listener = Listener::bind("0.0.0.0:54256")?;
        Ok(listener.listen()?)
    }
}
