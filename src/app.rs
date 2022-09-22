use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use crossbeam::channel::{Receiver, unbounded};
use crossterm::event;
use crossterm::event::{Event, KeyCode};
use tui::backend::Backend;
use tui::Terminal;
use crate::{Ampq, Config};
use crate::ui::{EditType, Ui};
use anyhow::Result;
use crate::file_logger::FileLogger;
use crate::models::enums::SelectedState;
use crate::models::read_value::ReadValue;

#[derive(PartialEq)]
pub enum Windows {
    Main,
    Options,
    OptionsStringInput,
    SelectionFilter,
    MultiSelectInput
}

#[derive(PartialEq)]
pub enum Mode {
    Normal,
    Scroll
}

pub struct App {
    pub console_logs: Vec<String>,
    pub active_window: Windows,
    pub mode: Mode,
    pub message_receiver: Receiver<ReadValue>,
    pub file_logger: FileLogger,
    pub config: Config,
    pub selection_filter: String,

    console_log_receiver: Receiver<String>,
    ampq: Ampq,
    tick_rate: u64,
}

impl App {
    pub fn new(config: Config) -> Result<App> {
        let (console_log_sender, console_log_receiver) = unbounded();
        let (message_sender, message_receiver) = unbounded();

        let ampq = Ampq::new(&config, console_log_sender.clone(), message_sender.clone())?;
        let file_logger = FileLogger::new(console_log_sender.clone());

        Ok(
            App {
                config,
                console_logs: vec![],
                console_log_receiver,
                message_receiver,
                ampq,
                active_window: Windows::Main,
                tick_rate: 100,
                file_logger,
                mode: Mode::Normal,
                selection_filter: "".to_string()
            }
        )
    }

    pub fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let tick_rate = Duration::from_millis(self.tick_rate);

