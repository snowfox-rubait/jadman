use sqlx::{SqlitePool, Result, Row};
use jadm_common::types::{Download, DownloadStatus, DownloadEngine};
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub async fn insert_download(pool: &SqlitePool, download: &Download) -> Result<()> {
    sqlx::query(
        "INSERT INTO downloads (
            id, url, filename, size, downloaded, status, category, folder, 
            resumable, connections, engine, mime_type, cookies, netscape_cookies, user_agent, error, added_at, completed_at, last_tried_at,
            write_subs, embed_thumbnail, embed_chapters, ghost_mode, format, live_support, live_from_start, compress_video, download_playlist, referer, write_description
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30)"
    )
    .bind(download.id.to_string())
    .bind(&download.url)
    .bind(&download.filename)
    .bind(download.size.map(|s| s as i64))
    .bind(download.downloaded as i64)
    .bind(serde_json::to_string(&download.status).unwrap().trim_matches('"'))
    .bind(&download.category)
    .bind(&download.folder)
    .bind(download.resumable)
    .bind(download.connections as i32)
    .bind(serde_json::to_string(&download.engine).unwrap().trim_matches('"'))
    .bind(&download.mime_type)
    .bind(&download.cookies)
    .bind(&download.netscape_cookies)
    .bind(&download.user_agent)
    .bind(&download.error)
    .bind(download.added_at.to_rfc3339())
    .bind(download.completed_at.map(|d| d.to_rfc3339()))
    .bind(download.last_tried_at.map(|d| d.to_rfc3339()))
    .bind(download.write_subs)
    .bind(download.embed_thumbnail)
    .bind(download.embed_chapters)
    .bind(download.ghost_mode)
    .bind(&download.format)
    .bind(download.live_support)
    .bind(download.live_from_start)
    .bind(download.compress_video)
    .bind(download.download_playlist)
    .bind(&download.referer)
    .bind(download.write_description)
    .execute(pool)
    .await?;
    
    Ok(())
}


pub async fn update_download(pool: &SqlitePool, download: &Download) -> Result<()> {
    sqlx::query(
        "UPDATE downloads SET 
            filename = ?2, size = ?3, downloaded = ?4, status = ?5, 
            mime_type = ?6, cookies = ?7, netscape_cookies = ?8, user_agent = ?9, error = ?10, completed_at = ?11, last_tried_at = ?12,
            write_subs = ?13, embed_thumbnail = ?14, embed_chapters = ?15, ghost_mode = ?16, format = ?17, engine = ?18, live_support = ?19, live_from_start = ?20, compress_video = ?21,
            download_playlist = ?22, referer = ?23, write_description = ?24
        WHERE id = ?1"
    )
    .bind(download.id.to_string())
    .bind(&download.filename)
    .bind(download.size.map(|s| s as i64))
    .bind(download.downloaded as i64)
    .bind(serde_json::to_string(&download.status).unwrap().trim_matches('"'))
    .bind(&download.mime_type)
    .bind(&download.cookies)
    .bind(&download.netscape_cookies)
    .bind(&download.user_agent)
    .bind(&download.error)
    .bind(download.completed_at.map(|d| d.to_rfc3339()))
    .bind(download.last_tried_at.map(|d| d.to_rfc3339()))
    .bind(download.write_subs)
    .bind(download.embed_thumbnail)
    .bind(download.embed_chapters)
    .bind(download.ghost_mode)
    .bind(&download.format)
    .bind(serde_json::to_string(&download.engine).unwrap().trim_matches('"'))
    .bind(download.live_support)
    .bind(download.live_from_start)
    .bind(download.compress_video)
    .bind(download.download_playlist)
    .bind(&download.referer)
    .bind(download.write_description)
    .execute(pool)
    .await?;
    Ok(())
}


pub async fn get_all_downloads(pool: &SqlitePool) -> Result<Vec<Download>> {
    let rows = sqlx::query("SELECT * FROM downloads")
        .fetch_all(pool)
        .await?;
    
    let mut downloads = Vec::new();
    for row in rows {
        let status_str: String = row.try_get("status").unwrap_or_else(|_| "queued".to_string());
        let engine_str: String = row.try_get("engine").unwrap_or_else(|_| "aria2c".to_string());
        
        let added_at_str: String = row.try_get("added_at").unwrap_or_default();
        let completed_at_str: Option<String> = row.try_get("completed_at").ok();
        let last_tried_at_str: Option<String> = row.try_get("last_tried_at").ok();

        let dl = Download {
            id: Uuid::parse_str(&row.try_get::<String, _>("id").unwrap_or_default()).unwrap_or_default(),
            url: row.try_get("url").unwrap_or_default(),
            filename: row.try_get("filename").ok(),
            size: row.try_get::<Option<i64>, _>("size").ok().flatten().map(|s| s as u64),
            downloaded: row.try_get::<i64, _>("downloaded").unwrap_or(0) as u64,
            status: serde_json::from_str(&format!("\"{}\"", status_str)).unwrap_or(DownloadStatus::Queued),
            category: row.try_get("category").ok(),
            folder: row.try_get("folder").unwrap_or_default(),
            resumable: row.try_get("resumable").unwrap_or(false),
            connections: row.try_get::<i32, _>("connections").unwrap_or(8) as u32,
            engine: serde_json::from_str(&format!("\"{}\"", engine_str)).unwrap_or(DownloadEngine::Aria2c),
            format: row.try_get("format").ok(),
            mime_type: row.try_get("mime_type").ok(),
            cookies: row.try_get("cookies").ok(),
            netscape_cookies: row.try_get("netscape_cookies").ok(),
            user_agent: row.try_get("user_agent").ok(),
            error: row.try_get("error").ok(),
            added_at: DateTime::parse_from_rfc3339(&added_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            completed_at: completed_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }),
            last_tried_at: last_tried_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }),
            write_subs: row.try_get::<bool, _>("write_subs").unwrap_or(false),
            embed_thumbnail: row.try_get::<bool, _>("embed_thumbnail").unwrap_or(false),
            embed_chapters: row.try_get::<bool, _>("embed_chapters").unwrap_or(false),
            ghost_mode: row.try_get::<bool, _>("ghost_mode").unwrap_or(false),
            live_support: row.try_get::<bool, _>("live_support").unwrap_or(false),
            live_from_start: row.try_get::<bool, _>("live_from_start").unwrap_or(false),
            compress_video: row.try_get::<bool, _>("compress_video").unwrap_or(false),
            download_playlist: row.try_get::<bool, _>("download_playlist").unwrap_or(false),
            referer: row.try_get("referer").ok(),
            write_description: row.try_get::<bool, _>("write_description").unwrap_or(false),
        };
        downloads.push(dl);
    }
    Ok(downloads)
}

pub async fn delete_download(pool: &SqlitePool, id: Uuid) -> Result<()> {
    sqlx::query("DELETE FROM downloads WHERE id = ?1")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn purge_old_history(pool: &SqlitePool, days: i64) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM downloads WHERE status IN ('done', 'cancelled', 'failed') AND added_at < datetime('now', '-' || ?1 || ' days')"
    )
    .bind(days)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
