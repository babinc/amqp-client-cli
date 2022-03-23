extern crate core;

use std::{env, io};
use std::io::Stdout;
use std::path::{Path, PathBuf};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::{CrosstermBackend},
    Terminal,
};
use crate::amqp::Ampq;
use crate::app::App;
use anyhow::Result;
use directories::BaseDirs;
use crate::config::Config;

mod app;
mod models;
mod ui;
mod amqp;
mod config;
mod theme;
mod file_logger;

fn main() -> Result<()> {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");

    println!("{} v{}", name, version);
    println!();

    let path_to_config = find_config_file();

    if let Some(res) = path_to_config {
        let config = Config::read_config(Path::new(&res))?;

        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut app = match App::new(config) {
            Ok(res) => res,
            Err(e) => {
                cleanup(&mut terminal)?;
                eprintln!("App Error: {}", e.to_string());
                return Ok(());
            }
        };

        match app.run_app(&mut terminal) {
            Ok(_) => {}
            Err(e) => {
                cleanup(&mut terminal)?;
                eprintln!("App Run Error: {}", e.to_string());
                return Ok(());
            }
        }

        // restore terminal
        cleanup(&mut terminal)?;
    }

    Ok(())
}

fn find_config_file() -> Option<String> {
    println!("Looking for configuration file.");
    let args: Vec<String> = env::args().collect();

    //paths to look for amqp-client-cli.json
    let mut paths_to_look: Vec<String> = vec![];

    //add argument path
    if args.len() > 1 {
        paths_to_look.push(args[1].to_string());
    }

    let file_name = "amqp-client-cli.json";

    //local execution path
    paths_to_look.push(file_name.to_string());

    //add OS paths
    if let Some(base_dirs) = BaseDirs::new() {
        // Linux:   /home/alice/.config
        // Windows: C:\Users\Alice\AppData\Roaming
        // macOS:   /Users/Alice/Library/Application Support
        let dir = base_dirs.config_dir().to_string_lossy().to_string();
        let full_path: PathBuf = [dir, file_name.to_string()].iter().collect();
        paths_to_look.push(full_path.to_string_lossy().to_string());


        // Linux:   /home/alice
        // Windows: C:\Users\Alice
        // macOS:   /Users/Alice
        let dir = base_dirs.home_dir().to_string_lossy().to_string();
        let full_path: PathBuf = [dir, file_name.to_string()].iter().collect();
        paths_to_look.push(full_path.to_string_lossy().to_string());
    }

    for path in paths_to_look.iter().map(|x| Path::new(x)) {
        if path.exists() && path.is_file() {
            println!("  Configuration file found at: {}", path.to_string_lossy().to_string());
            return Some(path.to_string_lossy().to_string());
        }
        else {
            eprintln!("  Configuration file not found at: {}", path.to_string_lossy());
        }
    }

    None
}

fn cleanup(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> std::io::Result<()> {
    disable_raw_mode()?;
    execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
    terminal.show_cursor()
}