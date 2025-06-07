use crate::consts;
use crate::tabs::*;
use crate::theme::THEME;
use crate::tui::Event;
use crate::{tui, util};
use anyhow::Result;
use color_eyre::eyre::WrapErr;
use crossterm::event::KeyCode;
use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
use itertools::Itertools;
use ratatui::widgets::{Clear, Paragraph};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Tabs},
};
use std::collections::HashMap;
use std::io;

use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};
use time::OffsetDateTime;
use tokio::sync::mpsc;

use tokio::task::JoinHandle;
use tui_logger::TuiLoggerWidget;
use crate::tabs::about::AboutTab;

#[derive(Debug, Default, Clone)]
pub struct App {
    pub mode: Mode,
    pub tab: MenuTabs,
    pub about_tab: AboutTab,
    pub input_mode: InputMode,
    pub cursor_position: usize,
    pub input: String,
    pub prefs: Preferences,
}

impl App {
    pub(crate) fn render_send_message_popup(&self, area: Rect, buf: &mut Buffer) {
        let popup_block = Block::default()
            .title("Enter message")
            .borders(Borders::ALL)
            .title_alignment(Alignment::Center)
            .border_set(symbols::border::DOUBLE)
            .style(THEME.middle);
        let popup_area = centered_rect(area, 60, 25);
        let _popup_layout = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Percentage(50)])
            .split(popup_area);

        Widget::render(Clear, area, buf);
        Widget::render(popup_block, popup_area, buf);
        Widget::render(
            Paragraph::new(self.input.clone()).style(THEME.message_selected),
            centered_rect(popup_area, 75, 25),
            buf,
        );
    }
}

#[derive(Debug, Clone, Default)]
pub struct Preferences {
    pub(crate) initialized: String,
    pub(crate) show_mqtt: bool,
}

#[derive(Debug, Clone, Default)]
pub enum Connection {
    TCP(String, u16),
    Serial(String),
    #[default]
    None,
}

impl App {
    fn chain_hook(&mut self) {
        let original_hook = std::panic::take_hook();

        std::panic::set_hook(Box::new(move |panic| {
            disable_raw_mode().unwrap();
            crossterm::execute!(io::stdout(), LeaveAlternateScreen).unwrap();
            original_hook(panic);
        }));
    }

