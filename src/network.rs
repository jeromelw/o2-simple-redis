use crate::{
    cmd::{Command, CommandExecutor},
    Backend, RespDecode, RespEncode, RespError,
};
use anyhow::Result;
use futures::SinkExt;
use tokio::net::TcpStream;
use tokio_stream::StreamExt;
use tracing::info;

use tokio_util::codec::{Decoder, Encoder, Framed};

use crate::RespFrame;

#[derive(Debug)]
struct RespFrameCodec;

impl Encoder<RespFrame> for RespFrameCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: RespFrame, dst: &mut bytes::BytesMut) -> Result<()> {
        let encoded = item.encode();
        dst.extend_from_slice(&encoded);
        Ok(())
    }
}

impl Decoder for RespFrameCodec {
    type Item = RespFrame;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>> {
        match RespFrame::decode(src) {
            Ok(frame) => Ok(Some(frame)),
            Err(RespError::NotComplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Debug)]
struct RedisRequest {
    frame: RespFrame,
    backend: Backend,
}

#[derive(Debug)]
struct RedisResponse {
    frame: RespFrame,
}

pub async fn stream_handler(stream: TcpStream, backend: Backend) -> Result<()> {
    //how to get a frame from a stream
    let mut framed = Framed::new(stream, RespFrameCodec);
    loop {
        match framed.next().await {
            Some(Ok(frame)) => {
                info!("Received frame: {:?}", frame);

                let req = RedisRequest {
                    frame,
                    backend: backend.clone(),
                };
                let res = request_handler(req).await?;
                info!("Sending frame: {:?}", res.frame);
                framed.send(res.frame).await?;
            }
            Some(Err(e)) => return Err(e),
            None => return Ok(()),
        }
    }
}

async fn request_handler(req: RedisRequest) -> Result<RedisResponse> {
    let (frame, backend) = (req.frame, req.backend);
    let cmd: Command = frame.try_into()?;
    info!("Executing command: {:?}", cmd);

    let ret = cmd.execute(&backend);
    Ok(RedisResponse { frame: ret })
}
