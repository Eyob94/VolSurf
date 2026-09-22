use serde::Serialize;
use strum_macros::Display;

#[derive(Debug, Clone, Default)]
pub struct IBMessage {
    id: IBKRMessageID,
    version: Option<u32>,
    fields: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Contract {
    pub con_id: Option<String>,
    pub symbol: String,
    pub sec_type: String,
    pub exchange: String,
    pub primary_exchange: Option<String>,
    pub currency: Currency,
    pub last_trade_date_or_contract_month: String,
    pub strike: Option<u32>,
    pub right: Option<OptionSide>,
    pub multiplier: Option<u32>,
    pub local_symbol: Option<String>,
    pub trading_class: Option<String>,
}

impl Contract {
    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = symbol.into();
        self
    }

    pub fn with_sec_type(mut self, sec_type: impl Into<String>) -> Self {
        self.sec_type = sec_type.into();
        self
    }

    pub fn with_exchange(mut self, exchange: impl Into<String>) -> Self {
        self.exchange = exchange.into();
        self
    }

    pub fn with_primary_exchange(mut self, primary_exchange: impl Into<String>) -> Self {
        self.primary_exchange = Some(primary_exchange.into());
        self
    }

    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    pub fn with_con_id(mut self, con_id: impl Into<String>) -> Self {
        self.con_id = Some(con_id.into());
        self
    }

    pub fn with_last_trade_date_or_contract_month(mut self, date: impl Into<String>) -> Self {
        self.last_trade_date_or_contract_month = date.into();
        self
    }

    pub fn with_strike(mut self, strike: u32) -> Self {
        self.strike = Some(strike);
        self
    }

    pub fn with_right(mut self, right: OptionSide) -> Self {
        self.right = Some(right);
        self
    }

    pub fn with_multiplier(mut self, multiplier: u32) -> Self {
        self.multiplier = Some(multiplier);
        self
    }

    pub fn with_local_symbol(mut self, local_symbol: impl Into<String>) -> Self {
        self.local_symbol = Some(local_symbol.into());
        self
    }

    pub fn with_trading_class(mut self, trading_class: impl Into<String>) -> Self {
        self.trading_class = Some(trading_class.into());
        self
    }
}

#[derive(Debug, Clone, Display, Eq, Hash, PartialEq, Default, Serialize)]
pub enum OptionSide {
    #[default]
    #[strum(serialize = "C")]
    Call,
    #[strum(serialize = "P")]
    Put,
}

#[derive(Debug, Clone, Display, Default)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Currency {
    #[default]
    Usd,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub enum IBKRMessageID {
    #[default]
    StartApi,
    ReqSecDefOptParams,
    ReqContractDetails,
    CancelMktData,
    ReqMarketDataType,
    ReqMktData,
}

impl IBKRMessageID {
    pub fn into_wire_id(self) -> u32 {
        match self {
            Self::StartApi => 71,
            Self::ReqMktData => 1,

            Self::CancelMktData => 2,
            Self::ReqSecDefOptParams => 78,
            Self::ReqContractDetails => 9,
            Self::ReqMarketDataType => 59,
        }
    }
}

fn push_field(buf: &mut Vec<u8>, value: impl ToString) {
    buf.extend_from_slice(value.to_string().as_bytes());
    buf.push(0);
}

impl IBMessage {
    pub fn contract(mut self, contract: Contract) -> Self {
        let fields = &mut self.fields;
        fields.push(contract.con_id.unwrap_or_default());
        fields.push(contract.symbol);
        fields.push(contract.sec_type);
        fields.push(contract.last_trade_date_or_contract_month);
        fields.push(contract.strike.map(|s| s.to_string()).unwrap_or_default());
        fields.push(contract.right.map(|s| s.to_string()).unwrap_or_default());
        fields.push(
            contract
                .multiplier
                .map(|s| s.to_string())
                .unwrap_or_default(),
        );
        fields.push(contract.exchange);
        fields.push(contract.primary_exchange.unwrap_or_default());
        fields.push(contract.currency.to_string());
        fields.push(contract.local_symbol.unwrap_or_default());
        fields.push(contract.trading_class.unwrap_or_default());

        self
    }

    pub fn with_id(mut self, id: IBKRMessageID) -> Self {
        self.id = id;
        self
    }

    pub fn with_version(mut self, version: u32) -> Self {
        self.version = Some(version);
        self
    }
    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.fields.push(field.into());
        self
    }

    pub fn start_api_bytes(client_id: u16) -> Vec<u8> {
        let mut payload = vec![];
        push_field(&mut payload, IBKRMessageID::StartApi.into_wire_id());
        push_field(&mut payload, 2);
        push_field(&mut payload, client_id);
        push_field(&mut payload, "");
        payload
    }

    pub fn handshake() -> Vec<u8> {
        let version_range = b"v100..176";
        let mut msg = Vec::with_capacity(4 + 4 + version_range.len());
        msg.extend_from_slice(b"API\0");
        msg.extend_from_slice(&(version_range.len() as i32).to_be_bytes());
        msg.extend_from_slice(version_range);
        msg
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut payload = vec![];
        push_field(&mut payload, self.id.into_wire_id());
        if let Some(version) = self.version {
            push_field(&mut payload, version);
        }
        for other in self.fields {
            push_field(&mut payload, other);
        }
        payload
    }
}
