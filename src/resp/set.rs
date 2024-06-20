use crate::RespDecode;
use crate::RespEncode;
use crate::RespError;

use bytes::Buf;
use bytes::BytesMut;

use std::ops::Deref;

use super::calc_total_length;

use super::frame::RespFrame;
use super::parse_length;
use super::BUF_CAP;
use super::CRLF_LEN;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct RespSet(pub(crate) Vec<RespFrame>);

//- set: "~<number-of-elements>\r\n<element-1>...<element-n>"
impl RespEncode for RespSet {
    fn encode(self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BUF_CAP);

        buf.extend_from_slice(format!("~{}\r\n", self.len()).as_bytes());
        for frame in self.0 {
            buf.extend_from_slice(&frame.encode());
        }
        buf
    }
}

impl RespDecode for RespSet {
    const PREFIX: &'static str = "~";

    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let total = calc_total_length(buf, len, end, Self::PREFIX)?;
        if total > buf.len() {
            return Err(RespError::NotComplete);
        }
        buf.advance(end + CRLF_LEN);

        let mut set = Vec::with_capacity(len);
        for _ in 0..len {
            let frame = RespFrame::decode(buf)?;
            set.push(frame);
        }

        Ok(RespSet::new(set))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let (end, len) = parse_length(buf, Self::PREFIX)?;
        let total = calc_total_length(buf, len, end, Self::PREFIX)?;
        Ok(total)
    }
}

impl Deref for RespSet {
    type Target = Vec<RespFrame>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RespSet {
    pub fn new(s: impl Into<Vec<RespFrame>>) -> Self {
        RespSet(s.into())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::BulkString;
    use crate::RespArray;
    use anyhow::Result;

    #[test]
    fn test_set() {
        let frame: RespFrame = RespSet::new([
            RespArray::new([1234.into(), true.into()]).into(),
            BulkString::new("world".to_string()).into(),
        ])
        .into();
        assert_eq!(
            frame.encode(),
            b"~2\r\n*2\r\n:+1234\r\n#t\r\n$5\r\nworld\r\n"
        );
    }

    #[test]
    fn test_set_decode() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"~2\r\n$3\r\nset\r\n$5\r\nhello\r\n");

        let frame = RespSet::decode(&mut buf)?;
        assert_eq!(
            frame,
            RespSet::new(vec![
                BulkString::new(b"set".to_vec()).into(),
                BulkString::new(b"hello".to_vec()).into()
            ])
        );

        Ok(())
    }
}