        let mut ui = Ui::new();

        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|frame| ui.draw(frame, self))?;

            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if crossterm::event::poll(timeout)? {
                let exit = self.handle_input(&mut ui)?;
                if exit {
                    return Ok(());
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.on_tick();
                last_tick = Instant::now();
            }
        }
    }

    fn handle_input(&mut self, ui: &mut Ui) -> Result<bool> {
        if let Event::Key(key) = event::read()? {
            match self.active_window {
                Windows::Main => {
                    match self.mode {
                        Mode::Normal => {
                            match key.code {
                                KeyCode::Char('q') => {
                                    self.exit();
                                    return Ok(true);
                                },
                                KeyCode::Char('k') => ui.main_index_up(), //VIM binding
                                KeyCode::Char('j') => ui.main_index_down(), //VIM binding
                                KeyCode::Esc => {
                                    self.exit();
                                    return Ok(true);
                                },
                                KeyCode::Up => ui.main_index_up(),
                                KeyCode::Down => ui.main_index_down(),
                                KeyCode::Left => ui.left_resize(),
                                KeyCode::Right => ui.right_resize(),
                                KeyCode::Char('l') => ui.show_logs = !ui.show_logs,
                                KeyCode::Enter => {
                                    let selected_id = ui.get_selected_item_id();
                                    match &mut self.config.items
                                        .iter_mut()
                                        .find(|x| x.id == selected_id) {
                                        None => self.console_logs.push(format!("Cannot find selected item in config.items")),
                                        Some(selected_item) => {
                                            if selected_item.selected_state == SelectedState::Unselected {
                                                selected_item.selected_state = SelectedState::PendingSubscription;
                                            }
                                            else if selected_item.selected_state == SelectedState::Subscribed {
                                                selected_item.selected_state = SelectedState::Unselected;
                                            }
                                            else if selected_item.selected_state == SelectedState::PendingSubscription {
                                                selected_item.selected_state = SelectedState::Unselected;
                                                match self.ampq.create_channel() {
                                                    Ok(channel) => {
                                                        let queue_name = self.ampq.create_queue_name(selected_item.exchange_name.as_str());
                                                        self.ampq.delete_queue(queue_name.as_str(), &channel);
                                                    }
                                                    Err(e) => {
                                                        self.console_logs.push(format!("Error creating channel: {}", e.to_string()));
                                                    }
                                                }
                                            }

                                            self.ampq.change_subscription(&selected_item, selected_id);
                                        }
                                    };
                                },
                                KeyCode::Char('e') => {
                                    self.active_window = Windows::Options;
                                    let selected_id = ui.get_selected_item_id();
                                    match self.config.items
                                        .iter()
                                        .find(|x| x.id == selected_id) {
                                            None => self.console_logs.push(format!("Cannot find selected item in config.items")),
                                            Some(selected_item) => {
                                                ui.show_options_popup( selected_item.clone())
                                            }
                                        };
                                },
                                KeyCode::Char('p') => {
                                    self.mode = Mode::Scroll;
                                    let current = crate::amqp::PAUSE.load(Ordering::SeqCst);
                                    let new_value = !current;
                                    self.console_logs.push(format!("PAUSED: {}", new_value));
                                    crate::amqp::PAUSE.store(new_value, Ordering::SeqCst);
                                },
                                KeyCode::Char('f') => {
                                    self.active_window = Windows::SelectionFilter;
                                }
                                KeyCode::Char('/') => {
                                    self.active_window = Windows::SelectionFilter;
                                }
                                KeyCode::Char('s') => {
                                    match self.config.save_config() {
                                        Ok(_) => self.console_logs.push(format!("Config File Saved: {}", self.config.path.as_str())),
                                        Err(e) => self.console_logs.push(format!("Error Saving Config File: {}", e.to_string()))
                                    }
                                }
                                KeyCode::Char('n') => self.send_publish_to_amqp(ui),
                                KeyCode::Char('P') => self.send_publish_to_amqp(ui),
                                _ => {}
                            }
                        }
                        Mode::Scroll => {
                            match key.code {
                                KeyCode::Char('q') => {
                                    self.exit();
                                    return Ok(true);
                                },
                                KeyCode::Char('k') => ui.scroll_up(), //VIM binding
                                KeyCode::Char('j') => ui.scroll_down(), //VIM binding
                                KeyCode::Esc => {
                                    self.exit();
                                    return Ok(true);
                                },
                                KeyCode::Up => ui.scroll_up(),
                                KeyCode::Down => ui.scroll_down(),
                                KeyCode::PageUp => ui.scroll_up_page(),
                                KeyCode::PageDown => ui.scroll_down_page(),
                                KeyCode::Left => ui.left_resize(),
                                KeyCode::Right => ui.right_resize(),
                                KeyCode::Char('l') => ui.show_logs = !ui.show_logs,
                                KeyCode::Char('p') => {
                                    self.mode = Mode::Normal;
                                    let current = crate::amqp::PAUSE.load(Ordering::SeqCst);
                                    let new_value = !current;
                                    self.console_logs.push(format!("PAUSED: {}", new_value));
                                    crate::amqp::PAUSE.store(new_value, Ordering::SeqCst);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Windows::Options => match key.code {
                    KeyCode::Esc => {
                        self.active_window = Windows::Main;
                        ui.hide_options_popup();
                    }
                    KeyCode::Down => ui.options_index_down(),
                    KeyCode::Up => ui.options_index_up(),
                    KeyCode::Char('j') => ui.options_index_down(), //VIM binding
                    KeyCode::Char('k') => ui.options_index_up(), //VIM binding
                    KeyCode::Char('e') => {
                        match ui.options_change_value() {
                            EditType::None => {}
                            EditType::String(res) => {
                                self.active_window = Windows::OptionsStringInput;
                                ui.show_string_input(res);
                            }
                            EditType::MultiSelect(res) => {
                                self.active_window = Windows::MultiSelectInput;
                                ui.show_multi_select_input(res);
                            }
                        }
                    },
                    KeyCode::Enter => {
                        self.active_window = Windows::Main;
                        let selected_id = ui.get_selected_item_id();
                        match self.config.items
                            .iter_mut()
                            .find(|x| x.id == selected_id) {
                            None => self.console_logs.push(format!("Cannot find selected item in config.items")),
                            Some(selected_item) => {
                                *selected_item = ui.options_exchange.clone();
                                ui.hide_options_popup();
                            }
                        }
                    },
                    _ => {}
                }
                Windows::OptionsStringInput => match key.code {
                    KeyCode::Enter => {
                        self.active_window = Windows::Options;
                        ui.set_option_value_string()
                    },
                    KeyCode::Char(c) => ui.string_input.push(c),
                    KeyCode::Backspace => {
                        ui.string_input.pop();
                    },
                    KeyCode::Esc => {
                        self.active_window = Windows::Options;
                        ui.hide_string_input()
                    },
                    _ => {}
                }
                Windows::SelectionFilter => match key.code {
                    KeyCode::Enter => self.active_window = Windows::Main,
                    KeyCode::Char(c) => self.selection_filter.push(c),
                    KeyCode::Backspace => {
                        self.selection_filter.pop();
                    },
                    KeyCode::Esc => self.active_window = Windows::Main,
                    _ => {}
                }
                Windows::MultiSelectInput => match key.code {
                    KeyCode::Down => ui.multi_select_index_down(),
                    KeyCode::Up => ui.multi_select_index_up(),
                    KeyCode::Char('j') => ui.multi_select_index_down(), //VIM binding
                    KeyCode::Char('k') => ui.multi_select_index_up(), //VIM binding
                    KeyCode::Enter => {
                        ui.multi_select_change_value();
                        self.active_window = Windows::Options
                    },
                    KeyCode::Esc => {
                        ui.hide_multi_select_input();
                        self.active_window = Windows::Options
                    },
                    _ => {}
                }
            }
        }

        Ok(false)
    }

    fn send_publish_to_amqp(&mut self, ui: &mut Ui) {
        let selected_id = ui.get_selected_item_id();
        match self.config.items
            .iter()
            .find(|x| x.id == selected_id) {
            None => self.console_logs.push(format!("Cannot find selected item in config.items")),
            Some(selected_item) => {
                self.ampq.publish(selected_item)
                    .unwrap_or_else(|e| {
                        self.console_logs.push(format!("Error publishing message: {}", e.to_string()));
                    });
            }
        };
    }

    fn on_tick(&mut self) {
        if let Ok(res) = self.console_log_receiver.try_recv() {
            self.console_logs.push(res);
        }

        if crate::amqp::PAUSE.load(Ordering::SeqCst) == false {
            self.file_logger.tick(self.tick_rate);
        }
    }

    fn exit(&mut self) {
        self.ampq.delete_remaining_queue().ok();
        self.config.save_config().ok();
    }
}