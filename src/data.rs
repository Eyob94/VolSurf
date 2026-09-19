use tracing::{debug, instrument};

#[derive(Debug, Clone, Default)]
pub struct IBData {
    pub handshake: Option<bool>,
    pub start_api: Option<bool>,
}

#[instrument(skip(raw_ib_response))]
pub fn parse_ib_bytes(raw_ib_response: Vec<u8>, data: &mut IBData) -> eyre::Result<()> {
    debug!(?raw_ib_response, "Raw IB Response");
    Ok(())
}
