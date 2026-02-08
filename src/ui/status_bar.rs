use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

pub fn render_status_bar(frame: &mut Frame, area: Rect) {
    let hints = Line::from(vec![
        Span::styled(" \u{2191}\u{2193} ", Style::default().bold()),
        Span::raw("navigate  "),
        Span::styled("\u{23ce} ", Style::default().bold()),
        Span::raw("run  "),
        Span::styled("\u{2423} ", Style::default().bold()),
        Span::raw("fav  "),
        Span::styled("\u{238b} ", Style::default().bold()),
        Span::raw("quit"),
    ]);
    frame.render_widget(Paragraph::new(hints).style(Style::default().dim()), area);
}
