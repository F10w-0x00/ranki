//! Application Configuration Manager for RAnki (config.ini)
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub config_path: PathBuf,
    pub collection_dir: String,
    pub hkey: String,
    pub sync_server: String,
    pub scale: f64,
}

impl AppConfig {
    pub fn load() -> Self {
        let candidates = [
            PathBuf::from("/mnt/us/extensions/ranki-dev/config.ini"),
            PathBuf::from("D:/extensions/ranki-dev/config.ini"),
            PathBuf::from("config.ini"),
        ];

        let path = candidates
            .iter()
            .find(|p| p.exists())
            .cloned()
            .unwrap_or_else(|| PathBuf::from("/mnt/us/extensions/ranki-dev/config.ini"));

        let mut config = Self {
            config_path: path.clone(),
            collection_dir: "/mnt/us/anki_data_test".to_string(),
            hkey: String::new(),
            sync_server: String::new(),
            scale: 1.2,
        };

        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.starts_with('[') {
                    continue;
                }
                if let Some((k, v)) = trimmed.split_once('=') {
                    match k.trim() {
                        "collection_dir" => config.collection_dir = v.trim().to_string(),
                        "hkey" => config.hkey = v.trim().to_string(),
                        "sync_server" => config.sync_server = v.trim().to_string(),
                        "scale" => {
                            if let Ok(s) = v.trim().parse::<f64>() {
                                config.scale = s;
                            }
                        }
                        _ => {}
                    }
                }
            }
            println!("[CONFIG] Loaded config from {:?} (hkey len: {}, sync_server: {:?})",
                path, config.hkey.len(), config.sync_server);
        } else {
            println!("[CONFIG] Warning: Could not read {:?}, using defaults", path);
        }

        config
    }

    pub fn save(&self) -> Result<(), String> {
        let content = format!(
            "[General]\ncollection_dir={}\nhkey={}\nsync_server={}\nscale={}\n",
            self.collection_dir, self.hkey, self.sync_server, self.scale
        );

        if let Some(parent) = self.config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        fs::write(&self.config_path, content)
            .map_err(|e| format!("Failed to save config to {:?}: {}", self.config_path, e))?;

        println!("[CONFIG] Successfully saved config to {:?}", self.config_path);
        Ok(())
    }

    pub fn get_resolved_collection_paths(&self) -> (PathBuf, PathBuf, PathBuf) {
        let base_str = self.collection_dir.replace('\\', "/");
        let base = Path::new(&base_str);

        // Check if on Kindle device directly
        let col = base.join("collection.anki2");
        let media = base.join("collection.media");
        let media_db = base.join("collection.media.db2");

        if col.exists() {
            return (col, media, media_db);
        }

        // Check Windows mounted drive D:\ if /mnt/us was specified
        if base_str.starts_with("/mnt/us/") {
            let win_str = format!("D:/{}", &base_str[8..]);
            let win_base = Path::new(&win_str);
            let win_col = win_base.join("collection.anki2");
            if win_col.exists() {
                return (
                    win_col,
                    win_base.join("collection.media"),
                    win_base.join("collection.media.db2"),
                );
            }
        }

        (col, media, media_db)
    }
}
