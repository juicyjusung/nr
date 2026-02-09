use crate::core::env_files::{EnvFileList, EnvScope};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use std::collections::HashSet;
use std::path::PathBuf;

pub fn render_env_selector(
    frame: &mut Frame,
    area: Rect,
    env_list: &EnvFileList,
    selected_index: usize,
    _scroll_offset: usize,
    selected_files: &HashSet<PathBuf>,
) {
    // Calculate modal size (centered, 60% width, 70% height)
    let modal_width = (area.width as f32 * 0.6) as u16;
    let modal_height = (area.height as f32 * 0.7) as u16;
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
        .title(" Environment Files ")
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

    // Build list of all env files with section headers
    let mut items = Vec::new();
    let mut flat_indices = Vec::new(); // Map display index -> (scope, file_index)

    // Package section
    if !env_list.package_files.is_empty() {
        let scope_display = if let Some(first) = env_list.package_files.first() {
            match &first.scope {
                EnvScope::Package(path) => format!("Package: {}", path.display()),
                _ => "Package:".to_string(),
            }
        } else {
            "Package:".to_string()
        };

        items.push(
            ListItem::new(Line::from(Span::styled(
                scope_display,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )))
            .style(Style::default()),
        );

        for (idx, _env_file) in env_list.package_files.iter().enumerate() {
            flat_indices.push(("package", idx));
        }
    }

    // Root section
    if !env_list.root_files.is_empty() {
        if !items.is_empty() {
            items.push(ListItem::new(Line::from(
                "─────────────────────────────────",
            )));
        }

        let scope_display = if let Some(first) = env_list.root_files.first() {
            match &first.scope {
                EnvScope::Root(path) => format!("Root: {}", path.display()),
                _ => "Root:".to_string(),
            }
        } else {
            "Root:".to_string()
        };

        items.push(
            ListItem::new(Line::from(Span::styled(
                scope_display,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )))
            .style(Style::default()),
        );

        for (idx, _env_file) in env_list.root_files.iter().enumerate() {
            flat_indices.push(("root", idx));
        }
    }

    // Render file items
    for (display_idx, (scope, file_idx)) in flat_indices.iter().enumerate() {
        let env_file = if *scope == "package" {
            &env_list.package_files[*file_idx]
        } else {
            &env_list.root_files[*file_idx]
        };

        let is_selected = display_idx == selected_index;
        let is_checked = selected_files.contains(&env_file.path);

        let checkbox = if is_checked { "[x]" } else { "[ ]" };
        let cursor = if is_selected { "❯ " } else { "  " };

        // Show parent directory path for context
        let path_hint = if let Some(parent) = env_file.path.parent() {
            if let Some(parent_name) = parent.file_name() {
                format!(" ({})", parent_name.to_string_lossy())
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let line_text = format!(
            "{}{} {}{}",
            cursor, checkbox, env_file.display_name, path_hint
        );

        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if is_checked {
            Style::default().fg(Color::Green)
        } else {
            Style::default()
        };

        items.push(ListItem::new(Line::from(line_text)).style(style));
    }

    let list = List::new(items);
    frame.render_widget(list, chunks[0]);

    // Status bar
    let status = Paragraph::new("↑↓: Navigate  Space: Toggle  Enter: Next  Esc: Cancel")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status, chunks[1]);
}
