use std::collections::HashMap;
use std::fs::File;
use std::io::{Read};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use amiquip::{Auth, Channel, Connection, ConnectionOptions, ConnectionTuning, ConsumerMessage, ConsumerOptions, ExchangeDeclareOptions, ExchangeType, FieldTable, QueueDeclareOptions, QueueDeleteOptions};
use chrono::{Local};
use crossbeam::channel::{Sender, unbounded};
use native_tls::{Certificate, Identity, TlsConnector};
use anyhow::{Result, Context, anyhow};
use bevy_reflect::Uuid;
use crate::Config;
use crate::models::enums::ExchangeTypeSer;
use crate::models::exchange_options::{ExchangeOptions};
use crate::models::read_value::ReadValue;

pub static PAUSE: AtomicBool = AtomicBool::new(false);

pub struct Ampq {
    current_subscriptions: HashMap<String, Sender<()>>,
    connection: Connection,
    log_sender: Sender<String>,
    message_sender: Sender<ReadValue>,
    queue_names: Vec<String>
}

impl Ampq {
    pub fn new(config: &Config, console_log_sender: Sender<String>, message_sender: Sender<ReadValue>) -> Result<Self> {
        let connection;
        if config.pfx_path.is_some() && config.pem_file.is_some() {
            let pfx_path_ref = config.pfx_path.as_ref().with_context(|| format!("Failed to convert pfx_path to reference"))?;
            let identity = get_identity(pfx_path_ref.as_str())?;

            let pem_file_ref = config.pem_file.as_ref().with_context(|| format!("Failed to convert pem_file_path to reference"))?;
            let cert = get_certificate(pem_file_ref.as_str())?;

            let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::from_str(config.host.as_str())?), config.port as u16);

            let stream = mio::net::TcpStream::connect(&socket_addr)?;

            let tls_connector = TlsConnector::builder()
                .identity(identity)
                .add_root_certificate(cert)
                .build()?;

            let domain_ref = config.domain.as_ref().with_context(|| format!("Failed to convert domain to reference"))?;

            connection = Connection::open_tls_stream(
                tls_connector,
               domain_ref.as_str(),
                stream,
                ConnectionOptions::default()
                    .auth(Auth::Plain {
                        username: config.username.clone(),
                        password: config.password.clone()
                    })
                    .heartbeat(30)
                    .channel_max(1024)
                    .connection_timeout(Some(Duration::from_millis(10_000))),
                ConnectionTuning::default())
                .with_context(|| "connecting to host")?;

