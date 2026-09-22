use crossbeam::channel::{Receiver, Sender};
use eyre::bail;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    runtime::Runtime,
    task::JoinHandle,
};
use tracing::{debug, error, info, instrument};

mod actions;
mod message;
mod parser;

pub use actions::*;

use crate::{Config, ibkr::parser::parse_ib_bytes};

pub use message::IBMessage;
pub use parser::IBData;

pub struct IBConnector {
    pub handles: [JoinHandle<eyre::Result<()>>; 2],
}

impl IBConnector {
    #[instrument(skip_all, name = "ib_connector")]
    pub fn new(
        config: &Config,
        data_tx: Sender<IBData>,
        msg_rx: Receiver<IBMessage>,
        rt: &Runtime,
    ) -> eyre::Result<Self> {
        let port = config.ib_port;
        let client_id = config.client_id;
        let (mut reader, mut writer, mut data) = match rt.block_on(async move {
            let mut data = IBData::default();

            let (mut reader, mut writer) = connect_to_ibkr(port).await?;

            writer.write_all(&IBMessage::handshake()).await?;

            data.handshake = Some(false);

            let handshake_payload = read_message_from_ibkr(&mut reader).await?;

            let handshake_msg = parse_message(&handshake_payload)?;

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

            Ok::<_, eyre::Report>((reader, writer, data))
        }) {
            Ok(l) => l,
            Err(e) => {
                error!(?e, "Error setting up ib connector");
                bail!("IB connector failed");
            }
        };

        let reader_handle = rt.spawn(async move {
            loop {
                let msg_rx = msg_rx.clone();
                let payload = tokio::task::spawn_blocking(move || msg_rx.recv().unwrap())
                    .await
                    .unwrap();

                info!(?payload, "Received payload");

                send_message_to_ibkr(&mut writer, payload.into_bytes())
                    .await
                    .unwrap();
            }
        });

        let writer_handle = rt.spawn(async move {
            loop {
                let payload = read_message_from_ibkr(&mut reader).await.unwrap();

                parse_ib_bytes(payload, &mut data).unwrap();
                data_tx.send(data.clone()).unwrap();
            }
        });

        Ok(Self {
            handles: [reader_handle, writer_handle],
        })
    }

    pub fn join(self, rt: &Runtime) {
        rt.spawn(async move {
            for handle in self.handles {
                if let Err(e) = handle.await {
                    error!(?e, "Error awaiting on ib connector");
                }
            }
        });
    }
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
