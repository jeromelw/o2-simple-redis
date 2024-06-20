use crate::RespDecode;
use crate::RespEncode;
use crate::RespError;
use bytes::BytesMut;

use super::extract_simple_frame_data;
use super::CRLF_LEN;

//- integer: ":[<+|->]<value>\r\n"
impl RespEncode for i64 {
    fn encode(self) -> Vec<u8> {
        let sign = if self < 0 { "" } else { "+" };
        format!(":{}{}\r\n", sign, self).into_bytes()
    }
}

impl RespDecode for i64 {
    const PREFIX: &'static str = ":";

    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;

        let data = buf.split_to(end + CRLF_LEN);

        let s = String::from_utf8_lossy(&data[Self::PREFIX.len()..end]);

        Ok(s.parse()?)
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::RespFrame;
    use anyhow::Result;

    #[test]
    fn test_integer() {
        let s: RespFrame = 100.into();
        assert_eq!(s.encode(), b":+100\r\n");

        let s: RespFrame = (-100).into();
        assert_eq!(s.encode(), b":-100\r\n");
    }

    #[test]
    fn test_integer_decode() -> Result<(), RespError> {
        let mut buf = BytesMut::from(":1000\r\n");
        let s = i64::decode(&mut buf)?;
        assert_eq!(s, 1000);

        let mut buf = BytesMut::from(":1000\r");
        let s = i64::decode(&mut buf);
        assert_eq!(s.unwrap_err(), RespError::NotComplete);

        Ok(())
    }
}
