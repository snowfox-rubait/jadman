use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};
use crate::app::{App, CategoryFilter, InputMode, Panel};
use crate::ui::theme;

// ═══════════════════════════════════════════════════════════════════
//  Main render entry-point
// ═══════════════════════════════════════════════════════════════════
pub fn render(f: &mut Frame, app: &mut App) {
    // Fill the entire background
    let bg_block = Block::default().style(theme::base_style());
    f.render_widget(bg_block, f.size());

    // Vertical layout: Header | Body | Status
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(0),    // body
            Constraint::Length(1), // status bar
        ])
        .split(f.size());

    render_header(f, app, outer[0]);
    render_body(f, app, outer[1]);
    render_status_bar(f, app, outer[2]);

    // Overlay popups
    match app.input_mode {
        InputMode::AddUrl => render_add_url_popup(f, app),
        InputMode::ConfirmDelete => render_confirm_delete_popup(f, app),
        InputMode::Help => render_help_overlay(f),
        InputMode::CookiePassword => render_cookie_password_popup(f, app),
        InputMode::Normal => {}
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Header Bar
// ═══════════════════════════════════════════════════════════════════
fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme::BORDER))
        .style(Style::default().bg(theme::SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(inner);

    // Left: Logo
    let logo = Line::from(vec![
        Span::styled("  JAD", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("Man", Style::default().fg(theme::TEXT_BRIGHT).add_modifier(Modifier::BOLD)),
        Span::styled("  Download Manager", Style::default().fg(theme::TEXT_DIM)),
    ]);
    f.render_widget(Paragraph::new(logo), header_chunks[0]);

    // Center: Connection status
    let (dot, label, color) = if app.connected {
        ("● ", "Connected", theme::GREEN)
    } else {
        ("● ", "Disconnected", theme::RED)
    };
    let conn_line = Line::from(vec![
        Span::styled(dot, Style::default().fg(color)),
        Span::styled(label, Style::default().fg(color)),
    ]);
    f.render_widget(
        Paragraph::new(conn_line).alignment(Alignment::Center),
        header_chunks[1],
    );

    // Right: Stats + time
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    let total = app.downloads.len();
    let active = app.active_count();
    let stats_line = Line::from(vec![
        Span::styled(format!("{}  ", now), Style::default().fg(theme::TEXT_DIM)),
        Span::styled(format!("↓ {} ", active), Style::default().fg(theme::ACCENT)),
        Span::styled(format!("Σ {}", total), Style::default().fg(theme::TEXT_DIM)),
        Span::raw("  "),
    ]);
    f.render_widget(
        Paragraph::new(stats_line).alignment(Alignment::Right),
        header_chunks[2],
    );
}

// ═══════════════════════════════════════════════════════════════════
//  Body (3-panel layout)
// ═══════════════════════════════════════════════════════════════════
fn render_body(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22),     // categories (fixed width)
            Constraint::Percentage(50), // download list
            Constraint::Min(30),        // detail
        ])
        .split(area);

    render_categories(f, app, chunks[0]);
    render_download_list(f, app, chunks[1]);
    render_detail(f, app, chunks[2]);
}

// ═══════════════════════════════════════════════════════════════════
//  Status / Help Bar
// ═══════════════════════════════════════════════════════════════════
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let keys = match app.input_mode {
        InputMode::AddUrl => vec![
            ("Enter", "Submit"),
            ("Esc", "Cancel"),
        ],
        InputMode::ConfirmDelete => vec![
            ("y", "Confirm"),
            ("n/Esc", "Cancel"),
        ],
        InputMode::Help => vec![
            ("Any key", "Close"),
        ],
        InputMode::CookiePassword => vec![
            ("Enter", "Submit"),
            ("Esc", "Cancel"),
        ],
        InputMode::Normal => vec![
            ("q", "Quit"),
            ("j/k", "Navigate"),
            ("h/l", "Panel"),
            ("p", "Pause"),
            ("r", "Resume"),
            ("s", "Stop"),
            ("d", "Remove"),
            ("D", "Delete+File"),
            ("a", "Add URL"),
            ("c", "Cookie Password"),
            ("?", "Help"),
        ],
    };

    let mut spans = Vec::new();
    for (i, (key, desc)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default().fg(theme::TEXT_DIM)));
        }
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(theme::BG)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(theme::TEXT_DIM),
        ));
    }

    // If there's a status message, show it on the right
    if let Some(msg) = &app.status_message {
        spans.push(Span::styled(
            format!("  │ {}", msg),
            Style::default().fg(theme::YELLOW),
        ));
    }

    let bar = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(theme::SURFACE));
    f.render_widget(bar, area);
}

