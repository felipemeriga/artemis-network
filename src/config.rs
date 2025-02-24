use serde::{Deserialize, Serialize};
use serde_yaml::Error;
use std::fs::File;
use std::io::Read;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub tcp_address: String,
    pub http_address: String,
    pub bootstrap_address: Option<String>,
    pub node_id: String,
    pub miner_wallet_address: String,
}

pub fn load_config(file_path: &str) -> Result<Config, Error> {
    let mut file = File::open(file_path).expect("Failed to open configuration file.");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read configuration file.");

    serde_yaml::from_str(&contents)
}
