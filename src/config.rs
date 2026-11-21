//! Minimalist, power-loss-safe application configuration (config.ini) for RAnki Route B.
//! Built-in defaults (20s circular focus timer, 20-card auto GC16 refresh) are intentionally
//! non-configurable in the UI to keep settings radically focused.
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_FONT_SIZE_PX: u32 = 42;
pub const MIN_FONT_SIZE_PX: u32 = 28;
pub const MAX_FONT_SIZE_PX: u32 = 54;
pub const BUILTIN_FOCUS_TIMER_SECS: u32 = 20;
pub const BUILTIN_AUTO_REFRESH_CARDS: u32 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub config_path: PathBuf,
    pub collection_dir: String,
    pub hkey: String,
    pub sync_server: String,
    pub font_size_px: u32,
}

impl AppConfig {
    pub fn default_at(path: PathBuf) -> Self {
        Self {
            config_path: path,
            collection_dir: "/mnt/us/anki_data_test".to_string(),
            hkey: String::new(),
            sync_server: String::new(),
            font_size_px: DEFAULT_FONT_SIZE_PX,
        }
    }

    pub fn load() -> Self {
        let mut candidates = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.push(dir.join("config.ini"));
            }
        }
        candidates.push(PathBuf::from("/mnt/us/extensions/ranki-dev/config.ini"));
        candidates.push(PathBuf::from("D:/extensions/ranki-dev/config.ini"));
        candidates.push(PathBuf::from("config.ini"));

        let path = candidates
            .iter()
            .find(|p| p.exists())
            .cloned()
            .unwrap_or_else(|| PathBuf::from("/mnt/us/extensions/ranki-dev/config.ini"));

        Self::load_from_path(path)
    }

    pub fn load_from_path(path: PathBuf) -> Self {
        let mut config = Self::default_at(path.clone());
        if let Ok(content) = fs::read_to_string(&path) {
            config.parse_ini_str(&content);
            println!(
                "[CONFIG] Loaded {:?} (hkey_len={}, font_size_px={})",
                path,
                config.hkey.len(),
                config.font_size_px
            );
        } else {
            println!("[CONFIG] Config {:?} not found; using safe defaults.", path);
        }
        config
    }

    pub fn parse_ini_str(&mut self, content: &str) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with('#')
                || trimmed.starts_with(';')
                || trimmed.starts_with('[')
            {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                let key = k.trim();
                let val = v.trim();
                match key {
                    "collection_dir" if !val.is_empty() => {
                        self.collection_dir = val.to_string();
                    }
                    "hkey" => {
                        self.hkey = val.to_string();
                    }
                    "sync_server" => {
                        self.sync_server = val.to_string();
                    }
                    "font_size_px" => {
                        if let Ok(px) = val.parse::<u32>() {
                            self.font_size_px = clamp_font_size(px);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn set_font_size_px(&mut self, px: u32) {
        self.font_size_px = clamp_font_size(px);
    }

    pub fn serialize_ini(&self) -> String {
        format!(
            "[General]\ncollection_dir={}\nhkey={}\nsync_server={}\nfont_size_px={}\n",
            self.collection_dir,
            self.hkey,
            self.sync_server,
            clamp_font_size(self.font_size_px)
        )
    }

    pub fn save(&self) -> Result<(), String> {
        let content = self.serialize_ini();
        if let Some(parent) = self.config_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create config dir {:?}: {}", parent, e))?;
            }
        }

        let tmp_path = self.config_path.with_extension("ini.tmp");
        fs::write(&tmp_path, &content)
            .map_err(|e| format!("Failed to write temp config {:?}: {}", tmp_path, e))?;

        if let Err(rename_err) = fs::rename(&tmp_path, &self.config_path) {
            let _ = fs::remove_file(&tmp_path);
            fs::write(&self.config_path, &content).map_err(|e| {
                format!(
                    "Failed to save config {:?} (rename: {}, direct: {})",
                    self.config_path, rename_err, e
                )
            })?;
        }

        println!("[CONFIG] Saved {:?}", self.config_path);
        Ok(())
    }

    pub fn get_resolved_collection_paths(&self) -> (PathBuf, PathBuf, PathBuf) {
        let base_str = self.collection_dir.replace('\\', "/");
        let base = Path::new(&base_str);

        let col = base.join("collection.anki2");
        let media = base.join("collection.media");
        let media_db = base.join("collection.media.db2");

        if col.exists() {
            return (col, media, media_db);
        }

        if let Some(stripped) = base_str.strip_prefix("/mnt/us/") {
            let win_str = format!("D:/{}", stripped);
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

pub fn clamp_font_size(px: u32) -> u32 {
    px.clamp(MIN_FONT_SIZE_PX, MAX_FONT_SIZE_PX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe_and_have_no_hardcoded_secret() {
        let cfg = AppConfig::default_at(PathBuf::from("test.ini"));
        assert_eq!(cfg.collection_dir, "/mnt/us/anki_data_test");
        assert!(cfg.hkey.is_empty());
        assert_eq!(cfg.font_size_px, 42);
        assert_eq!(BUILTIN_FOCUS_TIMER_SECS, 20);
        assert_eq!(BUILTIN_AUTO_REFRESH_CARDS, 8);
    }

    #[test]
    fn parse_clamps_bounds_and_handles_legacy_keys() {
        let mut cfg = AppConfig::default_at(PathBuf::from("test.ini"));
        cfg.parse_ini_str(
            "[General]\n\
             collection_dir=/mnt/us/anki_data_test\n\
             hkey=tok_abc123\n\
             sync_server=https://sync.example.com/\n\
             font_size_px=12\n\
             math_engine=katex\n\
             scale=1.2\n",
        );
        assert_eq!(cfg.hkey, "tok_abc123");
        assert_eq!(cfg.sync_server, "https://sync.example.com/");
        assert_eq!(cfg.font_size_px, MIN_FONT_SIZE_PX);

        cfg.parse_ini_str("font_size_px=999\n");
        assert_eq!(cfg.font_size_px, MAX_FONT_SIZE_PX);
    }

    #[test]
    fn atomic_save_and_reload_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "ranki_cfg_test_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("config.ini");

        let mut cfg = AppConfig::default_at(path.clone());
        cfg.hkey = "my_token".to_string();
        cfg.sync_server = "https://anki.local".to_string();
        cfg.set_font_size_px(46);
        cfg.save().expect("save should succeed");

        let loaded = AppConfig::load_from_path(path);
        assert_eq!(loaded, cfg);

        let _ = fs::remove_dir_all(&dir);
    }
}