// ═══════════════════════════════════════════════════════════════════
//  Categories Panel
// ═══════════════════════════════════════════════════════════════════
fn render_categories(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.focused_panel == Panel::Categories;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border_style(is_active))
        .title(Span::styled(
            " Categories ",
            if is_active {
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            },
        ))
        .style(theme::base_style());

    let filters = CategoryFilter::ALL_FILTERS;

    let items: Vec<ListItem> = filters
        .iter()
        .enumerate()
        .map(|(i, filter)| {
            let count = match filter {
                CategoryFilter::All => app.downloads.len(),
                _ => app.downloads.iter().filter(|dv| filter.matches(&dv.download.status)).count(),
            };

            let (icon, color) = match filter {
                CategoryFilter::All         => ("▣ ", theme::TEXT),
                CategoryFilter::Downloading => ("↓ ", theme::STATUS_DOWNLOADING),
                CategoryFilter::Finished    => ("✓ ", theme::STATUS_DONE),
                CategoryFilter::Paused      => ("⏸ ", theme::STATUS_PAUSED),
                CategoryFilter::Failed      => ("✗ ", theme::STATUS_FAILED),
                CategoryFilter::Queued      => ("◦ ", theme::STATUS_QUEUED),
            };

            let style = if i == app.category_index {
                theme::selected_style()
            } else {
                theme::base_style()
            };

            ListItem::new(Line::from(vec![
                Span::styled(icon, Style::default().fg(color)),
                Span::styled(filter.label(), Style::default().fg(theme::TEXT)),
                Span::styled(format!(" ({})", count), Style::default().fg(theme::TEXT_DIM)),
            ]))
            .style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

// ═══════════════════════════════════════════════════════════════════
//  Download List Panel
// ═══════════════════════════════════════════════════════════════════
fn render_download_list(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.focused_panel == Panel::DownloadList;
    let filtered = app.filtered_downloads();
    let filter_label = app.category_filter.label();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border_style(is_active))
        .title(Span::styled(
            format!(" {} ({}) ", filter_label, filtered.len()),
            if is_active {
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            },
        ))
        .style(theme::base_style());

    let header = Row::new(vec![
        Cell::from("  Name"),
        Cell::from("Progress"),
        Cell::from("Size"),
        Cell::from("Rate"),
        Cell::from("ETA"),
        Cell::from("Status"),
    ])
    .style(
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    )
    .height(1);

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, view)| {
            let dl = &view.download;
            let base = if i == app.selected_index && is_active {
                theme::selected_style()
            } else if app.selected_ids.contains(&dl.id) {
                Style::default().fg(theme::ACCENT).bg(theme::BG)
            } else {
                theme::row_style(i)
            };

            let status_color = theme::status_color(&dl.status);
            let icon = theme::status_icon(&dl.status);

            // Smart filename truncation (preserve extension)
            let max_name_len = 28;
            let raw_name = dl
                .filename
                .clone()
                .unwrap_or_else(|| {
                    // Show last path segment of URL
                    dl.url
                        .rsplit('/')
                        .next()
                        .unwrap_or(&dl.url)
                        .to_string()
                });
            let display_name = truncate_filename(&raw_name, max_name_len);

            let is_streaming = dl.live_support && (dl.size.is_none() || dl.size == Some(0)) && dl.status == jadm_common::types::DownloadStatus::Downloading;

            let progress_bar = if is_streaming {
                Line::from(vec![
                    Span::styled("📡 STREAMING", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
                ])
            } else {
                mini_progress_bar(view.percent, 12)
            };

            let size_str = if is_streaming {
                format_size(dl.downloaded)
            } else {
                dl.size
                    .map(|s| format_size(s))
                    .unwrap_or_else(|| "—".to_string())
            };
            let rate_str = if view.rate_bytes > 0 {
                format!("{}/s", format_size(view.rate_bytes))
            } else {
                "—".to_string()
            };
            let eta_str = if is_streaming {
                "Live".to_string()
            } else {
                view.eta_secs
                    .map(|s| format_duration(s))
                    .unwrap_or_else(|| "—".to_string())
            };

            Row::new(vec![
                Cell::from(Line::from(vec![
                    Span::styled(icon, Style::default().fg(status_color)),
                    Span::styled(display_name, Style::default().fg(theme::TEXT_BRIGHT)),
                ])),
                Cell::from(progress_bar),
                Cell::from(Span::styled(size_str, Style::default().fg(theme::TEXT_DIM))),
                Cell::from(Span::styled(rate_str, Style::default().fg(theme::ACCENT))),
                Cell::from(Span::styled(eta_str, Style::default().fg(theme::TEXT_DIM))),
                Cell::from(Span::styled(
                    dl.status.to_string(),
                    Style::default().fg(status_color),
                )),
            ])
            .style(base)
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Min(20),           // Name
            Constraint::Length(16),         // Progress
            Constraint::Length(10),         // Size
            Constraint::Length(10),         // Rate
            Constraint::Length(8),          // ETA
            Constraint::Length(13),         // Status
        ],
    )
    .header(header)
    .block(block);

    f.render_widget(table, area);
}

