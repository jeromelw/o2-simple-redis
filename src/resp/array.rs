use crate::RespDecode;
use crate::RespEncode;
use crate::RespError;
use crate::RespFrame;

use bytes::Buf;
use bytes::BytesMut;

use std::ops::Deref;

use super::calc_total_length;
use super::extract_fixed_data;
use super::parse_length;
use super::BUF_CAP;
use super::CRLF_LEN;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct RespArray(pub(crate) Vec<RespFrame>);

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct RespNullArray;

//- array: "*<number-of-elements>\r\n<element-1>...<element-n>"
//        - "*2\r\n$3\r\nget\r\n$5\r\nhello\r\n"
impl RespEncode for RespArray {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);
        buf.extend_from_slice(format!("*{}\r\n", self.len()).as_bytes());
        for frame in self.0 {
            buf.extend_from_slice(&frame.encode());
        }
        buf
    }
}

//- null array: "*-1\r\n"
impl RespEncode for RespNullArray {
    fn encode(self) -> Vec<u8> {
        b"*-1\r\n".to_vec()
    }
}

impl RespDecode for RespArray {
    const PREFIX: &'static str = "*";

    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let total = calc_total_length(buf, len, end, Self::PREFIX)?;
        if total > buf.len() {
            return Err(RespError::NotComplete);
        }
        buf.advance(end + CRLF_LEN);

        let mut array = Vec::new();
        for _ in 0..len {
            let frame = RespFrame::decode(buf)?;
            array.push(frame);
        }

        Ok(RespArray::new(array))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let total = calc_total_length(buf, len, end, Self::PREFIX)?;
        Ok(total)
    }
}

impl RespDecode for RespNullArray {
    const PREFIX: &'static str = "*";

    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        extract_fixed_data(buf, "*-1\r\n", "NullArray")?;
        Ok(RespNullArray)
    }

    fn expect_length(_buf: &[u8]) -> Result<usize, RespError> {
        Ok(5)
    }
}

impl Deref for RespArray {
    type Target = Vec<RespFrame>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RespArray {
    pub fn new(arr: impl Into<Vec<RespFrame>>) -> Self {
        RespArray(arr.into())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::BulkString;
    use anyhow::Result;

    #[test]
    fn test_array() {
        let frame: RespFrame = RespArray::new(vec![
            BulkString::new("set".to_string()).into(),
            BulkString::new("hello".to_string()).into(),
            BulkString::new("world".to_string()).into(),
        ])
        .into();
        assert_eq!(
            &frame.encode(),
            b"*3\r\n$3\r\nset\r\n$5\r\nhello\r\n$5\r\nworld\r\n"
        );
    }

    #[test]
    fn test_null_array() {
        let s: RespFrame = RespNullArray.into();
        assert_eq!(s.encode(), b"*-1\r\n");
    }

    #[test]
    fn test_null_array_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*-1\r\n");

        let frame = RespNullArray::decode(&mut buf)?;
        assert_eq!(frame, RespNullArray);

        Ok(())
    }

    #[test]
    fn test_array_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n$5\r\nhello\r\n");

        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new([b"set".into(), b"hello".into()]));

        buf.extend_from_slice(b"*2\r\n$3\r\nset\r\n");
        let ret = RespArray::decode(&mut buf);
        assert_eq!(ret.unwrap_err(), RespError::NotComplete);

        buf.extend_from_slice(b"$5\r\nhello\r\n");
        let frame = RespArray::decode(&mut buf)?;
        assert_eq!(frame, RespArray::new([b"set".into(), b"hello".into()]));

        Ok(())
    }
}