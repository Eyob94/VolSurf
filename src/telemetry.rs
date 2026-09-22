use std::fs::File;

use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_tracing() {
    let file = File::options()
        .create(true)
        .truncate(true)
        .write(true)
        .open("ibkr.json")
        .unwrap();
    let json_layer = fmt::layer()
        .json()
        .with_current_span(false)
        .with_span_list(false)
        .with_writer(file);

    let console_layer = fmt::layer();

    tracing_subscriber::registry()
        .with(console_layer)
        .with(json_layer)
        .with(EnvFilter::from_default_env())
        .init();
}