// ═══════════════════════════════════════════════════════════════════
//  Detail Panel
// ═══════════════════════════════════════════════════════════════════
fn render_detail(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.focused_panel == Panel::Detail;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border_style(is_active))
        .title(Span::styled(
            " Detail ",
            if is_active {
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            },
        ))
        .style(theme::base_style());

    let selected = app.selected_download();
    if selected.is_none() {
        let empty_msg = Paragraph::new(Line::from(vec![
            Span::styled("No download selected", Style::default().fg(theme::TEXT_DIM)),
        ]))
        .block(block)
        .alignment(Alignment::Center);
        f.render_widget(empty_msg, area);
        return;
    }
    let view = selected.unwrap();
    let dl = &view.download;

    let inner = block.inner(area);
    f.render_widget(block, area);

    let detail_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // progress bar
            Constraint::Length(10), // info fields
            Constraint::Length(7),  // Files list
            Constraint::Min(0),    // preview
        ])
        .split(inner);

    // ── Progress bar ──
    let is_streaming = dl.live_support && (dl.size.is_none() || dl.size == Some(0)) && dl.status == jadm_common::types::DownloadStatus::Downloading;
    render_detail_progress(f, view.percent, is_streaming, detail_chunks[0]);

    // ── Files list data ──
    let matching_files = get_download_files(dl);
    let total_size_on_disk: u64 = matching_files.iter()
        .filter(|f| f.status == "Done" || f.status == "Downloading")
        .map(|f| f.size)
        .sum();

    // ── Info fields ──
    let label_style = Style::default()
        .fg(theme::ACCENT)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(theme::TEXT_BRIGHT);
    let dim_style = Style::default().fg(theme::TEXT_DIM);

    let display_size = if dl.status == jadm_common::types::DownloadStatus::Done && total_size_on_disk > 0 {
        total_size_on_disk
    } else {
        dl.size.unwrap_or(dl.downloaded)
    };
    
    let display_downloaded = if dl.status == jadm_common::types::DownloadStatus::Done && total_size_on_disk > 0 {
        total_size_on_disk
    } else {
        dl.downloaded
    };

    let size_str = format_size(display_size);
    let dl_str = format_size(display_downloaded);
    let rate_str = if view.rate_bytes > 0 {
        format!("{}/s", format_size(view.rate_bytes))
    } else {
        "—".to_string()
    };
    let eta_str = view
        .eta_secs
        .map(|s| format_duration(s))
        .unwrap_or_else(|| "—".to_string());
    let added_str = dl.added_at.format("%b %d, %Y  %H:%M").to_string();

    let engine_icon = match dl.engine {
        jadm_common::types::DownloadEngine::Aria2c          => "⚡ ",
        jadm_common::types::DownloadEngine::Ytdlp           => "▶ ",
        jadm_common::types::DownloadEngine::ChromeNative    => "🌐 ",
        jadm_common::types::DownloadEngine::Camoufox        => "🦊 ",
        jadm_common::types::DownloadEngine::BrowserFetch    => "📡 ",
        jadm_common::types::DownloadEngine::DebuggerCapture => "🔍 ",
        jadm_common::types::DownloadEngine::WebGLCapture    => "🎮 ",
    };

    let status_color = theme::status_color(&dl.status);

    let mut info_lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("  URL       ", label_style),
            Span::styled(truncate_str(&dl.url, (detail_chunks[1].width as usize).saturating_sub(14)), dim_style),
        ]),
        Line::from(vec![
            Span::styled("  Folder    ", label_style),
            Span::styled(&dl.folder, value_style),
        ]),
        Line::from(vec![
            Span::styled("  Engine    ", label_style),
            Span::styled(engine_icon, Style::default().fg(theme::ACCENT)),
            Span::styled(dl.engine.to_string(), value_style),
        ]),
        Line::from(vec![
            Span::styled("  Size      ", label_style),
            Span::styled(
                if is_streaming {
                    format!("{} (Streaming)", dl_str)
                } else {
                    format!("{} / {}", dl_str, size_str)
                },
                value_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("  Rate      ", label_style),
            Span::styled(rate_str, Style::default().fg(theme::ACCENT)),
            Span::styled("  ETA  ", label_style),
            Span::styled(
                if is_streaming {
                    "Live".to_string()
                } else {
                    eta_str
                },
                value_style,
            ),
        ]),
        Line::from(vec![
            Span::styled("  Status    ", label_style),
            Span::styled(
                dl.status.to_string(),
                Style::default().fg(status_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Added     ", label_style),
            Span::styled(added_str, dim_style),
        ]),
    ];

    // Show error in red if present
    if let Some(err) = &dl.error {
        info_lines.push(Line::from(vec![
            Span::styled("  Error     ", Style::default().fg(theme::RED).add_modifier(Modifier::BOLD)),
            Span::styled(err.clone(), Style::default().fg(theme::RED)),
        ]));
    }

    let info_para = Paragraph::new(info_lines).wrap(Wrap { trim: false });
    f.render_widget(info_para, detail_chunks[1]);

    // ── Files block ──
    let files_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(" Files ", Style::default().fg(theme::TEXT_DIM)))
        .style(theme::base_style());

    let mut files_lines = Vec::new();
    for f_item in &matching_files {
        let size_info = if f_item.size > 0 {
            format_size(f_item.size)
        } else {
            "—".to_string()
        };
        
        let status_style = match f_item.status.as_str() {
            "Done" | "Embedded in video" => Style::default().fg(theme::GREEN),
            "Downloading" => Style::default().fg(theme::ACCENT),
            "Pending" => Style::default().fg(theme::TEXT_DIM),
            "Failed" => Style::default().fg(theme::RED),
            _ => Style::default().fg(theme::TEXT_DIM),
        };

        files_lines.push(Line::from(vec![
            Span::styled("  • ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled(truncate_str(&f_item.name, (detail_chunks[2].width as usize).saturating_sub(30)), Style::default().fg(theme::TEXT_BRIGHT)),
            Span::styled("  ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled(size_info, Style::default().fg(theme::TEXT_DIM)),
            Span::styled("  [", Style::default().fg(theme::TEXT_DIM)),
            Span::styled(f_item.status.clone(), status_style),
            Span::styled("]", Style::default().fg(theme::TEXT_DIM)),
        ]));
    }

    let files_para = Paragraph::new(files_lines).block(files_block);
    f.render_widget(files_para, detail_chunks[2]);

    // ── Preview area ──
    let preview_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(" Preview ", Style::default().fg(theme::TEXT_DIM)))
        .style(theme::base_style());

    // Locate the filename of the main media file if database has None or empty
    let main_filename = dl.filename.as_ref().filter(|f| !f.is_empty()).cloned().or_else(|| {
        matching_files.iter()
            .find(|f_item| f_item.name.contains("(Video)"))
            .map(|f_item| {
                // Strip the suffix " (Video)"
                f_item.name.replace(" (Video)", "")
            })
    });

    if let Some(filename) = &main_filename {
        let path = format!("{}/{}", dl.folder, filename);
        match app.preview_engine.get_preview(&path) {
            crate::preview::engine::PreviewData::Text(t) => {
                f.render_widget(
                    Paragraph::new(t)
                        .block(preview_block)
                        .scroll((app.scroll_offset, 0)),
                    detail_chunks[3],
                );
            }
            crate::preview::engine::PreviewData::Image => {
                f.render_widget(
                    Paragraph::new(Span::styled(
                        "  [Image — preview not available in terminal]",
                        Style::default().fg(theme::TEXT_DIM),
                    ))
                    .block(preview_block),
                    detail_chunks[3],
                );
            }
            crate::preview::engine::PreviewData::Video => {
                f.render_widget(
                    Paragraph::new(Span::styled(
                        "  [Video — preview not available in terminal]",
                        Style::default().fg(theme::TEXT_DIM),
                    ))
                    .block(preview_block),
                    detail_chunks[3],
                );
            }
            crate::preview::engine::PreviewData::None => {
                f.render_widget(
                    Paragraph::new(Span::styled(
                        "  Preview not available",
                        Style::default().fg(theme::TEXT_DIM),
                    ))
                    .block(preview_block),
                    detail_chunks[3],
                );
            }
        }
    } else {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  Filename unknown — preview unavailable",
                Style::default().fg(theme::TEXT_DIM),
            ))
            .block(preview_block),
            detail_chunks[3],
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
//  Detail progress bar (Unicode block chars with gradient)
// ═══════════════════════════════════════════════════════════════════
fn render_detail_progress(f: &mut Frame, percent: u8, is_streaming: bool, area: Rect) {
    if is_streaming {
        let text = "  📡 STREAMING LIVE... (Press 's' to Stop & save to file)";
        let spans = vec![
            Span::styled(text, Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
        ];
        let line = Line::from(spans);
        f.render_widget(Paragraph::new(line), area);
        return;
    }

    let width = area.width.saturating_sub(6) as usize; // space for "XXX% "
    if width == 0 {
        return;
    }

    let filled = (percent as usize * width) / 100;
    let color = theme::progress_color(percent);

    let mut spans = vec![Span::styled("  ", Style::default())];

    // Build the bar
    let mut bar = String::new();
    for i in 0..width {
        if i < filled {
            bar.push('█');
        } else if i == filled && percent < 100 {
            bar.push('░');
        } else {
            bar.push(' ');
        }
    }

    spans.push(Span::styled(
        bar.chars().take(filled).collect::<String>(),
        Style::default().fg(color),
    ));
    if filled < width {
        let rest: String = bar.chars().skip(filled).collect();
        spans.push(Span::styled(
            rest,
            Style::default().fg(theme::BORDER),
        ));
    }

    spans.push(Span::styled(
        format!(" {}%", percent),
        Style::default()
            .fg(color)
            .add_modifier(Modifier::BOLD),
    ));

    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), area);
}