    fn escape(&mut self) {
        self.mode = match self.tab {
            MenuTabs::About => self.about_tab.escape(),
        }
    }
    async fn function_key(&mut self, num: u8) {
        if num == 12 {
            self.mode = Mode::RestartComms;
        }
        match self.tab {
            MenuTabs::About => self.about_tab.function_key(num).await,
            _ => {}
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        self.chain_hook();
        let mut tui = tui::Tui::new()
            .unwrap()
            .tick_rate(consts::TICK_RATE)
            .frame_rate(consts::FRAME_RATE);

        let _ = tui.enter(); // Starts event handler, enters raw mode, enters alternate screen

        while self.is_running() {
            match self.tab {
                MenuTabs::About => self.about_tab.run().await,
                _ => {}
            }

            // draw screen
            let _ = self.draw(&mut tui.terminal);

            // process input
            if let Some(Event::Key(press)) = tui.next().await {
                use KeyCode::*;
                match self.input_mode {
                    InputMode::Normal => match press.code {
                        Char('q') | Esc => self.escape(),
                        Char('h') | Left => self.left(),
                        Char('l') | Right => self.right(),
                        Char('k') | Up => self.prev(),
                        Char('j') | Down => self.next(),
                        PageUp => self.prev_page(),
                        PageDown => self.next_page(),
                        Enter => self.enter_key().await,
                        BackTab => self.prev_tab(),
                        Tab => self.next_tab(),
                        F(n) => self.function_key(n).await,
                        _ => {}
                    },
                    InputMode::Editing => match press.code {
                        Enter => self.enter_key().await,
                        Char(to_insert) => self.enter_char(to_insert),
                        Backspace => {
                            self.delete_char();
                        }
                        Left => {
                            self.move_cursor_left();
                        }
                        Right => {
                            self.move_cursor_right();
                        }
                        Esc => {
                            self.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },
                }
            };


        }
        let _ = tui.exit(); // stops event handler, exits raw mode, exits alternate screen
        Ok(())
    }

    fn draw(&self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        terminal
            .draw(|frame| {
                frame.render_widget(self, frame.size());
            })
            .wrap_err("terminal.draw")
            .unwrap();
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.mode != Mode::Exiting
    }

    fn prev_tab(&mut self) {
        match self.tab {
            _ => self.tab = self.tab.prev()
        };
    }

    fn next_tab(&mut self) {
        match self.tab {
            _ => self.tab = self.tab.next()
        }
    }

    fn left(&mut self) {
        match self.tab {
            _ => {}
        }
    }

    fn right(&mut self) {
        match self.tab {
            _ => {}
        }
    }

    fn prev(&mut self) {
        match self.tab {
            MenuTabs::About => self.about_tab.prev_row(),
            _ => {}
        }
    }

    fn prev_page(&mut self) {
        match self.tab {
            MenuTabs::About => self.about_tab.prev_page(),
            _ => {}
        }
    }

    fn next(&mut self) {
        match self.tab {
            MenuTabs::About => self.about_tab.next_row(),
        }
    }

    fn next_page(&mut self) {
        match self.tab {
            MenuTabs::About => self.about_tab.next_row(),
            _ => {}
        }
    }

    async fn enter_key_messages(&mut self) {
        match self.input_mode {
            InputMode::Normal => {
                self.input_mode = InputMode::Editing;
            }
            InputMode::Editing => {
                if !self.input.is_empty() {
                    info!("Text was input");
                }
                self.input = "".to_string();
                self.cursor_position = 0;
                self.input_mode = InputMode::Normal;
            }
        }
    }

    async fn enter_key(&mut self) {
        match self.tab {
            MenuTabs::About => self.about_tab.enter_key(),
            _ => {}
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        self.input.insert(self.cursor_position, new_char);
        self.move_cursor_right();
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.cursor_position != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.len())
    }

    fn render_event_log(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .borders(Borders::ALL)
            .title("Event Log")
            .title_alignment(Alignment::Center)
            .border_set(symbols::border::DOUBLE)
            .style(THEME.middle);

        TuiLoggerWidget::default().block(block).render(area, buf)
    }

    fn render_bottom_bar(area: Rect, buf: &mut Buffer) {
        let keys = [
            ("H/←", "Left"),
            ("L/→", "Right"),
            ("K/↑", "Up"),
            ("J/↓", "Down"),
            ("Enter", "Interact/Send"),
            ("Q/Esc", "Quit"),
        ];
        let dt: OffsetDateTime = OffsetDateTime::now_utc();

        let mut spans = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::styled(format!(" {key} "), THEME.key_binding.key);
                let desc = Span::styled(format!(" {desc} "), THEME.key_binding.description);
                [key, desc]
            })
            .collect_vec();
        spans.push(Span::styled(
            format!("| {}", dt.format(consts::DATE_FORMAT).unwrap()),
            THEME.date_display,
        ));
        Line::from(spans)
            .centered()
            .style((Color::Indexed(236), Color::Indexed(232)))
            .render(area, buf);
    }

    pub fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = MenuTabs::iter().map(MenuTabs::title);
        Tabs::new(titles)
            .style(THEME.tabs)
            .highlight_style(THEME.tabs_selected)
            .divider("")
            .padding("", "")
            .select(self.tab as usize)
            .render(area, buf);
    }

    pub fn render_selected_tab(&self, area: Rect, buf: &mut Buffer) {
        match self.tab {
            MenuTabs::About => self.about_tab.render(area, buf),
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(12),
                Constraint::Length(1),
            ]);
        let [tabs, middle, event_log, bottom_bar] = layout.areas(area);
        Block::new().style(THEME.root).render(area, buf);
        self.render_tabs(tabs, buf);
        match self.input_mode {
            InputMode::Editing => self.render_send_message_popup(middle, buf),
            InputMode::Normal => self.render_selected_tab(middle, buf),
        }
        self.render_event_log(event_log, buf);
        App::render_bottom_bar(bottom_bar, buf);
    }
}

impl MenuTabs {
    pub(crate) fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }
    pub(crate) fn prev(self) -> Self {
        let current_index = self as usize;
        let prev_index = current_index.saturating_sub(1);
        Self::from_repr(prev_index).unwrap_or(self)
    }
    fn title(self) -> String {
        format!(" {self} ")
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Running,
    Exiting,
    RestartComms,
}

#[derive(Debug, Clone, Copy, Default, Display, EnumIter, FromRepr, PartialEq, Eq)]
pub enum MenuTabs {
    #[default]
    About,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

pub(crate) fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

