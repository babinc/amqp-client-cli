use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use crossbeam::channel::Sender;
use std::io::Write;
use std::path::Path;
use anyhow::{Result};

pub struct FileLogger {
    wait_time_ms: u64,
    since_last_write_ms: u64,
    file_paths: HashMap<String, Vec<String>>,
    console_log_sender: Sender<String>
}

impl FileLogger {
    pub fn new(console_log_sender: Sender<String>) -> Self {
        FileLogger {
            wait_time_ms: 1000,
            since_last_write_ms: 0,
            file_paths: Default::default(),
            console_log_sender,
        }
    }

    pub fn add_to_buffer(&mut self, file_path: String, line: &str) {
        if self.file_paths.contains_key(&file_path) {
            let key_value = self.file_paths.get_mut(&file_path).unwrap();
            key_value.push(line.to_string());
        }
        else {
            self.file_paths.insert(file_path, vec![line.to_string()]);
        }
    }

    fn write(&mut self) -> Result<()> {
        for file_data in self.file_paths.iter() {
            if Path::new(file_data.0).exists() == false {
                File::create(file_data.0)?;
            }

            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .open(file_data.0)?;

            for line in file_data.1 {
                writeln!(file, "{}", line.to_string())?;
            }
        }

        self.file_paths.clear();

        self.since_last_write_ms = 0;

        Ok(())
    }

    pub fn tick(&mut self, tick_rate: u64) {
        self.since_last_write_ms += tick_rate;
        if self.since_last_write_ms > self.wait_time_ms {
            if let Err(e) = self.write() {
                self.console_log_sender.send(format!("Error logging to file: {}", e.to_string())).unwrap();
            }
        }
    }
}