// ═══════════════════════════════════════════════════════════════════
//  Add URL Popup
// ═══════════════════════════════════════════════════════════════════
fn render_add_url_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 7, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .title(Span::styled(
            " Add Download URL ",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme::SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // label
            Constraint::Length(1), // spacing
            Constraint::Length(1), // input
        ])
        .split(inner);

    let label = Paragraph::new(Span::styled(
        "  Enter URL to download:",
        Style::default().fg(theme::TEXT),
    ));
    f.render_widget(label, chunks[0]);

    // Input field with cursor
    let input_display = format!("  {} █", app.input_buffer);
    let input_para = Paragraph::new(Span::styled(
        input_display,
        Style::default().fg(theme::TEXT_BRIGHT),
    ));
    f.render_widget(input_para, chunks[2]);
}

fn render_cookie_password_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 7, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .title(Span::styled(
            " Cookie Master Password ",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme::SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // label
            Constraint::Length(1), // spacing
            Constraint::Length(1), // input
        ])
        .split(inner);

    let label = Paragraph::new(Span::styled(
        "  Enter Cookie Master master password:",
        Style::default().fg(theme::TEXT),
    ));
    f.render_widget(label, chunks[0]);

    // Mask password characters with asterisks
    let masked = "*".repeat(app.input_buffer.len());
    let input_display = format!("  {} █", masked);
    let input_para = Paragraph::new(Span::styled(
        input_display,
        Style::default().fg(theme::TEXT_BRIGHT),
    ));
    f.render_widget(input_para, chunks[2]);
}

