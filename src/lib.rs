use std::io;

use tokio::io::{split, AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter, ReadHalf, WriteHalf};
use tokio::net::{TcpStream, ToSocketAddrs};

pub struct Connection {
    reader: BufReader<ReadHalf<TcpStream>>,
    writer: BufWriter<WriteHalf<TcpStream>>,
}

impl Connection {
    pub async fn connect<A>(addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        fn __new(stream: TcpStream) -> Connection {
            let (reader, writer) = split(stream);
            let reader = BufReader::new(reader);
            let writer = BufWriter::new(writer);
            Connection { reader, writer }
        }

        let stream = TcpStream::connect(addr).await?;
        Ok(__new(stream))
    }

    pub async fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.writer.write_all(data).await?;
        Ok(())
    }

    pub async fn receive(&mut self) -> io::Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(128);
        self.reader.read_until(b'\n', &mut buffer).await?;
        Ok(buffer)
    }
}
