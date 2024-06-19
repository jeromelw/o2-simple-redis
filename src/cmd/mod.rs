mod hmap;
mod map;

use crate::Backend;
use crate::{RespArray, RespError, RespFrame, SimpleString};
use enum_dispatch::enum_dispatch;
use lazy_static::lazy_static;
use thiserror::Error;

lazy_static! {
    static ref RESP_OK: RespFrame = SimpleString::new("OK").into();
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("{0}")]
    RespError(#[from] RespError),

    #[error("from ParseIntError: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

#[enum_dispatch]
pub trait CommandExecutor {
    fn execute(self, backend: &Backend) -> RespFrame;
}

#[enum_dispatch(CommandExecutor)]
#[derive(Debug)]

pub enum Command {
    Get(Get),
    Set(Set),
    HSet(HSet),
    HGet(HGet),
    HGetAll(HGetAll),
    Unrecognized(Unrecognized),
}

#[derive(Debug)]
pub struct Get {
    key: String,
}

#[derive(Debug)]
pub struct Set {
    key: String,
    value: RespFrame,
}

#[derive(Debug)]
pub struct HSet {
    key: String,
    field: String,
    value: RespFrame,
}

#[derive(Debug)]
pub struct HGet {
    key: String,
    field: String,
}

#[derive(Debug)]
pub struct HGetAll {
    key: String,
}

#[derive(Debug)]
pub struct Unrecognized;

impl TryFrom<RespFrame> for Command {
    type Error = CommandError;
    fn try_from(v: RespFrame) -> Result<Self, Self::Error> {
        match v {
            RespFrame::Array(array) => array.try_into(),
            _ => Err(CommandError::InvalidCommand(
                "Command must be an Array".to_string(),
            )),
        }
    }
}

impl TryFrom<RespArray> for Command {
    type Error = CommandError;
    fn try_from(v: RespArray) -> Result<Self, Self::Error> {
        match v.first() {
            Some(RespFrame::BulkString(ref cmd)) => match cmd.as_ref() {
                b"get" => Ok(Get::try_from(v)?.into()),
                b"set" => Ok(Set::try_from(v)?.into()),
                b"hget" => Ok(HGet::try_from(v)?.into()),
                b"hset" => Ok(HSet::try_from(v)?.into()),
                b"hgetall" => Ok(HGetAll::try_from(v)?.into()),
                _ => Ok(Unrecognized.into()),
            },
            _ => Err(CommandError::InvalidCommand(
                "Command must have a BulkString as the first argument".to_string(),
            )),
        }
    }
}

impl CommandExecutor for Unrecognized {
    fn execute(self, _: &Backend) -> RespFrame {
        RESP_OK.clone()
    }
}

fn validator_command(
    arr: &RespArray,
    names: &[&'static str],
    n_args: usize,
) -> Result<(), CommandError> {
    //test argument must have 2 elements
    if arr.len() != n_args + names.len() {
        return Err(CommandError::InvalidArgument(format!(
            "{} command must have {} arguments",
            names.join(" "),
            n_args
        )));
    }

    for (i, name) in names.iter().enumerate() {
        //test if first element is a BulkString
        match arr[i] {
            RespFrame::BulkString(ref cmd) => {
                if cmd.as_ref().to_ascii_lowercase() != name.as_bytes() {
                    return Err(CommandError::InvalidCommand(format!(
                        "Invalid Command: expects {}, get {}",
                        name,
                        String::from_utf8_lossy(cmd.as_ref())
                    )));
                }
            }
            _ => {
                return Err(CommandError::InvalidCommand(
                    "Get command must have GET as first argument".to_string(),
                ))
            }
        }
    }
    Ok(())
}

fn extract_args(arr: RespArray, start: usize) -> Result<Vec<RespFrame>, CommandError> {
    Ok(arr.0.into_iter().skip(start).collect::<Vec<RespFrame>>())
}