// ═══════════════════════════════════════════════════════════════════
//  Confirm Delete Popup
// ═══════════════════════════════════════════════════════════════════
fn render_confirm_delete_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 7, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::RED))
        .title(Span::styled(
            " Confirm Delete ",
            Style::default()
                .fg(theme::RED)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme::SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let name = app
        .selected_download()
        .and_then(|dv| dv.download.filename.clone())
        .unwrap_or_else(|| "this download".to_string());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let msg = Paragraph::new(Line::from(vec![
        Span::styled("  Delete file ", Style::default().fg(theme::TEXT)),
        Span::styled(
            truncate_str(&name, 30),
            Style::default()
                .fg(theme::RED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" from disk?", Style::default().fg(theme::TEXT)),
    ]));
    f.render_widget(msg, chunks[0]);

    let prompt = Paragraph::new(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            " y ",
            Style::default()
                .fg(theme::BG)
                .bg(theme::RED)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Yes  ", Style::default().fg(theme::TEXT_DIM)),
        Span::styled(
            " n ",
            Style::default()
                .fg(theme::BG)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" No", Style::default().fg(theme::TEXT_DIM)),
    ]));
    f.render_widget(prompt, chunks[2]);
}

// ═══════════════════════════════════════════════════════════════════
//  Help Overlay
// ═══════════════════════════════════════════════════════════════════
fn render_help_overlay(f: &mut Frame) {
    let area = centered_rect(55, 22, f.size());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .title(Span::styled(
            " Keyboard Shortcuts ",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(theme::SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let key_style = Style::default()
        .fg(theme::ACCENT)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme::TEXT);
    let section_style = Style::default()
        .fg(theme::YELLOW)
        .add_modifier(Modifier::BOLD);

    let help_lines = vec![
        Line::from(Span::styled("  Navigation", section_style)),
        help_line("    j / ↓", "Move down", key_style, desc_style),
        help_line("    k / ↑", "Move up", key_style, desc_style),
        help_line("    h / ←", "Previous panel", key_style, desc_style),
        help_line("    l / →", "Next panel", key_style, desc_style),
        help_line("    Tab", "Next panel", key_style, desc_style),
        help_line("    g / G", "Jump to top / bottom", key_style, desc_style),
        Line::from(Span::raw("")),
        Line::from(Span::styled("  Download Control", section_style)),
        help_line("    p", "Pause download", key_style, desc_style),
        help_line("    r", "Resume download", key_style, desc_style),
        help_line("    s", "Stop download", key_style, desc_style),
        help_line("    d", "Remove from list", key_style, desc_style),
        help_line("    D", "Delete download + file", key_style, desc_style),
        help_line("    a", "Add new URL", key_style, desc_style),
        Line::from(Span::raw("")),
        Line::from(Span::styled("  General", section_style)),
        help_line("    c", "Set Cookie Master Password", key_style, desc_style),
        help_line("    ?", "Toggle this help", key_style, desc_style),
        help_line("    q", "Quit", key_style, desc_style),
    ];

    f.render_widget(Paragraph::new(help_lines), inner);
}

fn help_line<'a>(key: &'a str, desc: &'a str, ks: Style, ds: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{:<14}", key), ks),
        Span::styled(desc, ds),
    ])
}

