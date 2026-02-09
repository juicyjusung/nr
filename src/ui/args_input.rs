use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn render_args_input(
    frame: &mut Frame,
    area: Rect,
    input: &str,
    cursor_pos: usize,
    history: &[String],
    history_index: Option<usize>,
) {
    // Calculate modal size (centered, 60% width, 50% height)
    let modal_width = (area.width as f32 * 0.6) as u16;
    let modal_height = (area.height as f32 * 0.5) as u16;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x: area.x + modal_x,
        y: area.y + modal_y,
        width: modal_width,
        height: modal_height,
    };

    // Clear the background area
    frame.render_widget(Clear, modal_area);

    // Render modal block with opaque background
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Additional Arguments ")
        .style(Style::default().bg(Color::Black));
    frame.render_widget(block, modal_area);

    // Split modal into: input field + examples + history list + status bar
    let chunks = Layout::vertical([
        Constraint::Length(3), // Input field
        Constraint::Length(2), // Examples
        Constraint::Min(1),    // History list
        Constraint::Length(1), // Status bar
    ])
    .split(modal_area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    }));

    // Render input field with cursor at position
    let input_text = if input.is_empty() {
        vec![Span::styled(
            "█",
            Style::default().bg(Color::White).fg(Color::Black),
        )]
    } else {
        let mut spans = Vec::new();
        let chars: Vec<char> = input.chars().collect();

        // Characters before cursor
        if cursor_pos > 0 {
            spans.push(Span::raw(chars[..cursor_pos].iter().collect::<String>()));
        }

        // Cursor (block character at position)
        if cursor_pos < chars.len() {
            spans.push(Span::styled(
                chars[cursor_pos].to_string(),
                Style::default().bg(Color::White).fg(Color::Black),
            ));

            // Characters after cursor
            if cursor_pos + 1 < chars.len() {
                spans.push(Span::raw(
                    chars[cursor_pos + 1..].iter().collect::<String>(),
                ));
            }
        } else {
            // Cursor at end
            spans.push(Span::styled(
                "█",
                Style::default().bg(Color::White).fg(Color::Black),
            ));
        }

        spans
    };

    let input_widget = Paragraph::new(Line::from({
        let mut line = vec![Span::raw("Args: ")];
        line.extend(input_text);
        line
    }))
    .style(Style::default())
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(input_widget, chunks[0]);

    // Render examples
    let examples = Paragraph::new(vec![Line::from(vec![
        Span::styled("Examples: ", Style::default().fg(Color::DarkGray)),
        Span::styled("--port 3000", Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled("--watch", Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled("--env production", Style::default().fg(Color::Green)),
    ])])
    .style(Style::default());
    frame.render_widget(examples, chunks[1]);

    // Render history list (show up to 5 most recent)
    if !history.is_empty() {
        let mut history_items = vec![ListItem::new(Line::from(Span::styled(
            "Recent (↑↓):",
            Style::default().fg(Color::Cyan),
        )))];

        for (idx, entry) in history.iter().take(5).enumerate() {
            let is_selected = history_index == Some(idx);
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let line_text = if is_selected {
                format!("❯ {}", entry)
            } else {
                format!("  {}", entry)
            };

            history_items.push(ListItem::new(Line::from(line_text)).style(style));
        }

        let history_list = List::new(history_items);
        frame.render_widget(history_list, chunks[2]);
    }

    // Status bar
    let status = Paragraph::new("←→: Move  ↑↓: History  Enter: Next  Esc: Cancel")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status, chunks[3]);
}
