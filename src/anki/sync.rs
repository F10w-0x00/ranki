//! Non-blocking Background Wi-Fi AnkiWeb / Custom-Server Sync Worker (DB-only, 0 media downloads).
//! Runs strictly on a spawned OS thread with an isolated single-thread tokio runtime so the
//! GTK2 main loop NEVER freezes during network operations.
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use anki::collection::CollectionBuilder;
use anki::error::{AnkiError, NetworkErrorKind, SyncErrorKind};
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
    Success {
        message: String,
        new_endpoint: Option<String>,
    },
    FullSyncRequired {
        upload_ok: bool,
        download_ok: bool,
        server_message: String,
        new_endpoint: Option<String>,
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
    let mut url = if raw.starts_with("http://") || raw.starts_with("https://") {
        raw.to_string()
    } else {
        format!("https://{}", raw)
    };
    if !url.ends_with('/') {
        url.push('/');
    }
    Some(url)
}

/// Parse & validate endpoint URL matching official Anki's `TryFrom<SyncAuth>` (`.join("./")` guard
/// ensures `base.join("sync/")` inside `anki::sync` never panics or drops path segments).
pub fn parse_endpoint_url(endpoint: Option<&str>) -> Result<Option<reqwest::Url>, String> {
    let Some(url_str) = normalize_endpoint(endpoint) else {
        return Ok(None);
    };
    let parsed = reqwest::Url::parse(&url_str)
        .and_then(|u| u.join("./"))
        .map_err(|e| format!("同步服务器地址无效: {}", e))?;
    Ok(Some(parsed))
}

fn build_sync_client(total_timeout: Option<Duration>) -> Result<reqwest::Client, reqwest::Error> {
    // Matches official Anki rslib (`backend/mod.rs`): HTTP/1.1 avoids nginx HTTP/2 proxy streaming issues.
    let mut builder = reqwest::Client::builder()
        .http1_only()
        .connect_timeout(Duration::from_secs(30));
    if let Some(t) = total_timeout {
        builder = builder.timeout(t);
    }
    builder.build()
}