// ═══════════════════════════════════════════════════════════════════
//  Utility functions
// ═══════════════════════════════════════════════════════════════════

/// Create a centered Rect of `percent_x`% width and `height` lines.
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    let width = (area.width as u32 * percent_x as u32 / 100) as u16;
    let x = (area.width.saturating_sub(width)) / 2;

    Rect::new(
        area.x + x,
        popup_layout[1].y,
        width.min(area.width),
        height.min(area.height),
    )
}

/// Build a mini inline progress bar for the download list.
fn mini_progress_bar(percent: u8, width: usize) -> Line<'static> {
    let filled = (percent as usize * width) / 100;
    let color = theme::progress_color(percent);

    let bar_filled: String = (0..filled).map(|_| '█').collect();
    let bar_empty: String = (filled..width).map(|_| '░').collect();

    Line::from(vec![
        Span::styled(bar_filled, Style::default().fg(color)),
        Span::styled(bar_empty, Style::default().fg(theme::BORDER)),
        Span::styled(
            format!("{:>3}%", percent),
            Style::default().fg(color),
        ),
    ])
}

/// Truncate a filename intelligently, preserving the extension.
fn truncate_filename(name: &str, max: usize) -> String {
    if name.len() <= max {
        return name.to_string();
    }
    // Find the last dot for extension
    if let Some(dot_pos) = name.rfind('.') {
        let ext = &name[dot_pos..]; // includes the dot
        if ext.len() < max.saturating_sub(4) {
            let stem_len = max - ext.len() - 2; // "…" takes ~1 char
            return format!("{}…{}", &name[..stem_len], ext);
        }
    }
    // No extension or extension too long — just truncate
    format!("{}…", &name[..max.saturating_sub(1)])
}

