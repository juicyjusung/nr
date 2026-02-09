use crate::core::package_manager::PackageManager;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use std::path::Path;

pub fn render_execution_confirm(
    frame: &mut Frame,
    area: Rect,
    pm: PackageManager,
    script_name: &str,
    env_files: &[String],
    args: &str,
    cwd: &Path,
) {
    // Calculate modal size (centered, 70% width, 60% height)
    let modal_width = (area.width as f32 * 0.7) as u16;
    let modal_height = (area.height as f32 * 0.6) as u16;
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
        .title(" Ready to Execute ")
        .style(Style::default().bg(Color::Black));
    frame.render_widget(block, modal_area);

    // Split modal into content + status bar
    let chunks = Layout::vertical([
        Constraint::Min(1),    // Content
        Constraint::Length(1), // Status bar
    ])
    .split(modal_area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    }));

    // Build content
    let mut content_items = Vec::new();

    // Command preview
    let cmd_args = pm.run_args(script_name);
    let cmd_text = if args.is_empty() {
        format!("$ {} {}", pm.command_name(), cmd_args.join(" "))
    } else {
        format!("$ {} {} {}", pm.command_name(), cmd_args.join(" "), args)
    };

    content_items.push(ListItem::new(Line::from(Span::styled(
        cmd_text,
        Style::default().fg(Color::Green).bold(),
    ))));

    content_items.push(ListItem::new(Line::from("")));

    // Environment files
    if !env_files.is_empty() {
        content_items.push(ListItem::new(Line::from(Span::styled(
            "Env:",
            Style::default().fg(Color::Cyan),
        ))));

        for env_file in env_files {
            content_items.push(
                ListItem::new(Line::from(format!("  • {}", env_file)))
                    .style(Style::default().fg(Color::DarkGray)),
            );
        }

        content_items.push(ListItem::new(Line::from("")));
    }

    // Working directory
    content_items.push(
        ListItem::new(Line::from(vec![
            Span::styled("CWD: ", Style::default().fg(Color::Cyan)),
            Span::raw(cwd.display().to_string()),
        ]))
        .style(Style::default().fg(Color::DarkGray)),
    );

    let content_list = List::new(content_items);
    frame.render_widget(content_list, chunks[0]);

    // Status bar
    let status =
        Paragraph::new("Enter: Execute  Esc: Cancel").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status, chunks[1]);
}
