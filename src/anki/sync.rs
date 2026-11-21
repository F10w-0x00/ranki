//! Non-blocking Background Wi-Fi AnkiWeb / Custom-Server Sync Worker (DB-only, 0 media downloads).
//! Runs strictly on a spawned OS thread with an isolated single-thread tokio runtime so the
//! GTK2 main loop NEVER freezes during network operations.
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use anki::collection::CollectionBuilder;
use anki::sync::collection::normal::SyncActionRequired;
use anki::sync::login::SyncAuth;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    Normal,
    FullDownload,
    FullUpload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncOutcome {
    Success(String),
    FullSyncRequired {
        upload_ok: bool,
        download_ok: bool,
        server_message: String,
    },
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginOutcome {
    Success { hkey: String, sync_server: String },
    Error(String),
}

pub fn normalize_endpoint(endpoint: Option<&str>) -> Option<String> {
    let raw = endpoint?.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        Some(raw.to_string())
    } else {
        Some(format!("https://{}", raw))
    }
}

/// Spawn background login worker; exchanges username + password over HTTPS rustls for an hkey token.
pub fn spawn_login(
    username: String,
    password: String,
    sync_server: String,
) -> Receiver<LoginOutcome> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let res = run_login_blocking(username, password, sync_server);
        let _ = tx.send(res);
    });
    rx
}

fn run_login_blocking(username: String, mut password: String, sync_server: String) -> LoginOutcome {
    let user = username.trim();
    let pass = password.trim().to_string();
    if user.is_empty() || pass.is_empty() {
        password.clear();
        return LoginOutcome::Error("账号和密码不能为空".to_string());
    }

    let endpoint_clean = normalize_endpoint(Some(&sync_server));

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            password.clear();
            return LoginOutcome::Error(format!("创建 HTTPS 客户端失败: {}", e));
        }
    };

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            password.clear();
            return LoginOutcome::Error(format!("初始化网络运行时失败: {}", e));
        }
    };

    println!("[SYNC] Background login started for user: {}", user);
    let login_res = rt.block_on(async {
        anki::sync::login::sync_login(user, &pass, endpoint_clean, client).await
    });
    password.clear();

    match login_res {
        Ok(auth) => {
            println!("[SYNC] Background login succeeded (hkey len={})", auth.hkey.len());
            LoginOutcome::Success {
                hkey: auth.hkey,
                sync_server: sync_server.trim().to_string(),
            }
        }
        Err(e) => LoginOutcome::Error(format!("登录失败: {:?}", e)),
    }
}

/// Spawn background DB-only sync worker (NEVER downloads media!).
/// Caller MUST close the main-thread Collection before spawning and reopen it upon receiving result.
pub fn spawn_collection_sync(
    col_path: PathBuf,
    media_dir: PathBuf,
    media_db: PathBuf,
    hkey: String,
    sync_server: String,
    mode: SyncMode,
) -> Receiver<SyncOutcome> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let res = run_sync_blocking(col_path, media_dir, media_db, hkey, sync_server, mode);
        let _ = tx.send(res);
    });
    rx
}

fn run_sync_blocking(
    col_path: PathBuf,
    media_dir: PathBuf,
    media_db: PathBuf,
    hkey: String,
    sync_server: String,
    mode: SyncMode,
) -> SyncOutcome {
    if hkey.trim().is_empty() {
        return SyncOutcome::Error("尚未登录 AnkiWeb，请先在设置中登录".to_string());
    }

    let endpoint_url = match normalize_endpoint(Some(&sync_server)) {
        Some(url_str) => match reqwest::Url::parse(&url_str) {
            Ok(u) => Some(u),
            Err(e) => return SyncOutcome::Error(format!("同步服务器地址无效: {}", e)),
        },
        None => None,
    };

    let auth = SyncAuth {
        hkey: hkey.trim().to_string(),
        endpoint: endpoint_url,
        io_timeout_secs: Some(45),
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
    {
        Ok(c) => c,
        Err(e) => return SyncOutcome::Error(format!("创建 HTTPS 客户端失败: {}", e)),
    };

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => return SyncOutcome::Error(format!("初始化网络运行时失败: {}", e)),
    };

    let mut builder = CollectionBuilder::new(&col_path);
    builder.set_media_paths(&media_dir, &media_db);
    let mut col = match builder.build() {
        Ok(c) => c,
        Err(e) => return SyncOutcome::Error(format!("无法打开本地集合数据库: {:?}", e)),
    };

    match mode {
        SyncMode::Normal => {
            println!("[SYNC] Starting DB-only normal_sync (0 media downloads)...");
            let res = rt.block_on(async { col.normal_sync(auth, client).await });
            let _ = col
                .storage
                .db()
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
            let _ = col.close(None);

            match res {
                Ok(out) => match out.required {
                    SyncActionRequired::NoChanges => {
                        SyncOutcome::Success("已是最新，无需同步。".to_string())
                    }
                    SyncActionRequired::NormalSyncRequired => {
                        SyncOutcome::Success("同步完成，牌组进度已与云端保持一致。".to_string())
                    }
                    SyncActionRequired::FullSyncRequired {
                        upload_ok,
                        download_ok,
                    } => SyncOutcome::FullSyncRequired {
                        upload_ok,
                        download_ok,
                        server_message: out.server_message,
                    },
                },
                Err(e) => SyncOutcome::Error(format!("同步失败: {:?}", e)),
            }
        }
        SyncMode::FullDownload => {
            println!("[SYNC] Starting full_download (cloud -> Kindle)...");
            let res = rt.block_on(async { col.full_download(auth, client).await });
            match res {
                Ok(()) => SyncOutcome::Success("已从云端完整下载最新集合。".to_string()),
                Err(e) => SyncOutcome::Error(format!("云端下载失败: {:?}", e)),
            }
        }
        SyncMode::FullUpload => {
            println!("[SYNC] Starting full_upload (Kindle -> cloud)...");
            let res = rt.block_on(async { col.full_upload(auth, client).await });
            match res {
                Ok(()) => SyncOutcome::Success("已将本地集合完整上传至云端。".to_string()),
                Err(e) => SyncOutcome::Error(format!("本地上传失败: {:?}", e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_normalization_handles_empty_and_missing_scheme() {
        assert_eq!(normalize_endpoint(None), None);
        assert_eq!(normalize_endpoint(Some("   ")), None);
        assert_eq!(
            normalize_endpoint(Some("sync.example.com")),
            Some("https://sync.example.com".to_string())
        );
        assert_eq!(
            normalize_endpoint(Some("http://192.168.1.5:8080/")),
            Some("http://192.168.1.5:8080/".to_string())
        );
    }

    #[test]
    fn login_rejects_empty_credentials_off_main_thread() {
        let rx = spawn_login("".to_string(), "".to_string(), "".to_string());
        match rx.recv().unwrap() {
            LoginOutcome::Error(msg) => assert!(msg.contains("不能为空")),
            other => panic!("Expected error, got {:?}", other),
        }
    }
}
