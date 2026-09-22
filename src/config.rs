use clap::Parser;

#[derive(Debug, Parser)]
pub struct Config {
    #[arg(short, long, default_value = "4001", env = "IB_PORT")]
    pub ib_port: u16,

    #[arg(short, long, default_value = "5", env = "CLIENT_ID")]
    pub client_id: u16,
}