            console_log_sender.send(format!("Secure connection to: {}:{}", config.host, config.port))?;
        }
        else {
            let connection_string = format!("amqp://{}:{}@{}:{}", config.username, config.password, config.host, config.port);
            connection = Connection::insecure_open(connection_string.as_str())?;

            console_log_sender.send(format!("Connected to: {}:{}", config.host, config.port))?;
        }

        Ok(
            Ampq {
                message_sender,
                connection,
                log_sender: console_log_sender,
                current_subscriptions: HashMap::new(),
                queue_names: vec![]
            }
        )
    }

    pub fn add_subscription(&mut self, exchange_name: String, exchange_type: ExchangeType, queue_routing_key: String, selected_id: Uuid) -> Result<()> {
        let thread_sender = self.message_sender.clone();
        let thread_channel = self.create_channel()?;
        let thread_log_sender = self.log_sender.clone();
        let queue_name = self.create_queue_name(exchange_name.as_str());

        if self.queue_names.contains(&queue_name) == false {
            self.queue_names.push(queue_name.clone());
        }

        let (sender, receiver) = unbounded();

        self.current_subscriptions.insert(exchange_name.clone(), sender);

        thread_log_sender.send(format!("Channel created: {}", thread_channel.channel_id())).ok();

        thread::spawn(move || {
            let exchange_declare_options = ExchangeDeclareOptions {
                durable: true,
                auto_delete: false,
                internal: false,
                arguments: Default::default()
            };

            let exchange = match thread_channel.exchange_declare(exchange_type, exchange_name.clone(), exchange_declare_options) {
                Ok(res) => res,
                Err(err) => {
                    thread_log_sender.send(format!("Exchange error: {}", err.to_string())).ok();
                    return;
                }
            };

            match thread_channel.queue_declare(queue_name.clone(), QueueDeclareOptions { exclusive: false, ..QueueDeclareOptions::default() }) {
                Ok(queue) => {
                    thread_log_sender.send(format!("Queue Created: {}", queue_name.clone())).ok();

                    queue.bind(&exchange, queue_routing_key, FieldTable::new()).unwrap_or_else(|e| {
                        thread_log_sender.send(format!("Error binding to queue: {}", e.to_string())).ok();
                    });

                    match queue.consume(ConsumerOptions { no_ack: true, ..ConsumerOptions::default() }) {
                        Ok(consumer) => {
                            loop {
                                if let Ok(_) = receiver.try_recv() {
                                    match queue.delete(QueueDeleteOptions::default()) {
                                        Ok(_) => {
                                            thread_log_sender.send(format!("Queue Deleted: {}", queue_name)).ok();
                                        },
                                        Err(e) => {
                                            thread_log_sender.send(format!("Error deleting queue: {}", e.to_string())).ok();
                                        }
                                    }
                                    break;
                                }

                                let consumer_message = consumer.receiver().recv();
                                if let Ok(message) = consumer_message {
                                    match message {
                                        ConsumerMessage::Delivery(delivery) => {
                                            if PAUSE.load(Ordering::SeqCst) == false {
                                                let body = String::from_utf8_lossy(&delivery.body);

                                                let now = Local::now();

                                                thread_sender.send(ReadValue {
                                                    id: selected_id,
                                                    exchange_name: exchange_name.clone(),
                                                    value: body.to_string(),
                                                    timestamp: now.clone()
                                                }).ok();
                                            }
                                        }
                                        _ => {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            thread_log_sender.send(format!("Error creating consumer: {}", e.to_string())).ok();
                        }
                    };
                }
                Err(e) => {
                    thread_log_sender.send(format!("Error deleting queue: {}", e.to_string())).ok();
                }
            };

            thread_log_sender.send(format!("Unsubscribed from: {}", exchange_name)).ok();
        });

        Ok(())
    }

    pub fn change_subscription(&mut self, exchange_options: &ExchangeOptions, selected_id: Uuid) {
        match self.current_subscriptions.get(exchange_options.exchange_name.as_str()) {
            None => {
                //TODO implement from/into to let the enum itself handle this conversion
                let exchange_type = match &exchange_options.exchange_type {
                    ExchangeTypeSer::Direct => ExchangeType::Direct,
                    ExchangeTypeSer::Fanout => ExchangeType::Fanout,
                    ExchangeTypeSer::Topic => ExchangeType::Topic,
                    ExchangeTypeSer::Headers => ExchangeType::Headers,
                };
                self.add_subscription(exchange_options.exchange_name.clone(), exchange_type, exchange_options.queue_routing_key.clone(), selected_id).ok();
            }
            Some(unsubscribe_sender) => {
                unsubscribe_sender.send(()).ok();
                self.log_sender.send(format!("Unsubscribing from: {}", exchange_options.exchange_name.clone())).ok();
                self.current_subscriptions.remove(&exchange_options.exchange_name);
            }
        };
    }

    pub fn create_queue_name(&self, exchange_name: &str) -> String {
        format!("{}.{}", env!("CARGO_PKG_NAME"), exchange_name)
    }

    pub fn create_channel(&mut self) -> Result<Channel> {
        let channel = self.connection.open_channel(None)?;
        Ok(channel)
    }

    pub fn delete_remaining_queue(&mut self) -> Result<()> {
        let channel = self.create_channel()?;
        for queue_name in self.queue_names.iter() {
            self.delete_queue(queue_name.as_str(), &channel);
        }

        Ok(())
    }

    pub fn delete_queue(&self, queue_name: &str, channel: &Channel) {
        match channel.queue_delete(queue_name, QueueDeleteOptions::default()) {
            Ok(_) => self.log_sender.send(format!("Queue Deleted: {}", queue_name)).ok(),
            Err(e) => self.log_sender.send(format!("Error deleting queue: {}, error: {}", queue_name, e.to_string())).ok()
        };
    }
}

fn get_certificate(pem_file_path: &str) -> Result<Certificate> {
    let path = Path::new(pem_file_path);
    if path.exists() {
        let output = Command::new("openssl")
            .arg("x509")
            .arg("-in")
            .arg(pem_file_path)
            .arg("-inform")
            .arg("pem")
            .stderr(Stdio::piped())
            .output()
            .with_context(|| "Error with openssl command".to_string())?;

        let cert = Certificate::from_pem(&output.stdout)
            .with_context(|| "Error getting certificate from openssl command output")?;

        Ok(cert)
    }
    else {
        return Err(anyhow!("Pem file path does not exist: {}", pem_file_path));
    }
}

fn get_identity(pfx_path: &str) -> Result<Identity> {
    let mut file = File::open(pfx_path)
        .with_context(|| format!("Error opening pfx file"))?;

    let mut identity = vec![];
    file.read_to_end(&mut identity)
        .with_context(|| format!("Error reading pfx file"))?;

    let identity = Identity::from_pkcs12(&identity, "")
        .with_context(|| "Error getting Identity from pfx file".to_string())?;

    Ok(identity)
}
