use crate::Error;
use crate::Result;

use crate::Config;
use crate::Connection;

use crate::frame;
use crate::packet;
use crate::rand;

pub struct Pipe {
    pub client: Box<Connection>,
    pub server: Box<Connection>,
}

impl Pipe {
    pub fn new() -> Result<Pipe> {
        let mut config = Config::new(crate::VERSION_DRAFT17)?;
        config.load_cert_chain_from_pem_file("examples/cert.crt")?;
        config.load_priv_key_from_pem_file("examples/cert.key")?;
        config.set_initial_max_data(30);
        config.set_initial_max_stream_data_bidi_local(15);
        config.set_initial_max_stream_data_bidi_remote(15);
        config.set_initial_max_streams_bidi(3);
        config.set_initial_max_streams_uni(3);
        config.verify_peer(false);

        Ok(Pipe {
            client: create_conn(&mut config, false)?,
            server: create_conn(&mut config, true)?,
        })
    }

    pub fn with_client_config(cln_config: &mut Config) -> Result<Pipe> {
        let mut config = Config::new(crate::VERSION_DRAFT17)?;
        config.load_cert_chain_from_pem_file("examples/cert.crt")?;
        config.load_priv_key_from_pem_file("examples/cert.key")?;
        config.set_initial_max_data(30);
        config.set_initial_max_stream_data_bidi_local(15);
        config.set_initial_max_stream_data_bidi_remote(15);
        config.set_initial_max_streams_bidi(3);
        config.set_initial_max_streams_uni(3);

        Ok(Pipe {
            client: create_conn(cln_config, false)?,
            server: create_conn(&mut config, true)?,
        })
    }

    pub fn handshake(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut len = self.client.send(buf)?;

        while !self.client.is_established() && !self.server.is_established() {
            len = recv_send(&mut self.server, buf, len)?;
            len = recv_send(&mut self.client, buf, len)?;
        }

        Ok(())
    }

    pub fn advance(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut client_done = false;
        let mut server_done = false;

        let mut len = 0;

        while !client_done || !server_done {
            len = recv_send(&mut self.client, buf, len)?;
            client_done = len == 0;

            len = recv_send(&mut self.server, buf, len)?;
            server_done = len == 0;
        }

        Ok(())
    }

    pub fn send_pkt_to_server(
        &mut self, pkt_type: packet::Type, frames: &[frame::Frame],
        buf: &mut [u8],
    ) -> Result<usize> {
        let len = self.client.encode_pkt_for_test(pkt_type, frames, buf)?;

        recv_send(&mut self.server, buf, len)
    }
}

fn create_conn(cfg: &mut Config, is_server: bool) -> Result<Box<Connection>> {
    let mut scid = [0; 16];
    rand::rand_bytes(&mut scid[..]);

    Connection::new(&scid, None, cfg, is_server)
}

fn recv_send(conn: &mut Connection, buf: &mut [u8], len: usize) -> Result<usize> {
    let mut left = len;

    while left > 0 {
        match conn.recv(&mut buf[len - left..len]) {
            Ok(read) => left -= read,

            Err(Error::Done) => break,

            Err(e) => return Err(e),
        }
    }

    assert_eq!(left, 0);

    let mut off = 0;

    while off < buf.len() {
        match conn.send(&mut buf[off..]) {
            Ok(write) => off += write,

            Err(Error::Done) => break,

            Err(e) => return Err(e),
        }
    }

    Ok(off)
}
