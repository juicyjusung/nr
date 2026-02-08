use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

pub fn render_header_bar(
    frame: &mut Frame,
    area: Rect,
    project_name: &str,
    project_path: &str,
    package_manager: &str,
) {
    let display_path = shorten_path(project_path);

    let line = Line::from(vec![
        Span::styled(project_name, Style::default().fg(Color::Cyan).bold()),
        Span::styled("  ", Style::default()),
        Span::styled(display_path, Style::default().dim()),
        Span::styled("  ", Style::default()),
        Span::styled(package_manager, Style::default().fg(Color::Green)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn shorten_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Some(home_str) = home.to_str() {
            if let Some(rest) = path.strip_prefix(home_str) {
                return format!("~{rest}");
            }
        }
    }
    path.to_string()
}
