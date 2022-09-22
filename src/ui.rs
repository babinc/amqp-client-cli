use bevy_reflect::{GetField, Reflect, Uuid};
use serde_json::Value;
use tui::backend::Backend;
use tui::Frame;
use tui::layout::{Alignment, Constraint, Corner, Direction, Layout, Rect};
use tui::style::{Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table};
use crate::{App, theme};
use crate::app::{Mode, Windows};
use crate::models::exchange_options::{ExchangeOptions};
use bevy_reflect::Struct;
use itertools::Itertools;
use crate::file_logger::FileLogger;
use crate::models::enums::{ExchangeTypeSer, SelectedState};

pub enum EditType {
    None,
    String(String),
    MultiSelect(ExchangeTypeSer)
}

pub struct Ui {
    pub options_exchange: ExchangeOptions,
    pub show_logs: bool,
    pub string_input: String,

    selector_index: usize,
    options_window_index: i32,
    left_size: u16,
    right_size: u16,
    show_options: bool,
    show_string_input: bool,
    show_multi_select_input: bool,
    multi_select_index: i32,
    multi_select_list_items: Vec<String>,
    options_exchange_type: ExchangeTypeSer,
    options_count: usize,
    line_buffer: Vec<String>,
    line_buffer_size: usize,
    window_lines: Vec<String>,
    messages_window_height: i32,
    scroll_position: i32,
    messages_upper_scroll: usize,
    messages_lower_scroll: usize,
    selector_length: usize,
    selector_ids: Vec<Uuid>
}

impl Ui {
    pub fn new() -> Self {
        let options_exchange = ExchangeOptions::default();
        let options_count = options_exchange.iter_fields().len() - 1;

        Ui {
            selector_index: 0,
            options_window_index: 0,
            left_size: 25,
            right_size: 75,
            show_options: false,
            show_logs: true,
            options_exchange,
            options_count,
            line_buffer: vec![],
            line_buffer_size: 1000,
            window_lines: vec![],
            messages_window_height: 0,
            show_string_input: false,
            string_input: "".to_string(),
            scroll_position: 0,
            messages_upper_scroll: 0,
            messages_lower_scroll: 0,
            show_multi_select_input: false,
            multi_select_index: 0,
            multi_select_list_items: vec![],
            options_exchange_type: ExchangeTypeSer::Direct,
            selector_length: 0,
            selector_ids: vec![],
        }
    }

