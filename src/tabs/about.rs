use crate::app::Mode;
use crate::theme::THEME;
use crate::{PAGE_SIZE, PREFERENCES};
use ratatui::{prelude::*, widgets::*};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AboutTab {
    row_index: usize,
}

impl AboutTab {
    pub async fn run(&mut self) {}
    pub fn prev_page(&mut self) {}
    pub fn next_page(&mut self) {
        info!("next page");
    }

    pub fn escape(&mut self) -> Mode {
        Mode::Exiting
    }
    pub async fn function_key(&mut self, num: u8) {
        match num {
            1 => {
                info!("F1");
            },
            _ => {}
        }
    }
    pub fn enter_key(&mut self) {
        info!("enter key");
    }
    pub fn prev_row(&mut self) {
        self.row_index = self.row_index.saturating_sub(1);
    }

    pub fn next_row(&mut self) {
        self.row_index = self.row_index.saturating_add(1);
    }
}
impl Widget for AboutTab {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // herein lies the ui code for the tab
        Paragraph::new("{{project-name}}")
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .title("About")
                    .title_alignment(Alignment::Center)
                    .border_set(symbols::border::DOUBLE)
                    .style(THEME.middle),
            )
            .render(area, buf);
    }
}
