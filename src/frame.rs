use std::io::Cursor;

use bytes::{Buf, Bytes};

#[derive(Debug)]
pub enum Error {
    StreamEndedEarly,

    Protocol(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::StreamEndedEarly => write!(f, "stream ended early"),
            Error::Protocol(s) => write!(f, "protocol error, {s}"),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    fn due_to_protocol<S>(reason: S) -> Error
    where
        S: ToString,
    {
        Error::Protocol(reason.to_string())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Frame {
    Simple(String),
    Error(String),
    Integer(u64),
    Bulk(Bytes),
    Nil,
    Array(Vec<Frame>),
}

impl Frame {
    const SIMPLE: u8 = b'+';
    const ERRORS: u8 = b'-';
    const INTEGERS: u8 = b':';
    const BULK: u8 = b'$';
    const ARRAY: u8 = b'*';

    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        if !src.has_remaining() {
            return Err(Error::StreamEndedEarly);
        }
        let first = src.get_u8();
        match first {
            Frame::SIMPLE => Frame::parse_simple(src),
            Frame::ERRORS => Frame::parse_error(src),
            Frame::INTEGERS => Frame::parse_integer(src),
            Frame::BULK => Frame::parse_bulk(src),
            Frame::ARRAY => Frame::parse_array(src),
            actual => Err(Error::due_to_protocol(format!("invalid frame type byte `{actual}`"))),
        }
    }

    fn parse_simple(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        let line = Frame::read_line(src)?;
        Ok(Frame::Simple(String::from_utf8_lossy(line).into()))
    }

    fn parse_error(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        let line = Frame::read_line(src)?;
        Ok(Frame::Error(String::from_utf8_lossy(line).into()))
    }

    fn parse_integer(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        let line = Frame::read_line(src)?;
        let integer = atoi::atoi::<u64>(line).ok_or_else(|| Error::due_to_protocol("invalid frame format"))?;
        Ok(Frame::Integer(integer))
    }

    fn parse_bulk(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        if !src.has_remaining() {
            return Err(Error::StreamEndedEarly);
        }
        let first = src.chunk()[0];
        return match first {
            b'-' => {
                let line = Frame::read_line(src)?;
                if line != b"-1" {
                    return Err(Error::StreamEndedEarly);
                }
                Ok(Frame::Nil)
            }
            _ => {
                let line = Frame::read_line(src)?;
                let length = atoi::atoi::<u64>(line).ok_or_else(|| Error::due_to_protocol("invalid frame format"))?;
                let n = (length + 2) as usize;
                if src.remaining() < n {
                    return Err(Error::StreamEndedEarly);
                }
                let data = Bytes::copy_from_slice(&src.chunk()[..length as usize]);
                src.advance(n);
                Ok(Frame::Bulk(data))
            }
        };
    }

    fn parse_array(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        let line = Frame::read_line(src)?;
        let length = atoi::atoi::<u64>(line).ok_or_else(|| Error::due_to_protocol("invalid frame format"))?;
        let mut array = Vec::with_capacity(length as usize);

        for _ in 0..length {
            array.push(Frame::parse(src)?);
        }
        Ok(Frame::Array(array))
    }

    fn read_line<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len() - 1;
        for i in start..end {
            if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
                src.set_position((i + 2) as u64);
                return Ok(&src.get_ref()[start..i]);
            }
        }
        Err(Error::StreamEndedEarly)
    }
}

impl std::fmt::Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Frame::Simple(s) => s.fmt(f),
            Frame::Error(e) => write!(f, "error: {}", e),
            Frame::Integer(num) => num.fmt(f),
            _ => todo!(),
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    pub fn test_parse_simple() {
        let source = b"+PONG\r\n" as &[u8];
        let mut source = Cursor::new(source);

        let frame = Frame::parse(&mut source).unwrap();

        assert_eq!(Frame::Simple(String::from("PONG")), frame);
    }

    #[test]
    pub fn test_parse_error() {
        let source = b"-ERR AUTH <password> called without any password configured for the default user. Are you sure your configuration is correct?\r\n" as &[u8];
        let mut source = Cursor::new(source);

        let frame = Frame::parse(&mut source).unwrap();

        assert_eq!(Frame::Error(String::from("ERR AUTH <password> called without any password configured for the default user. Are you sure your configuration is correct?")), frame);
    }

    #[test]
    pub fn test_parse_integer() {
        let source = b":791\r\n" as &[u8];
        let mut source = Cursor::new(source);

        let frame = Frame::parse(&mut source).unwrap();

        assert_eq!(Frame::Integer(791), frame);
    }

    #[test]
    pub fn test_parse_bulk() {
        let source = b"$11\r\nHello world\r\n" as &[u8];
        let mut source = Cursor::new(source);

        let frame = Frame::parse(&mut source).unwrap();

        assert_eq!(Frame::Bulk(Bytes::from(b"Hello world" as &[u8])), frame);
    }

    #[test]
    pub fn test_parse_array() {
        let source = b"*2\r\n+one\r\n+two\r\n" as &[u8];
        let mut source = Cursor::new(source);

        let frame = Frame::parse(&mut source).unwrap();

        assert_eq!(Frame::Array(vec![Frame::Simple("one".to_owned()), Frame::Simple("two".to_owned()),]), frame);
    }
}
