#![allow(dead_code)]

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
            Self::SIMPLE => Self::parse_simple(src),
            Self::ERRORS => Self::parse_error(src),
            Self::INTEGERS => Self::parse_integer(src),
            Self::BULK => Self::parse_bulk(src),
            Self::ARRAY => Self::parse_array(src),
            actual => return Err(Error::Protocol(format!("invalid frame type byte `{actual}`"))),
        }
    }

    fn parse_simple(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len() - 1;
        for i in start..end {
            if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
                src.set_position((i + 2) as u64);
                let string = String::from_utf8_lossy(&src.get_ref()[start..i]);
                return Ok(Frame::Simple(string.into()));
            }
        }
        Err(Error::StreamEndedEarly)
    }

    fn parse_error(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len() - 1;
        for i in start..end {
            if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
                src.set_position((i + 2) as u64);
                let string = String::from_utf8_lossy(&src.get_ref()[start..i]);
                return Ok(Frame::Error(string.into()));
            }
        }
        Err(Error::StreamEndedEarly)
    }

    fn parse_integer(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        let start = src.position() as usize;
        let end = src.get_ref().len() - 1;
        for i in start..end {
            if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
                src.set_position((i + 2) as u64);
                let bytes = &src.get_ref()[start..i];
                if let Some(integer) = atoi::atoi::<u64>(bytes) {
                    return Ok(Frame::Integer(integer));
                }
                return Err(Error::Protocol(String::from("")));
            }
        }
        Err(Error::StreamEndedEarly)
    }

    fn parse_bulk(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        Err(Error::StreamEndedEarly) // TODO
    }

    fn parse_array(src: &mut Cursor<&[u8]>) -> Result<Frame, Error> {
        Err(Error::StreamEndedEarly) // TODO
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
    pub fn test_parse_bulk() {}

    #[test]
    pub fn test_parse_array() {
        let source = b"*3\r\none\r\ntwo\r\nthree\r\n" as &[u8];
        let mut source = Cursor::new(source);

        let frame = Frame::parse(&mut source).unwrap();
        assert!(matches!(frame, Frame::Array(_)));
        assert_eq!(Frame::Array(vec![]))
    }
}