/// Truncate a string with ellipsis.
fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

/// Format bytes into human-readable size.
fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.1}G", b / GB)
    } else if b >= MB {
        format!("{:.1}M", b / MB)
    } else if b >= KB {
        format!("{:.0}K", b / KB)
    } else {
        format!("{}B", bytes)
    }
}

/// Format seconds into human-readable duration.
fn format_duration(secs: u64) -> String {
    if secs >= 3600 {
        format!("{}h{:02}m", secs / 3600, (secs % 3600) / 60)
    } else if secs >= 60 {
        format!("{}m{:02}s", secs / 60, secs % 60)
    } else {
        format!("{}s", secs)
    }
}

struct DownloadFileDetails {
    name: String,
    size: u64,
    status: String,
}

fn get_download_files(dl: &jadm_common::types::Download) -> Vec<DownloadFileDetails> {
    use std::path::Path;
    let mut files = Vec::new();
    
    let filename_opt = dl.filename.as_ref().filter(|f| !f.is_empty());
    let stem = if let Some(filename) = filename_opt {
        let p = Path::new(filename);
        p.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename)
            .to_string()
    } else {
        dl.url
            .rsplit('/')
            .next()
            .unwrap_or(&dl.url)
            .split('?')
            .next()
            .unwrap_or(&dl.url)
            .to_string()
    };

    let folder_path = Path::new(&dl.folder);
    let mut main_file_found = false;
    let mut subs_file_found = false;
    let mut thumb_file_found = false;

    if folder_path.exists() && folder_path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(folder_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if name.starts_with(&stem) {
                            if let Ok(metadata) = entry.metadata() {
                                let size = metadata.len();
                                let ext = Path::new(&name).extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                                
                                let file_label = if ext == "vtt" || ext == "srt" || ext == "ass" {
                                    subs_file_found = true;
                                    "Subtitle".to_string()
                                } else if ext == "jpg" || ext == "png" || ext == "webp" {
                                    thumb_file_found = true;
                                    "Thumbnail".to_string()
                                } else {
                                    main_file_found = true;
                                    "Video".to_string()
                                };

                                let status = match dl.status {
                                    jadm_common::types::DownloadStatus::Done => "Done".to_string(),
                                    jadm_common::types::DownloadStatus::Failed => "Failed".to_string(),
                                    jadm_common::types::DownloadStatus::Cancelled => "Cancelled".to_string(),
                                    jadm_common::types::DownloadStatus::Paused => "Paused".to_string(),
                                    _ => "Downloading".to_string(),
                                };

                                files.push(DownloadFileDetails {
                                    name: format!("{} ({})", name, file_label),
                                    size,
                                    status,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Add virtual assets if they are requested but not found as standalone files on disk
    if !main_file_found {
        let expected_name = filename_opt.cloned().unwrap_or_else(|| format!("{}.(ext)", stem));
        let status = match dl.status {
            jadm_common::types::DownloadStatus::Done => "Done".to_string(),
            jadm_common::types::DownloadStatus::Failed => "Failed".to_string(),
            jadm_common::types::DownloadStatus::Cancelled => "Cancelled".to_string(),
            jadm_common::types::DownloadStatus::Paused => "Paused".to_string(),
            jadm_common::types::DownloadStatus::Queued => "Queued".to_string(),
            _ => "Downloading".to_string(),
        };
        files.push(DownloadFileDetails {
            name: format!("{} (Video)", expected_name),
            size: dl.size.unwrap_or(dl.downloaded),
            status,
        });
    }

    if dl.write_subs && !subs_file_found {
        let status = match dl.status {
            jadm_common::types::DownloadStatus::Done => "Not Available / Skipped".to_string(),
            jadm_common::types::DownloadStatus::Failed => "Failed".to_string(),
            jadm_common::types::DownloadStatus::Cancelled => "Cancelled".to_string(),
            jadm_common::types::DownloadStatus::Paused => "Paused".to_string(),
            _ => "Pending".to_string(),
        };
        files.push(DownloadFileDetails {
            name: "Subtitle File".to_string(),
            size: 0,
            status,
        });
    }

    if dl.embed_thumbnail {
        let status = match dl.status {
            jadm_common::types::DownloadStatus::Done => "Embedded in video".to_string(),
            jadm_common::types::DownloadStatus::Failed => "Failed".to_string(),
            jadm_common::types::DownloadStatus::Cancelled => "Cancelled".to_string(),
            jadm_common::types::DownloadStatus::Paused => "Paused".to_string(),
            _ => if thumb_file_found { "Downloading".to_string() } else { "Pending".to_string() },
        };
        if !thumb_file_found || dl.status == jadm_common::types::DownloadStatus::Done {
            files.push(DownloadFileDetails {
                name: "Thumbnail".to_string(),
                size: 0,
                status,
            });
        }
    }

    if dl.embed_chapters {
        let status = match dl.status {
            jadm_common::types::DownloadStatus::Done => "Embedded in video".to_string(),
            jadm_common::types::DownloadStatus::Failed => "Failed".to_string(),
            jadm_common::types::DownloadStatus::Cancelled => "Cancelled".to_string(),
            jadm_common::types::DownloadStatus::Paused => "Paused".to_string(),
            _ => "Pending".to_string(),
        };
        files.push(DownloadFileDetails {
            name: "Chapter Timestamps".to_string(),
            size: 0,
            status,
        });
    }

    files
}
