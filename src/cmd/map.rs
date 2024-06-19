use crate::cmd::RESP_OK;
use crate::{
    cmd::{CommandError, Get, Set},
    RespArray, RespFrame,
};

use super::{extract_args, validator_command, CommandExecutor};

impl CommandExecutor for Get {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        match backend.get(&self.key) {
            Some(value) => value,
            None => RespFrame::Null(crate::RespNull),
        }
    }
}

impl CommandExecutor for Set {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        backend.set(self.key, self.value);
        RESP_OK.clone()
    }
}

impl TryFrom<RespArray> for Get {
    type Error = CommandError;

    fn try_from(arr: RespArray) -> Result<Self, Self::Error> {
        validator_command(&arr, &["get"], 1)?;

        let mut args = extract_args(arr, 1)?.into_iter();

        match args.next() {
            Some(RespFrame::BulkString(key)) => Ok(Get {
                key: String::from_utf8(key.0)?,
            }),
            _ => Err(CommandError::InvalidArgument("Invalid key".to_string())),
        }
    }
}

impl TryFrom<RespArray> for Set {
    type Error = CommandError;

    fn try_from(arr: RespArray) -> Result<Self, Self::Error> {
        validator_command(&arr, &["set"], 2)?;

        let mut args = extract_args(arr, 1)?.into_iter();

        let key = match args.next() {
            Some(RespFrame::BulkString(key)) => String::from_utf8(key.0)?,
            _ => return Err(CommandError::InvalidArgument("Invalid key".to_string())),
        };

        let value = match args.next() {
            Some(value) => value,
            _ => return Err(CommandError::InvalidArgument("Invalid value".to_string())),
        };

        Ok(Set { key, value })
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use bytes::BytesMut;

    use super::*;
    use crate::{RespArray, RespDecode};

    #[test]
    fn test_get_try_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$3\r\nget\r\n$5\r\nhello\r\n");

        let frame = RespArray::decode(&mut buf)?;

        let result: Get = frame.try_into()?;

        assert_eq!(result.key, "hello");

        Ok(())
    }

    #[test]
    fn test_set_try_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*3\r\n$3\r\nset\r\n$5\r\nhello\r\n$5\r\nworld\r\n");

        let frame = RespArray::decode(&mut buf)?;

        let result: Set = frame.try_into()?;

        assert_eq!(result.key, "hello");
        assert_eq!(result.value, RespFrame::BulkString(b"world".into()));

        Ok(())
    }

    #[test]
    fn test_get_set_execute() -> Result<()> {
        let backend = crate::Backend::new();

        let set = Set {
            key: "hello".to_string(),
            value: RespFrame::BulkString(b"world".into()),
        };

        let get = Get {
            key: "hello".to_string(),
        };

        let set_frame = set.execute(&backend);
        let get_frame = get.execute(&backend);

        assert_eq!(set_frame, RESP_OK.clone());
        assert_eq!(get_frame, RespFrame::BulkString(b"world".into()));

        Ok(())
    }
}
