use std::net::ToSocketAddrs;

use clap::crate_version;
use jsonrpc_core::{IoHandler, Result as RpcResult};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::{Server, ServerBuilder};

use crate::Error;
use crate::Result;

#[rpc]
pub trait Rpc {
    #[rpc(name = "version")]
    fn version(&self) -> RpcResult<String> {
        Ok(crate_version!().to_string())
    }

    #[rpc(name = "build_replica_db")]
    fn build_replica_db(&self, replica_addr: String) -> RpcResult<()> {
        todo!()
    }
}

pub struct RpcImpl;
impl Rpc for RpcImpl {}

fn build_handler() -> IoHandler {
    let mut io = IoHandler::new();
    io.extend_with(RpcImpl.to_delegate());
    io
}

pub struct RpcServer {
    server: Server,
}

impl RpcServer {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| Error::InvalidAddress)?;

        let server = ServerBuilder::new(build_handler()).start_http(&addr)?;
        Ok(Self { server })
    }

    pub fn listen(self) {
        self.server.wait()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn listen() -> Result<()> {
        let server = RpcServer::bind("0.0.0.0:54256")?;
        server.listen();
        Ok(())
    }
}
