use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

pub fn render_search_input(frame: &mut Frame, area: Rect, query: &str) {
    let display = format!("> {query}\u{2588}");
    let paragraph = Paragraph::new(display).style(Style::default().fg(Color::Cyan));
    frame.render_widget(paragraph, area);
}
