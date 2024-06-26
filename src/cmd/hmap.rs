use crate::{
    cmd::{CommandError, HGet, HGetAll, HSet},
    BulkString, RespArray, RespFrame,
};

use super::{extract_args, validator_command, CommandExecutor};

impl CommandExecutor for HGet {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        match backend.hget(&self.key, &self.field) {
            Some(value) => value,
            None => RespFrame::Null(crate::RespNull),
        }
    }
}

impl CommandExecutor for HSet {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        backend.hset(self.key, self.field, self.value);
        crate::cmd::RESP_OK.clone()
    }
}

impl CommandExecutor for HGetAll {
    fn execute(self, backend: &crate::Backend) -> RespFrame {
        let hmap = backend.hmap.get(&self.key);

        match hmap {
            Some(hmap) => {
                let mut data = Vec::with_capacity(hmap.len());

                for v in hmap.iter() {
                    let key = v.key().to_owned();
                    data.push((key, v.value().clone()));
                }

                if self.sort {
                    data.sort_by(|a, b| a.0.cmp(&b.0));
                }

                let ret = data
                    .into_iter()
                    .flat_map(|(k, v)| vec![BulkString::new(k).into(), v])
                    .collect::<Vec<RespFrame>>();

                RespArray::new(ret).into()
            }
            None => RespArray::new([]).into(),
        }
    }
}

impl TryFrom<RespArray> for HGet {
    type Error = CommandError;

    fn try_from(arr: RespArray) -> Result<Self, Self::Error> {
        validator_command(&arr, &["hget"], 2)?;

        let mut args = extract_args(arr, 1)?.into_iter();

        let key = match args.next() {
            Some(RespFrame::BulkString(key)) => String::from_utf8(key.0)?,
            _ => return Err(CommandError::InvalidArgument("Invalid key".to_string())),
        };

        let field = match args.next() {
            Some(RespFrame::BulkString(field)) => String::from_utf8(field.0)?,
            _ => return Err(CommandError::InvalidArgument("Invalid field".to_string())),
        };

        Ok(HGet { key, field })
    }
}

impl TryFrom<RespArray> for HSet {
    type Error = CommandError;

    fn try_from(arr: RespArray) -> Result<Self, Self::Error> {
        validator_command(&arr, &["hset"], 3)?;

        let mut args = extract_args(arr, 1)?.into_iter();

        let key = match args.next() {
            Some(RespFrame::BulkString(key)) => String::from_utf8(key.0)?,
            _ => return Err(CommandError::InvalidArgument("Invalid key".to_string())),
        };

        let field = match args.next() {
            Some(RespFrame::BulkString(field)) => String::from_utf8(field.0)?,
            _ => return Err(CommandError::InvalidArgument("Invalid field".to_string())),
        };

        let value = match args.next() {
            Some(value) => value,
            _ => return Err(CommandError::InvalidArgument("Invalid value".to_string())),
        };

        Ok(HSet { key, field, value })
    }
}

impl TryFrom<RespArray> for HGetAll {
    type Error = CommandError;

    fn try_from(arr: RespArray) -> Result<Self, Self::Error> {
        validator_command(&arr, &["hgetall"], 1)?;

        let mut args = extract_args(arr, 1)?.into_iter();

        let key = match args.next() {
            Some(RespFrame::BulkString(key)) => String::from_utf8(key.0)?,
            _ => return Err(CommandError::InvalidArgument("Invalid key".to_string())),
        };

        Ok(HGetAll { key, sort: false })
    }
}

#[cfg(test)]

mod tests {
    use crate::cmd::RESP_OK;
    use anyhow::Result;
    use bytes::BytesMut;

    use super::*;
    use crate::{RespArray, RespDecode};

    #[test]
    fn test_hget_try_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*3\r\n$4\r\nhget\r\n$3\r\nmap\r\n$5\r\nhello\r\n");

        let arr = RespArray::decode(&mut buf)?;

        let get = HGet::try_from(arr)?;

        assert_eq!(get.key, "map");
        assert_eq!(get.field, "hello");

        Ok(())
    }

    #[test]
    fn test_hset_try_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*4\r\n$4\r\nhset\r\n$3\r\nmap\r\n$5\r\nhello\r\n$5\r\nworld\r\n");

        let arr = RespArray::decode(&mut buf)?;

        let set = HSet::try_from(arr)?;

        assert_eq!(set.key, "map");
        assert_eq!(set.field, "hello");
        assert_eq!(set.value, RespFrame::BulkString("world".into()));

        Ok(())
    }

    #[test]
    fn test_hgetall_try_from_resp_array() -> Result<()> {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"*2\r\n$7\r\nhgetall\r\n$3\r\nmap\r\n");

        let arr = RespArray::decode(&mut buf)?;

        let get_all = HGetAll::try_from(arr)?;

        assert_eq!(get_all.key, "map");

        Ok(())
    }

    #[test]
    fn test_hset_hget_hgetall_commands() -> Result<()> {
        let backend = crate::Backend::new();
        let cmd = HSet {
            key: "map".to_string(),
            field: "hello".to_string(),
            value: RespFrame::BulkString(b"world".into()),
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, RESP_OK.clone());

        let cmd = HSet {
            key: "map".to_string(),
            field: "hello1".to_string(),
            value: RespFrame::BulkString(b"world1".into()),
        };
        cmd.execute(&backend);

        let cmd = HGet {
            key: "map".to_string(),
            field: "hello".to_string(),
        };
        let result = cmd.execute(&backend);
        assert_eq!(result, RespFrame::BulkString(b"world".into()));

        let cmd = HGetAll {
            key: "map".to_string(),
            sort: true,
        };
        let result = cmd.execute(&backend);

        let expected = RespArray::new([
            BulkString::from("hello").into(),
            BulkString::from("world").into(),
            BulkString::from("hello1").into(),
            BulkString::from("world1").into(),
        ]);
        assert_eq!(result, expected.into());
        Ok(())
    }
}
