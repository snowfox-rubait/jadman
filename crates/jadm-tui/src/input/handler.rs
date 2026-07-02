use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, InputMode, Panel};
use crate::client::rpc::RpcClient;
use jadm_common::protocol::Request;

pub async fn handle_key_event(key: KeyEvent, app: &mut App, rpc: &RpcClient) {
    match app.input_mode {
        InputMode::Normal => handle_normal_mode(key, app, rpc).await,
        InputMode::AddUrl => handle_add_url_mode(key, app, rpc).await,
        InputMode::ConfirmDelete => handle_confirm_delete(key, app, rpc).await,
        InputMode::Help => handle_help_mode(key, app),
        InputMode::CookiePassword => handle_cookie_password_mode(key, app, rpc).await,
    }
}

async fn handle_normal_mode(key: KeyEvent, app: &mut App, rpc: &RpcClient) {
    match key.code {
        KeyCode::Char('q') => app.running = false,

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            match app.focused_panel {
                Panel::Categories => app.next_category(),
                Panel::DownloadList => app.next_download(),
                Panel::Detail => {
                    app.scroll_offset = app.scroll_offset.saturating_add(1);
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            match app.focused_panel {
                Panel::Categories => app.prev_category(),
                Panel::DownloadList => app.prev_download(),
                Panel::Detail => {
                    app.scroll_offset = app.scroll_offset.saturating_sub(1);
                }
            }
        }
        KeyCode::Char('h') | KeyCode::Left => app.prev_panel(),
        KeyCode::Char('l') | KeyCode::Right => app.next_panel(),
        KeyCode::Tab => app.next_panel(),
        KeyCode::BackTab => app.prev_panel(),

        // Jump to top/bottom
        KeyCode::Char('g') => {
            app.selected_index = 0;
        }
        KeyCode::Char('G') => {
            let count = app.filtered_downloads().len();
            if count > 0 {
                app.selected_index = count - 1;
            }
        }

        // Download controls
        KeyCode::Char('p') => {
            if let Some(dl) = app.selected_download() {
                let _ = rpc.send(Request::PauseDownload { id: dl.download.id }).await;
            }
        }
        KeyCode::Char('r') => {
            if let Some(dl) = app.selected_download() {
                let _ = rpc.send(Request::ResumeDownload { id: dl.download.id }).await;
            }
        }
        KeyCode::Char('s') => {
            if let Some(dl) = app.selected_download() {
                let _ = rpc.send(Request::StopDownload { id: dl.download.id }).await;
            }
        }
        // 'd' = remove from list only
        KeyCode::Char('d') => {
            if let Some(dl) = app.selected_download() {
                let _ = rpc
                    .send(Request::DeleteDownload {
                        id: dl.download.id,
                        delete_file: false,
                    })
                    .await;
                app.check_bounds();
            }
        }
        // 'D' = confirm delete with file
        KeyCode::Char('D') => {
            if app.selected_download().is_some() {
                app.input_mode = InputMode::ConfirmDelete;
            }
        }

        // Add URL dialog
        KeyCode::Char('a') => {
            app.input_mode = InputMode::AddUrl;
            app.input_buffer.clear();
        }

        // Cookie Master password dialog
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.input_mode = InputMode::CookiePassword;
            app.input_buffer.clear();
        }

        // Help overlay
        KeyCode::Char('?') => {
            app.input_mode = InputMode::Help;
            app.show_help = true;
        }

        _ => {}
    }
}

async fn handle_add_url_mode(key: KeyEvent, app: &mut App, rpc: &RpcClient) {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            let url = app.input_buffer.trim().to_string();
            if !url.is_empty() {
                let result = rpc
                    .send(Request::AddDownload {
                        url,
                        folder: None,
                        priority: None,
                        cookies: None,
                        category: None,
                        mime_type: None,
                        write_subs: None,
                        embed_thumbnail: None,
                        embed_chapters: None,
                        format: None,
                        netscape_cookies: None,
                        user_agent: None,
                        ghost_mode: None,
                        engine: None,
                        live_support: None,
                        live_from_start: None,
                        compress_video: None,
                        download_playlist: None,
                        referer: None,
                        write_description: None,
                    })
                    .await;
                match result {
                    Ok(_) => app.status_message = Some("Download added!".to_string()),
                    Err(e) => app.status_message = Some(format!("Error: {}", e)),
                }
            }
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            // Ctrl+V paste (best effort in terminal)
            if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'v' {
                // Can't reliably paste in raw terminal; just insert 'v'
                app.input_buffer.push(c);
            } else {
                app.input_buffer.push(c);
            }
        }
        _ => {}
    }
}

async fn handle_confirm_delete(key: KeyEvent, app: &mut App, rpc: &RpcClient) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(dl) = app.selected_download() {
                let _ = rpc
                    .send(Request::DeleteDownload {
                        id: dl.download.id,
                        delete_file: true,
                    })
                    .await;
                app.status_message = Some("Download and file deleted".to_string());
                app.check_bounds();
            }
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
}

fn handle_help_mode(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
            app.show_help = false;
        }
        _ => {
            app.input_mode = InputMode::Normal;
            app.show_help = false;
        }
    }
}

async fn handle_cookie_password_mode(key: KeyEvent, app: &mut App, rpc: &RpcClient) {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            let password = app.input_buffer.trim().to_string();
            if !password.is_empty() {
                let result = rpc.send(Request::SetCookiePassword { password }).await;
                match result {
                    Ok(_) => app.status_message = Some("Cookie Master password set!".to_string()),
                    Err(e) => app.status_message = Some(format!("Error: {}", e)),
                }
            }
            app.input_mode = InputMode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
}