    pub fn draw<B: Backend>(&mut self, frame: &mut Frame<B>, app: &mut App) {
        let grid_1_constraints;
        if self.show_logs {
            grid_1_constraints = vec![Constraint::Length(3), Constraint::Percentage(80), Constraint::Percentage(20)];
        }
        else {
            grid_1_constraints = vec![Constraint::Length(3), Constraint::Percentage(100)];
        }

        let vertical_grid = Layout::default()
            .direction(Direction::Vertical)
            .constraints(grid_1_constraints.as_slice())
            .split(frame.size());

        let horizontal_grid = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(self.left_size), Constraint::Percentage(self.right_size)].as_ref())
            .split(vertical_grid[1]);

        self.draw_header(frame, app, &vertical_grid);

        self.draw_selector(frame, app, &horizontal_grid);

        self.draw_messages(frame, app, &horizontal_grid);

        if self.show_logs {
            self.draw_logs(frame, app, &vertical_grid);
        }

        if self.show_options {
            self.draw_options(frame);
        }

        if self.show_string_input {
            self.draw_string_input(frame);
        }

        if self.show_multi_select_input {
            self.draw_multi_select_input(frame);
        }
    }

    fn draw_header<B: Backend>(&mut self, frame: &mut Frame<B>, app: &App, grid: &Vec<Rect>) {
        let content = match app.active_window {
            Windows::Main => {
                match app.mode {
                    Mode::Normal => " ↑ Select | ↓ Select | → Width | ← Width | (Enter) select | (F)ilter | (L)ogs | (E)dit | (P)ause | (S)ave | (n) (Shift+P) Publish Message | (Esc) (Q)uit |",
                    Mode::Scroll => " ↑ Scroll Up | ↓ Scroll Down | (Pg Up) Page Up | (Pg Dn) Page Down | → Width | ← Width | (L)ogs | (P)ause | (Esc) (Q)uit |"
                }
            }
            Windows::Options => " ↑ Select | ↓ Select | (Esc) Close Window | (E)dit Value | (Enter) Apply Changes |",
            Windows::OptionsStringInput => " (Esc) Close Window | (Enter) Change Value |",
            Windows::SelectionFilter => " (Esc) Close Window | (Enter) Change Value |",
            Windows::MultiSelectInput => " ↑ Select | ↓ Select | (Esc) Close Window | (Enter) Change Value |",
        };

        let block = Block::default().borders(Borders::TOP | Borders::BOTTOM);
        let paragraph = Paragraph::new(content)
            .block(block)
            .style(Style::default()
                .fg(theme::FOREGROUND)
                .bg(theme::BACKGROUND)
                .add_modifier(Modifier::BOLD));

        frame.render_widget(paragraph, grid[0]);
    }

    fn draw_selector<B: Backend>(&mut self, frame: &mut Frame<B>, app: &mut App, grid: &Vec<Rect>) {
        let index = self.selector_index;
        let mut count = 0;
        let selection_filter = app.selection_filter.clone();

        let filtered_items: Vec<(&str, SelectedState, Uuid)> = app
            .config
            .items
            .iter()
            .filter(|item| {
                let name: &str;
                if item.alias.len() > 0 {
                    name = item.alias.as_str();
                }
                else {
                    name = item.exchange_name.as_str();
                }

                name.to_lowercase().contains(selection_filter.to_lowercase().as_str())
            })
            .map(|item| {
                let name;
                if item.alias.len() > 0 {
                    name = item.alias.as_str();
                }
                else {
                    name = item.exchange_name.as_str();
                }

                (name, item.selected_state.clone(), item.id)
            })
            .sorted_by(|a, b| a.0.cmp(&b.0))
            .collect();

        self.selector_ids = filtered_items.iter().map(|x| x.2).collect();

        let rows: Vec<Row> = filtered_items
            .iter()
            .map(|item| {
                let row;

                let name_style = match item.1 {
                    SelectedState::Unselected => Style::default().fg(theme::FOREGROUND),
                    SelectedState::PendingSubscription => Style::default().fg(theme::PENDING),
                    SelectedState::Subscribed => Style::default().fg(theme::SELECTED)
                };

                let name_cell = Cell::from(item.0).style(name_style);

                let indicator_cell;
                if count == index {
                    indicator_cell = Cell::from(">").style(Style::default().fg(theme::INPUT));
                }
                else {
                    indicator_cell = Cell::from(" ").style(Style::default().fg(theme::INPUT));
                }

                row = Row::new(vec![indicator_cell, name_cell]);

                count += 1;

                row
            })
            .collect();

        let table_constraints = [Constraint::Length(1), Constraint::Length(grid[0].width - 1)];

        let mut top_len = 0;
        let mut render_filter = false;
        let filter_color = if app.active_window == Windows::SelectionFilter { theme::INPUT } else { theme::FOREGROUND };
        if app.selection_filter.len() > 0 || app.active_window == Windows::SelectionFilter {
            top_len = 3;
            render_filter = true;
        }

        let window_height = (grid[0].height - 2 - top_len) as usize;
        let mut scroll_difference = 0;
        if index >= window_height {
            scroll_difference = index - window_height + 1;
        }

        let upper_scroll;
        let lower_scroll;
        self.selector_length = rows.len();
        if self.selector_length > window_height {
            upper_scroll = scroll_difference;
            lower_scroll = window_height + scroll_difference;
        }
        else {
            upper_scroll = 0;
            lower_scroll = self.selector_length;
        }

        let table = Table::new(rows[upper_scroll..lower_scroll].to_vec())
            .style(Style::default().fg(theme::FOREGROUND).bg(theme::BACKGROUND))
            .block(Block::default().borders(Borders::ALL).title(Span::styled("Selector", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))))
            .widths(&table_constraints)
            .column_spacing(1);

        let grid_constraints = [Constraint::Length(top_len), Constraint::Length(grid[0].height - 1)];

        let vertical_grid = Layout::default()
            .direction(Direction::Vertical)
            .constraints(grid_constraints.as_slice())
            .split(grid[0]);

        let filter_text = Paragraph::new(app.selection_filter.clone())
            .style(Style::default().fg(filter_color))
            .block(Block::default().borders(Borders::ALL).title("Filter"));

        if render_filter {
            frame.render_widget(filter_text, vertical_grid[0]);
        }
        frame.render_widget(table, vertical_grid[1]);
    }

    fn draw_messages<B: Backend>(&mut self, frame: &mut Frame<B>, app: &mut App, grid: &Vec<Rect>) {
        self.messages_window_height = (grid[1].height - 2) as i32;

        if let Ok(read_value) = app.message_receiver.try_recv() {
            let mut selected_item = &mut app.config.items.iter_mut().find(|x| x.id == read_value.id).unwrap();
            if selected_item.selected_state == SelectedState::PendingSubscription {
                selected_item.selected_state = SelectedState::Subscribed;
            }

            let exchange = app.config
                .items
                .iter()
                .find(|x| x.exchange_name == read_value.exchange_name)
                .unwrap();

            let name: &str;
            if exchange.alias.len() > 0 {
                name = exchange.alias.as_str();
            }
            else {
                name = exchange.exchange_name.as_str();
            }

            if exchange.pretty {
                let json_value: Value = serde_json::from_str(&read_value.value).unwrap();
                let pretty_json = serde_json::to_string_pretty(&json_value).unwrap();

                let header_line = "-".repeat(grid[1].width as usize);
                self.add_line(header_line.as_str(), app);

                let time_stamp = read_value.timestamp.format("%Y/%m/%d %I:%M:%S%.6f %p").to_string();
                let time_stamp_name = format!("{} | {}", name.to_string(), time_stamp);
                self.add_line(time_stamp_name.as_str(), app);
                Self::add_log(&mut app.file_logger, exchange.log_file.as_str(), time_stamp_name.as_str());

                for line in pretty_json.split("\n") {
                    self.add_line(line, app);
                    Self::add_log(&mut app.file_logger, exchange.log_file.as_str(), line);
                }
            }
            else {
                let header_line = "-".repeat(grid[1].width as usize).to_string();
                self.add_line(header_line.as_str(), app);

                let time_stamp = read_value.timestamp.format("%Y/%m/%d %I:%M:%S%.6f %p").to_string();
                let time_stamp_name = format!("{} | {}", name.to_string(), time_stamp);
                self.add_line(time_stamp_name.as_str(), app);
                Self::add_log(&mut app.file_logger, exchange.log_file.as_str(), time_stamp_name.as_str());

                let message_line = read_value.value.as_str();
                self.add_line(message_line, app);
                Self::add_log(&mut app.file_logger, exchange.log_file.as_str(), message_line);
            }
        }

        let mut spans: Vec<Spans> = vec![];

        match app.mode {
            Mode::Normal => {
                for message_line in self.window_lines.iter() {
                    spans.push(Spans::from(message_line.to_string()));
                }
            }
            Mode::Scroll => {
                if self.line_buffer.len() > self.messages_window_height as usize {
                    self.messages_upper_scroll = (self.line_buffer.len() - self.messages_window_height as usize) - self.scroll_position as usize;
                    self.messages_lower_scroll = self.line_buffer.len() - self.scroll_position as usize;

                    for message_line in self.line_buffer[self.messages_upper_scroll..self.messages_lower_scroll].iter() {
                        spans.push(Spans::from(message_line.to_string()));
                    }
                }
                else {
                    for message_line in self.line_buffer.iter() {
                        spans.push(Spans::from(message_line.to_string()));
                    }
                }
            }
        }

        let paragraph = Paragraph::new(spans.clone())
            .style(Style::default().bg(theme::BACKGROUND).fg(theme::FOREGROUND))
            .block(Block::default().borders(Borders::ALL).title(Span::styled("Messages", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))))
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, grid[1]);
    }

    fn add_log(file_logger: &mut FileLogger, log_file: &str, line: &str) {
        if log_file.len() > 0 {
            file_logger.add_to_buffer(log_file.to_string(), line);
        }
    }

    fn add_line(&mut self, line: &str, app: &App) {
        if app.mode == Mode::Normal {
            self.window_lines.push(line.to_string());

            //remove what would render past the window
            let window_overflow = (self.window_lines.len() as i32) - self.messages_window_height;
            if window_overflow > 0 {
                for _ in 0..window_overflow {
                    self.window_lines.remove(0);
                }
            }

            if self.line_buffer.len() == self.line_buffer_size {
                self.line_buffer.remove(0);
            }
            self.line_buffer.push(line.to_string());
        }
    }

    fn draw_logs<B: Backend>(&mut self, frame: &mut Frame<B>, app: &mut App, grid: &Vec<Rect>) {
        let items: Vec<ListItem> = app
            .console_logs
            .iter()
            .rev()
            .map(|log| {
                ListItem::new(log.as_ref())
            })
            .collect();

        let logs_list = List::new(items)
            .style(Style::default().bg(theme::BACKGROUND).fg(theme::FOREGROUND))
            .block(Block::default().borders(Borders::ALL).title(Span::styled("Logs", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))))
            .start_corner(Corner::BottomLeft);

        frame.render_widget(logs_list, grid[2]);
    }

    fn draw_options<B: Backend>(&mut self, frame: &mut Frame<B>) {
        let fields = extract_fields_from_struct(&self.options_exchange);
        let option_windows_index = self.options_window_index;

        let mut count = 0;
        let rows: Vec<Row> = fields
            .iter()
            .map(|item| {
                let row;
                if count == option_windows_index {
                    row = Row::new(vec![item.0.to_string(), item.1.to_string()]).style(Style::default().fg(theme::INPUT))
                }
                else {
                    row = Row::new(vec![item.0.to_string(), item.1.to_string()]).style(Style::default().fg(theme::FOREGROUND))
                }
                count += 1;
                return row;
            })
            .collect();

        let width_constraints = [Constraint::Percentage(50), Constraint::Percentage(50)];

        let table = Table::new(rows)
            .style(Style::default().fg(theme::FOREGROUND))
            .block(Block::default().style(Style::default().fg(theme::ACCENT)).borders(Borders::ALL).title("Options"))
            .widths(&width_constraints)
            .column_spacing(1);

        let area = Self::center_rect_absolute(55, 9, frame.size());
        frame.render_widget(Clear, area);
        frame.render_widget(table, area);
    }

    fn draw_string_input<B: Backend>(&self, frame: &mut Frame<B>) {
        let input = Paragraph::new(self.string_input.as_ref())
            .style(Style::default().fg(theme::INPUT))
            .block(Block::default().borders(Borders::ALL).title("Input"));

        let area = Self::center_input(65, frame.size());
        frame.render_widget(Clear, area);
        frame.render_widget(input, area);
    }

    fn draw_multi_select_input<B: Backend>(&mut self, frame: &mut Frame<B>) {
        self.multi_select_list_items.clear();
        for value in ExchangeTypeSer::iterator() {
            self.multi_select_list_items.push(format!("{:?}", value));
        }

        let mut count = 0;
        let mut list_items: Vec<ListItem> = vec![];
        for value in self.multi_select_list_items.iter() {
            let item;
            if count == self.multi_select_index {
                item = ListItem::new(value.clone()).style(Style::default().fg(theme::INPUT));
            }
            else {
                item = ListItem::new(value.clone()).style(Style::default().fg(theme::FOREGROUND));
            }
            list_items.push(item);
            count += 1;
        }

        let input = List::new(list_items)
            .style(Style::default().fg(theme::ACCENT))
            .block(Block::default().borders(Borders::ALL).title("Select"));

        let area = Self::centered_rect(12, 12, frame.size());
        frame.render_widget(Clear, area);
        frame.render_widget(input, area);
    }

    pub fn multi_select_change_value(&mut self) {
        self.show_multi_select_input = false;
        for (i, item) in self.multi_select_list_items.iter().enumerate() {
            if i == self.multi_select_index as usize {
                let selected_item = ExchangeTypeSer::from(item.as_str());
                self.options_exchange.exchange_type = selected_item;
                break;
            }
        }
    }

    pub fn options_change_value(&mut self) -> EditType {
        let exchange_options_clone = self.options_exchange.clone();
        let mut rtn = EditType::None;

        for (i, value) in exchange_options_clone.iter_fields().enumerate() {
            if i == self.options_window_index as usize {
                let name = exchange_options_clone.name_at(i).unwrap().clone();
                if let Some(value) = value.downcast_ref::<bool>() {
                    *self.options_exchange.get_field_mut::<bool>(name).unwrap() = !value;
                }

                if let Some(value) = value.downcast_ref::<String>() {
                   rtn = EditType::String(value.clone());
                }

                if let Some(res) = value.downcast_ref::<ExchangeTypeSer>() {
                    rtn = EditType::MultiSelect(res.clone());
                }
            }
        }

        rtn
    }

    pub fn main_index_down(&mut self) {
        if self.selector_index < self.selector_length - 1 {
            self.selector_index += 1;
        }
    }

    pub fn main_index_up(&mut self) {
        if self.selector_index > 0 {
            self.selector_index -= 1;
        }
    }

    pub fn options_index_down(&mut self) {
        let options_count = self.options_count as i32;
        if self.options_window_index < options_count - 1 {
            self.options_window_index += 1;
        }
    }

    pub fn options_index_up(&mut self) {
        if self.options_window_index > 0 {
            self.options_window_index -= 1;
        }
    }

    pub fn multi_select_index_down(&mut self) {
        let options_count = ExchangeTypeSer::iterator().count();
        if self.multi_select_index < (options_count as i32) - 1 {
            self.multi_select_index += 1;
        }
    }

    pub fn multi_select_index_up(&mut self) {
        if self.multi_select_index > 0 {
            self.multi_select_index -= 1;
        }
    }

    pub fn left_resize(&mut self) {
        if self.left_size > 1 {
            self.left_size -= 1;
            self.right_size += 1;
        }
    }

    pub fn right_resize(&mut self) {
        if self.right_size > 1 {
            self.right_size -= 1;
            self.left_size += 1;
        }
    }

    pub fn show_options_popup(&mut self, exchange: ExchangeOptions) {
        self.options_exchange = exchange;
        self.show_options = true;
    }

    pub fn hide_options_popup(&mut self) {
        self.show_options = false;
    }

    pub fn get_selected_item_id(&mut self) -> Uuid {
        self.selector_ids[self.selector_index]
    }

    pub fn show_string_input(&mut self, edit_string: String) {
        self.show_string_input = true;
        self.string_input = edit_string;
    }

    pub fn show_multi_select_input(&mut self, exchange_type: ExchangeTypeSer) {
        self.show_multi_select_input = true;
        self.options_exchange_type = exchange_type;
    }

    pub fn hide_multi_select_input(&mut self) {
        self.show_multi_select_input = false;
    }

    pub fn hide_string_input(&mut self) {
        self.show_string_input = false;
        self.string_input = "".to_string();
    }

    pub fn set_option_value_string(&mut self) {
        self.show_string_input = false;

        let exchange_options_clone = self.options_exchange.clone();

        for (i, value) in exchange_options_clone.iter_fields().enumerate() {
            if i == self.options_window_index as usize {
                let name = exchange_options_clone.name_at(i).unwrap().clone();
                if let Some(_) = value.downcast_ref::<String>() {
                    *self.options_exchange.get_field_mut::<String>(name).unwrap() = self.string_input.clone();
                }
            }
        }

        self.string_input = "".to_string();
    }

    pub fn scroll_up(&mut self) {
        if self.messages_upper_scroll >= 1 {
            self.scroll_position += 1;
        }
    }

    pub fn scroll_up_page(&mut self) {
        if self.messages_upper_scroll >= self.messages_window_height as usize {
            self.scroll_position += self.messages_window_height;
        }
        else {
            self.scroll_position = self.line_buffer.len() as i32 - self.messages_window_height;
        }
    }

    pub fn scroll_down(&mut self) {
        if self.messages_lower_scroll < self.line_buffer.len() {
            self.scroll_position -= 1;
        }
    }

    pub fn scroll_down_page(&mut self) {
        if self.messages_lower_scroll < self.line_buffer.len() - self.messages_window_height as usize {
            self.scroll_position -= self.messages_window_height;
        }
        else {
            self.scroll_position = 0;
        }
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_y) / 2),
                    Constraint::Percentage(percent_y),
                    Constraint::Percentage((100 - percent_y) / 2),
                ]
                    .as_ref(),
            )
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_x) / 2),
                    Constraint::Percentage(percent_x),
                    Constraint::Percentage((100 - percent_x) / 2),
                ]
                    .as_ref(),
            )
            .split(popup_layout[1])[1]
    }

    fn center_rect_absolute(absolute_x: u16, absolute_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length((r.height - absolute_y) / 2),
                    Constraint::Length(absolute_y),
                    Constraint::Length((r.height - absolute_y) / 2),
                ]
                    .as_ref(),
            )
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length((r.width - absolute_x) / 2),
                    Constraint::Length(absolute_x),
                    Constraint::Length((r.width - absolute_x) / 2),
                ]
                    .as_ref(),
            )
            .split(popup_layout[1])[1]
    }


    fn center_input(percent_x: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(100 / 2),
                    Constraint::Length(3),
                    Constraint::Percentage(100 / 2),
                ]
                    .as_ref(),
            )
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_x) / 2),
                    Constraint::Percentage(percent_x),
                    Constraint::Percentage((100 - percent_x) / 2),
                ]
                    .as_ref(),
            )
            .split(popup_layout[1])[1]
    }
}

fn extract_fields_from_struct<T: bevy_reflect::Struct + Reflect>(item: &T) -> Vec<(String, String)> {
    let mut fields: Vec<(String, String)> = vec![];

    for (i, value) in item.iter_fields().enumerate() {
        let name = item.name_at(i).unwrap().clone();
        if let Some(value) = value.downcast_ref::<bool>() {
            fields.push((name.to_string(), value.to_string()));
        }

        if let Some(value) = value.downcast_ref::<String>() {
            fields.push((name.to_string(), value.to_string()));
        }

        if let Some(value) = value.downcast_ref::<ExchangeTypeSer>() {
            fields.push((name.to_string(), format!("{:?}", value)));
        }
    }

    fields
}