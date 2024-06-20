use crate::RespDecode;
use crate::RespEncode;
use crate::RespError;

use bytes::BytesMut;

use std::ops::Deref;

use super::extract_simple_frame_data;
use super::CRLF_LEN;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct SimpleString(pub(crate) String);

//- simple string: "+OK\r\n"
impl RespEncode for SimpleString {
    fn encode(self) -> Vec<u8> {
        format!("+{}\r\n", self.0).into_bytes()
    }
}

impl RespDecode for SimpleString {
    const PREFIX: &'static str = "+";
    fn decode(buf: &mut BytesMut) -> Result<Self, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;

        let data = buf.split_to(end + CRLF_LEN);

        let s = String::from_utf8_lossy(&data[Self::PREFIX.len()..end]);

        Ok(SimpleString::new(s.to_string()))
    }

    fn expect_length(buf: &[u8]) -> Result<usize, RespError> {
        let end = extract_simple_frame_data(buf, Self::PREFIX)?;
        Ok(end + CRLF_LEN)
    }
}

impl Deref for SimpleString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SimpleString {
    pub fn new(s: impl Into<String>) -> Self {
        SimpleString(s.into())
    }
}

impl From<&str> for SimpleString {
    fn from(s: &str) -> Self {
        SimpleString(s.to_string())
    }
}

impl AsRef<str> for SimpleString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::RespFrame;
    use anyhow::Result;

    #[test]
    fn test_simple_string() {
        let s: RespFrame = SimpleString::new("OK".to_string()).into();
        assert_eq!(s.encode(), b"+OK\r\n");
    }

    #[test]
    fn test_simple_string_decode() -> Result<(), RespError> {
        let mut buf = BytesMut::from("+OK\r\n");
        let s = SimpleString::decode(&mut buf)?;
        assert_eq!(s, SimpleString::new("OK".to_string()));

        let mut buf = BytesMut::from("+OK\r");
        let s = SimpleString::decode(&mut buf);
        assert_eq!(s.unwrap_err(), RespError::NotComplete);

        Ok(())
    }
}
