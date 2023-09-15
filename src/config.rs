use std::fs;
use std::fs::{File};
use std::path::Path;
use serde_derive::{Deserialize, Serialize};
use anyhow::{Result, Context, anyhow};
use bevy_reflect::Uuid;
use crate::models::enums::SelectedState;
use crate::models::exchange_options::{ExchangeOptions, ExchangeOptionsSer};

#[derive(Deserialize, Serialize, Debug, Default)]
struct ConfigSer {
    pub host: String,
    pub port: u64,
    pub username: String,
    pub password: String,
    pub pfx_path: Option<String>,
    pub pem_file: Option<String>,
    pub domain: Option<String>,
    pub items: Vec<ExchangeOptionsSer>,
    pub protocol: Option<String>,
}

pub struct Config {
    pub host: String,
    pub port: u64,
    pub username: String,
    pub password: String,
    pub pfx_path: Option<String>,
    pub pem_file: Option<String>,
    pub domain: Option<String>,
    pub items: Vec<ExchangeOptions>,
    pub path: String,
    pub protocol: String,
}

impl Config {
    pub fn read_config(file_path: &Path) -> Result<Config> {
        if file_path.exists() && file_path.is_file() {
            let file_str = fs::read_to_string(file_path)?;

            if file_str.len() > 0 {
                let config_ser: ConfigSer = serde_json::from_str(file_str.as_ref()).with_context(|| "parsing qamqp-client-cli.json")?;

                let mut exchanges: Vec<ExchangeOptions> = vec![];

                for exchange_ser in config_ser.items {
                    let exchange = ExchangeOptions {
                        id: Uuid::new_v4(),
                        exchange_name: exchange_ser.exchange_name,
                        exchange_type: exchange_ser.exchange_type.clone(),
                        queue_routing_key: exchange_ser.queue_routing_key.unwrap_or_default(),
                        alias: exchange_ser.alias.unwrap_or_default(),
                        pretty: exchange_ser.pretty.unwrap_or_default(),
                        log_file: exchange_ser.log_file.unwrap_or_default(),
                        publish_file: exchange_ser.publish_file.unwrap_or_default(),
                        selected_state: SelectedState::Unselected
                    };

                    exchanges.push(exchange);
                }

                let config = Config {
                    host: config_ser.host,
                    port: config_ser.port,
                    username: config_ser.username,
                    password: config_ser.password,
                    pfx_path: config_ser.pfx_path,
                    pem_file: config_ser.pem_file,
                    domain: config_ser.domain,
                    items: exchanges,
                    path: file_path.to_string_lossy().to_string(),
                    protocol: config_ser.protocol.unwrap_or("amqp".to_owned())
                };

                return Ok(config);
            }
        }

        return Err(anyhow!("Cannot read config file: {:?}", file_path));
    }
    
    pub fn save_config(&self) -> Result<()> {
        let path = Path::new(self.path.as_str());
        
        if path.exists() == false {
            File::create(path)?;
        }
        
        let mut exchanges_ser: Vec<ExchangeOptionsSer> = vec![];

        for item in self.items.iter() {
            let mut alias = None;
            if item.alias.len() > 0 {
                alias = Some(item.alias.clone())
            }

            let pretty;
            match item.pretty {
                true => pretty = Some(true),
                false => pretty = Some(false)
            }

            let mut log_file = None;
            if item.log_file.len() > 0 {
                log_file = Some(item.log_file.clone());
            }

            let mut publish_file = None;
            if item.publish_file.len() > 0 {
                publish_file = Some(item.publish_file.clone());
            }

            let mut queue_routing_key = None;
            if item.queue_routing_key.len() > 0 {
                queue_routing_key = Some(item.queue_routing_key.clone());
            }

            exchanges_ser.push(ExchangeOptionsSer {
                exchange_name: item.exchange_name.clone(),
                exchange_type: item.exchange_type.clone(),
                queue_routing_key,
                alias,
                pretty,
                log_file,
                publish_file
            });
        }
        
        let config_ser = ConfigSer {
            host: self.host.clone(),
            port: self.port,
            username: self.username.clone(),
            password: self.password.clone(),
            pfx_path: self.pfx_path.clone(),
            pem_file: self.pem_file.clone(),
            domain: self.domain.clone(),
            items: exchanges_ser,
            protocol: self.protocol.clone().into(),
        };
        
        let json = serde_json::to_string_pretty(&config_ser)?;

        fs::remove_file(self.path.as_str())?;
        fs::write(self.path.as_str(), json)?;

        Ok(())
    }
}