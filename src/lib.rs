#![allow(dead_code)]

use std::io;

use bytes::BytesMut;
use tokio::io::BufWriter;
use tokio::net::{TcpStream, ToSocketAddrs};

mod frame;

pub struct Connection {
    inner: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub async fn connect<A>(addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        fn __new(stream: TcpStream) -> Connection {
            let inner = BufWriter::new(stream);
            let buffer = BytesMut::with_capacity(4 * 1024);

            Connection { inner, buffer }
        }

        let stream = TcpStream::connect(addr).await?;
        Ok(__new(stream))
    }
}
