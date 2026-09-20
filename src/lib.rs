use crossbeam::channel::{Receiver, Sender};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    task::JoinHandle,
};
use tracing::{debug, info, instrument};

use crate::{
    config::Config,
    data::{IBData, parse_ib_bytes},
    message::IBMessage,
};

mod config;
mod data;
mod message;
pub mod ui;

pub struct IBConnector {
    pub handle: JoinHandle<eyre::Result<()>>,
}

impl IBConnector {
    #[instrument(skip_all, name = "ib_connector")]
    pub fn new(config: &Config, data_tx: Sender<IBData>, msg_rx: Receiver<IBMessage>) -> Self {
        let port = config.ib_port;
        let client_id = config.client_id;
        let handle = tokio::spawn(async move {
            let mut data = IBData::default();

            let (mut reader, mut writer) = connect_to_ibkr(port).await?;

            writer.write_all(&IBMessage::handshake()).await?;

            data.handshake = Some(false);

            let handshake_payload = read_message_from_ibkr(&mut reader).await?;

            let handshake_msg = parse_message(&handshake_payload);

            debug!(?handshake_msg, "Handshake complete");

            data.handshake = Some(true);

            send_message_to_ibkr(&mut writer, IBMessage::start_api_bytes(client_id)).await?;

            data.start_api = Some(false);

            loop {
                let payload = read_message_from_ibkr(&mut reader).await?;
                let fields = parse_message(&payload)?;

                info!(?fields, "API searching");

                if fields.first() == Some(&"9") {
                    info!("API is accepted");
                    data.start_api = Some(true);
                    break;
                }
            }

            tokio::spawn(async move {
                loop {
                    let msg_rx = msg_rx.clone();
                    let payload = tokio::task::spawn_blocking(move || msg_rx.recv().unwrap())
                        .await
                        .unwrap();

                    send_message_to_ibkr(&mut writer, payload.into_bytes())
                        .await
                        .unwrap();
                }
            });

            tokio::spawn(async move {
                loop {
                    let payload = read_message_from_ibkr(&mut reader).await.unwrap();

                    parse_ib_bytes(payload, &mut data).unwrap();
                    data_tx.send(data.clone()).unwrap();
                }
            });

            Ok::<(), eyre::Report>(())
        });

        Self { handle }
    }

    pub fn join() {}
}

pub async fn connect_to_ibkr(port: u16) -> eyre::Result<(OwnedReadHalf, OwnedWriteHalf)> {
    let (reader, writer) = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await?
        .into_split();

    Ok((reader, writer))
}

pub async fn read_message_from_ibkr(
    reader: &mut tokio::net::tcp::OwnedReadHalf,
) -> eyre::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = i32::from_be_bytes(len_buf) as usize;

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;

    debug!(len, "Message read");
    Ok(payload)
}

pub async fn send_message_to_ibkr(
    writer: &mut OwnedWriteHalf,
    payload: Vec<u8>,
) -> eyre::Result<()> {
    let len = payload.len() as u32;
    let byte_len = len.to_be_bytes();

    writer.write_all(&byte_len).await?;

    writer.write_all(&payload).await?;
    debug!(len, "Message sent");

    Ok(())
}

pub fn parse_message(byte_msg: &[u8]) -> eyre::Result<Vec<&str>> {
    byte_msg
        .split(|b| *b == 0)
        .filter(|x| !x.is_empty())
        .map(|a| std::str::from_utf8(a).map_err(Into::into))
        .collect::<eyre::Result<Vec<_>>>()
}