pub fn format_sync_error(action: &str, err: &AnkiError) -> String {
    match err {
        AnkiError::NetworkError { source } => match source.kind {
            NetworkErrorKind::Offline => format!("{}：网络未连接，请检查 Kindle Wi-Fi", action),
            NetworkErrorKind::Timeout => format!("{}：网络连接超时，请重试", action),
            NetworkErrorKind::ProxyAuth => format!("{}：网络代理认证失败", action),
            NetworkErrorKind::Other => format!("{}：网络异常，请查看日志", action),
        },
        AnkiError::SyncError { source } => match source.kind {
            SyncErrorKind::AuthFailed => format!("{}：账号登录已失效，请在设置中重新登录", action),
            SyncErrorKind::ClockIncorrect => format!("{}：设备时间误差过大，请校准时钟", action),
            SyncErrorKind::ServerError => format!("{}：服务器暂时不可用，请稍后重试", action),
            SyncErrorKind::ClientTooOld => format!("{}：客户端同步协议版本过旧", action),
            SyncErrorKind::UploadTooLarge => format!("{}：集合体积超过服务器限制", action),
            SyncErrorKind::Conflict => format!("{}：云端同步冲突，请重试", action),
            SyncErrorKind::DatabaseCheckRequired => format!("{}：数据库校验异常，请检查集合", action),
            SyncErrorKind::ServerMessage if !source.info.trim().is_empty() => {
                let brief: String = source.info.trim().chars().take(48).collect();
                format!("{}：{}", action, brief)
            }
            _ => format!("{}：同步协议错误，请查看日志", action),
        },
        _ => format!("{}失败，请查看 ranki_dev_stdout.log", action),
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

    let client = match build_sync_client(Some(Duration::from_secs(30))) {
        Ok(c) => c,
        Err(e) => {
            password.clear();
            eprintln!("[SYNC] ERROR login client build: {:?}", e);
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
            eprintln!("[SYNC] ERROR login runtime build: {:?}", e);
            return LoginOutcome::Error(format!("初始化网络运行时失败: {}", e));
        }
    };

    println!(
        "[SYNC] Background login started for user: {} (endpoint={})",
        user,
        endpoint_clean.as_deref().unwrap_or("https://sync.ankiweb.net/")
    );
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
        Err(e) => {
            eprintln!("[SYNC] ERROR login failed: {:?}", e);
            LoginOutcome::Error(format_sync_error("登录失败", &e))
        }
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

    let endpoint_url = match parse_endpoint_url(Some(&sync_server)) {
        Ok(u) => u,
        Err(msg) => {
            eprintln!("[SYNC] ERROR invalid endpoint {:?}: {}", sync_server, msg);
            return SyncOutcome::Error(msg);
        }
    };
    let endpoint_label = endpoint_url
        .as_ref()
        .map(|u| u.as_str())
        .unwrap_or("https://sync.ankiweb.net/")
        .to_string();

    let auth = SyncAuth {
        hkey: hkey.trim().to_string(),
        endpoint: endpoint_url,
        io_timeout_secs: Some(60),
    };

    // No hard total timeout: IoMonitor enforces 60s stall timeout so large full syncs over slow Wi-Fi succeed.
    let client = match build_sync_client(None) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[SYNC] ERROR sync client build: {:?}", e);
            return SyncOutcome::Error(format!("创建 HTTPS 客户端失败: {}", e));
        }
    };

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[SYNC] ERROR sync runtime build: {:?}", e);
            return SyncOutcome::Error(format!("初始化网络运行时失败: {}", e));
        }
    };

    let mut builder = CollectionBuilder::new(&col_path);
    builder.set_media_paths(&media_dir, &media_db);
    let mut col = match builder.build() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[SYNC] ERROR open collection {:?}: {:?}", col_path, e);
            return SyncOutcome::Error("无法打开本地集合数据库，请查看日志".to_string());
        }
    };

    match mode {
        SyncMode::Normal => {
            println!(
                "[SYNC] Starting DB-only normal_sync (0 media downloads, endpoint={})...",
                endpoint_label
            );
            let res = rt.block_on(async { col.normal_sync(auth, client).await });
            let _ = col
                .storage
                .db()
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
            let _ = col.close(None);

            match res {
                Ok(out) => {
                    println!(
                        "[SYNC] normal_sync done: required={:?} host_num={} new_endpoint={:?}",
                        out.required, out.host_number, out.new_endpoint
                    );
                    let new_endpoint = out.new_endpoint;
                    match out.required {
                        SyncActionRequired::NoChanges => SyncOutcome::Success {
                            message: "已是最新，无需同步。".to_string(),
                            new_endpoint,
                        },
                        SyncActionRequired::NormalSyncRequired => SyncOutcome::Success {
                            message: "同步完成，牌组进度已与云端保持一致。".to_string(),
                            new_endpoint,
                        },
                        SyncActionRequired::FullSyncRequired {
                            upload_ok,
                            download_ok,
                        } => SyncOutcome::FullSyncRequired {
                            upload_ok,
                            download_ok,
                            server_message: out.server_message,
                            new_endpoint,
                        },
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[SYNC] ERROR normal_sync failed (endpoint={}): {:?}",
                        endpoint_label, e
                    );
                    SyncOutcome::Error(format_sync_error("同步失败", &e))
                }
            }
        }
        SyncMode::FullDownload => {
            println!(
                "[SYNC] Starting full_download (cloud -> Kindle, endpoint={})...",
                endpoint_label
            );
            let res = rt.block_on(async { col.full_download(auth, client).await });
            match res {
                Ok(()) => {
                    println!("[SYNC] full_download succeeded (endpoint={})", endpoint_label);
                    SyncOutcome::Success {
                        message: "已从云端完整下载最新集合。".to_string(),
                        new_endpoint: None,
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[SYNC] ERROR full_download failed (endpoint={}): {:?}",
                        endpoint_label, e
                    );
                    SyncOutcome::Error(format_sync_error("云端下载失败", &e))
                }
            }
        }
        SyncMode::FullUpload => {
            println!(
                "[SYNC] Starting full_upload (Kindle -> cloud, endpoint={})...",
                endpoint_label
            );
            let res = rt.block_on(async { col.full_upload(auth, client).await });
            match res {
                Ok(()) => {
                    println!("[SYNC] full_upload succeeded (endpoint={})", endpoint_label);
                    SyncOutcome::Success {
                        message: "已将本地集合完整上传至云端。".to_string(),
                        new_endpoint: None,
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[SYNC] ERROR full_upload failed (endpoint={}): {:?}",
                        endpoint_label, e
                    );
                    SyncOutcome::Error(format_sync_error("本地上传失败", &e))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_normalization_handles_empty_missing_scheme_and_trailing_slash() {
        assert_eq!(normalize_endpoint(None), None);
        assert_eq!(normalize_endpoint(Some("   ")), None);
        assert_eq!(
            normalize_endpoint(Some("sync.example.com")),
            Some("https://sync.example.com/".to_string())
        );
        assert_eq!(
            normalize_endpoint(Some("http://192.168.1.5:8080/")),
            Some("http://192.168.1.5:8080/".to_string())
        );
        assert_eq!(
            normalize_endpoint(Some("https://sync11.ankiweb.net")),
            Some("https://sync11.ankiweb.net/".to_string())
        );
    }

    #[test]
    fn parse_endpoint_preserves_subpath_and_joins_sync_cleanly() {
        let u = parse_endpoint_url(Some("http://192.168.1.5:8080/anki"))
            .unwrap()
            .unwrap();
        assert_eq!(u.as_str(), "http://192.168.1.5:8080/anki/");
        assert_eq!(
            u.join("sync/").unwrap().join("download").unwrap().as_str(),
            "http://192.168.1.5:8080/anki/sync/download"
        );
        assert!(parse_endpoint_url(Some("")).unwrap().is_none());
        assert!(parse_endpoint_url(Some("http://[::1")).is_err());
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
