#[cfg(test)]
mod tests {
    use crate::queue::manager::{QueueManager, AddDownloadParams};
    use crate::aria2::client::MockAria2ClientTrait;
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use jadm_common::types::{DownloadStatus, DownloadEngine};
    use crate::aria2::types::Aria2Status;

    async fn setup_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::db::schema::init_db(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_add_download() {
        let pool = setup_test_pool().await;
        let mock_aria2 = Arc::new(MockAria2ClientTrait::new());
        let manager = Arc::new(QueueManager::new(pool, mock_aria2, 5));

        let params = AddDownloadParams {
            url: "https://example.com/file.zip".to_string(),
            folder: "/tmp/downloads".to_string(),
            category: None,
            format: None,
            mime_type: None,
            cookies: None,
            netscape_cookies: None,
            user_agent: None,
            ghost_mode: false,
            write_subs: false,
            embed_thumbnail: false,
            embed_chapters: false,
            engine: None,
            live_support: false,
            live_from_start: false,
            compress_video: false,
            download_playlist: false,
            referer: None,
        };

        let (id, _) = manager.add_download(params).await.unwrap();
        
        let dl = manager.get_download(&id).unwrap();
        assert_eq!(dl.download.url, "https://example.com/file.zip");
        assert_eq!(dl.download.status, DownloadStatus::Queued);
    }

    #[tokio::test]
    async fn test_start_download_aria2() {
        let pool = setup_test_pool().await;
        let mut mock_aria2 = MockAria2ClientTrait::new();
        
        mock_aria2.expect_add_uri()
            .returning(|_, _| Ok("gid_123".to_string()));
            
        let manager = Arc::new(QueueManager::new(pool, Arc::new(mock_aria2), 5));

        let params = AddDownloadParams {
            url: "https://example.com/file.zip".to_string(),
            folder: "/tmp/downloads".to_string(),
            category: None,
            format: None,
            mime_type: Some("application/zip".to_string()),
            cookies: None,
            netscape_cookies: None,
            user_agent: None,
            ghost_mode: false,
            write_subs: false,
            embed_thumbnail: false,
            embed_chapters: false,
            engine: Some("aria2c".to_string()),
            live_support: false,
            live_from_start: false,
            compress_video: false,
            download_playlist: false,
            referer: None,
        };

        let (id, _) = manager.add_download(params).await.unwrap();
        manager.start_download(id, None).await.unwrap();
        
        let dl = manager.get_download(&id).unwrap();
        assert_eq!(dl.download.status, DownloadStatus::Downloading);
    }

    #[tokio::test]
    async fn test_tick_aria2_completion() {
        let pool = setup_test_pool().await;
        let mut mock_aria2 = MockAria2ClientTrait::new();
        
        mock_aria2.expect_add_uri()
            .returning(|_, _| Ok("gid_123".to_string()));
            
        mock_aria2.expect_tell_status()
            .returning(|_| Ok(Aria2Status {
                gid: "gid_123".to_string(),
                status: "complete".to_string(),
                total_length: "1000".to_string(),
                completed_length: "1000".to_string(),
                download_speed: "0".to_string(),
                files: vec![],
                error_code: None,
                error_message: None,
            }));
            
        let manager = Arc::new(QueueManager::new(pool, Arc::new(mock_aria2), 5));

        let params = AddDownloadParams {
            url: "https://example.com/file.zip".to_string(),
            folder: "/tmp/downloads".to_string(),
            category: None,
            format: None,
            mime_type: Some("application/zip".to_string()),
            cookies: None,
            netscape_cookies: None,
            user_agent: None,
            ghost_mode: false,
            write_subs: false,
            embed_thumbnail: false,
            embed_chapters: false,
            engine: Some("aria2c".to_string()),
            live_support: false,
            live_from_start: false,
            compress_video: false,
            download_playlist: false,
            referer: None,
        };

        let (id, _) = manager.add_download(params).await.unwrap();
        manager.start_download(id, None).await.unwrap();
        
        manager.tick().await.unwrap();
        
        let dl = manager.get_download(&id).unwrap();
        assert_eq!(dl.download.status, DownloadStatus::Done);
    }

    #[tokio::test]
    async fn test_tick_aria2_html_trap() {
        let pool = setup_test_pool().await;
        let mut mock_aria2 = MockAria2ClientTrait::new();
        
        let temp_dir = tempfile::tempdir().unwrap();
        let html_file_path = temp_dir.path().join("trap.html");
        std::fs::write(&html_file_path, "<!doctype html><html><body>Trap</body></html>").unwrap();
        let html_file_path_str = html_file_path.to_string_lossy().to_string();

        mock_aria2.expect_add_uri()
            .returning(|_, _| Ok("gid_123".to_string()));
            
        mock_aria2.expect_tell_status()
            .returning(move |_| Ok(Aria2Status {
                gid: "gid_123".to_string(),
                status: "complete".to_string(),
                total_length: "100".to_string(),
                completed_length: "100".to_string(),
                download_speed: "0".to_string(),
                files: vec![crate::aria2::types::Aria2File {
                    index: "1".to_string(),
                    path: html_file_path_str.clone(),
                    length: "100".to_string(),
                    completed_length: "100".to_string(),
                    selected: "true".to_string(),
                    uris: vec![],
                }],
                error_code: None,
                error_message: None,
            }));
            
        let manager = Arc::new(QueueManager::new(pool, Arc::new(mock_aria2), 5));

        let params = AddDownloadParams {
            url: "https://example.com/movie.mkv".to_string(),
            folder: temp_dir.path().to_string_lossy().to_string(),
            category: None,
            format: None,
            mime_type: Some("video/x-matroska".to_string()),
            cookies: None,
            netscape_cookies: None,
            user_agent: None,
            ghost_mode: false,
            write_subs: false,
            embed_thumbnail: false,
            embed_chapters: false,
            engine: Some("aria2c".to_string()),
            live_support: false,
            live_from_start: false,
            compress_video: false,
            download_playlist: false,
            referer: None,
        };

        let (id, _) = manager.add_download(params).await.unwrap();
        manager.start_download(id, None).await.unwrap();
        
        manager.tick().await.unwrap();
        
        let dl = manager.get_download(&id).unwrap();
        // Should have fallen back to Ytdlp and be Queued (waiting for next tick to start)
        assert_eq!(dl.download.status, DownloadStatus::Queued);
        assert_eq!(dl.download.engine, DownloadEngine::Ytdlp);
        // File should be deleted
        assert!(!html_file_path.exists());
    }

    #[tokio::test]
    async fn test_tick_aria2_to_ytdlp_fallback_on_error() {
        let pool = setup_test_pool().await;
        let mut mock_aria2 = MockAria2ClientTrait::new();
        
        mock_aria2.expect_add_uri()
            .returning(|_, _| Ok("gid_123".to_string()));
            
        mock_aria2.expect_tell_status()
            .returning(|_| Ok(Aria2Status {
                gid: "gid_123".to_string(),
                status: "error".to_string(),
                total_length: "0".to_string(),
                completed_length: "0".to_string(),
                download_speed: "0".to_string(),
                files: vec![],
                error_code: Some("1".to_string()),
                error_message: Some("Resource not found".to_string()),
            }));
            
        let manager = Arc::new(QueueManager::new(pool, Arc::new(mock_aria2), 5));

        let params = AddDownloadParams {
            url: "https://example.com/file.zip".to_string(),
            folder: "/tmp/downloads".to_string(),
            category: None,
            format: None,
            mime_type: Some("application/zip".to_string()),
            cookies: None,
            netscape_cookies: None,
            user_agent: None,
            ghost_mode: false,
            write_subs: false,
            embed_thumbnail: false,
            embed_chapters: false,
            engine: Some("aria2c".to_string()),
            live_support: false,
            live_from_start: false,
            compress_video: false,
            download_playlist: false,
            referer: None,
        };

        let (id, _) = manager.add_download(params).await.unwrap();
        manager.start_download(id, None).await.unwrap();
        
        manager.tick().await.unwrap();
        
        let dl = manager.get_download(&id).unwrap();
        assert_eq!(dl.download.status, DownloadStatus::Queued);
        assert_eq!(dl.download.engine, DownloadEngine::Ytdlp);
    }

    #[tokio::test]
    async fn test_handle_siphon_chunk() {
        let pool = setup_test_pool().await;
        let mock_aria2 = Arc::new(MockAria2ClientTrait::new());
        let manager = Arc::new(QueueManager::new(pool, mock_aria2, 5));

        let temp_dir = tempfile::tempdir().unwrap();
        let folder_str = temp_dir.path().to_string_lossy().to_string();

        let params = AddDownloadParams {
            url: "https://example.com/testfile.txt".to_string(),
            folder: folder_str.clone(),
            category: None,
            format: None,
            mime_type: Some("text/plain".to_string()),
            cookies: None,
            netscape_cookies: None,
            user_agent: None,
            ghost_mode: false,
            write_subs: false,
            embed_thumbnail: false,
            embed_chapters: false,
            engine: Some("browser_fetch".to_string()),
            live_support: false,
            live_from_start: false,
            compress_video: false,
            download_playlist: false,
            referer: None,
        };

        let (id, _) = manager.add_download(params).await.unwrap();

        // Write first chunk
        manager.handle_siphon_chunk(
            id,
            0,
            false,
            "testfile.txt".to_string(),
            20,
            b"Hello ".to_vec(),
        ).await.unwrap();

        // Verify status and size after first chunk
        let dl = manager.get_download(&id).unwrap();
        assert_eq!(dl.download.status, DownloadStatus::Downloading);
        assert_eq!(dl.download.downloaded, 6);
        assert_eq!(dl.download.size, Some(20));

        // Write second (last) chunk
        manager.handle_siphon_chunk(
            id,
            1,
            true,
            "testfile.txt".to_string(),
            20,
            b"World!".to_vec(),
        ).await.unwrap();

        // Verify status and size after final chunk
        let dl = manager.get_download(&id).unwrap();
        assert_eq!(dl.download.status, DownloadStatus::Done);
        assert_eq!(dl.download.downloaded, 12);

        // Check file contents
        let file_path = temp_dir.path().join("General/Documents/testfile.txt");
        assert!(file_path.exists());
        let content = std::fs::read_to_string(file_path).unwrap();
        assert_eq!(content, "Hello World!");
    }

    #[tokio::test]
    async fn test_handle_siphon_chunk_path_traversal() {
        let pool = setup_test_pool().await;
        let mock_aria2 = Arc::new(MockAria2ClientTrait::new());
        let manager = Arc::new(QueueManager::new(pool, mock_aria2, 5));

        let temp_dir = tempfile::tempdir().unwrap();
        let folder_str = temp_dir.path().to_string_lossy().to_string();

        let params = AddDownloadParams {
            url: "https://example.com/testfile.txt".to_string(),
            folder: folder_str.clone(),
            category: None,
            format: None,
            mime_type: Some("text/plain".to_string()),
            cookies: None,
            netscape_cookies: None,
            user_agent: None,
            ghost_mode: false,
            write_subs: false,
            embed_thumbnail: false,
            embed_chapters: false,
            engine: Some("browser_fetch".to_string()),
            live_support: false,
            live_from_start: false,
            compress_video: false,
            download_playlist: false,
            referer: None,
        };

        let (id, _) = manager.add_download(params).await.unwrap();

        // Write first chunk with malicious path traversal in filename
        manager.handle_siphon_chunk(
            id,
            0,
            true,
            "../../../../malicious.txt".to_string(),
            14,
            b"traversal test".to_vec(),
        ).await.unwrap();

        // Verify the file was written to the safe target folder name, NOT traversed upwards!
        let traversed_path = temp_dir.path().join("malicious.txt");
        assert!(!traversed_path.exists());

        let safe_path = temp_dir.path().join("General/Documents/malicious.txt");
        assert!(safe_path.exists());
        let content = std::fs::read_to_string(safe_path).unwrap();
        assert_eq!(content, "traversal test");
    }
}
