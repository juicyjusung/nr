use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

pub fn render_status_bar(frame: &mut Frame, area: Rect) {
    let hints = Line::from(vec![
        Span::styled(" ↑↓ ", Style::default().bold()),
        Span::raw("navigate  "),
        Span::styled("⏎ ", Style::default().bold()),
        Span::raw("run  "),
        Span::styled("⇥ ", Style::default().bold()),
        Span::raw("config  "),
        Span::styled("␣ ", Style::default().bold()),
        Span::raw("fav  "),
        Span::styled("⎋ ", Style::default().bold()),
        Span::raw("quit"),
    ]);
    frame.render_widget(Paragraph::new(hints).style(Style::default().dim()), area);
}
