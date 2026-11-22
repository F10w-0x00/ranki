//! Minimalist bilingual UI localization (English default, Chinese optional) for RAnki Route B.
//! Strict zero-emoji ASCII/UTF-8 text so Kindle OS 5.14.3 never renders tofu boxes.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UiLang {
    #[default]
    En,
    Zh,
}

impl UiLang {
    pub fn from_code(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "zh" | "zh-cn" | "zh_cn" | "zh-hans" | "cn" | "chinese" => Self::Zh,
            _ => Self::En,
        }
    }

    pub fn as_code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Zh => "zh",
        }
    }

    pub fn is_cjk(self) -> bool {
        matches!(self, Self::Zh)
    }

    // --- Deck Browser ---
    pub fn deck_due_summary(self, n: u32) -> String {
        match self {
            Self::En => format!("Due {}", n),
            Self::Zh => format!("待复习 {} 张", n),
        }
    }

    pub fn btn_sync(self) -> &'static str {
        match self {
            Self::En => "Sync",
            Self::Zh => "同步",
        }
    }

    pub fn btn_settings(self) -> &'static str {
        match self {
            Self::En => "Settings",
            Self::Zh => "设置",
        }
    }

    pub fn btn_refresh(self) -> &'static str {
        match self {
            Self::En => "Refresh",
            Self::Zh => "刷新",
        }
    }

    pub fn btn_exit(self) -> &'static str {
        match self {
            Self::En => "Exit",
            Self::Zh => "退出",
        }
    }

    pub fn empty_decks_hint(self) -> &'static str {
        match self {
            Self::En => "No decks yet (tap Settings to sign in & sync AnkiWeb)",
            Self::Zh => "暂无牌组（请点击右上角「设置」登录并同步 AnkiWeb）",
        }
    }

    pub fn btn_prev_page(self) -> &'static str {
        match self {
            Self::En => "< Prev",
            Self::Zh => "< 上一页",
        }
    }

    pub fn btn_next_page(self) -> &'static str {
        match self {
            Self::En => "Next >",
            Self::Zh => "下一页 >",
        }
    }

    pub fn page_of(self, cur: usize, total: usize) -> String {
        match self {
            Self::En => format!("Page {} / {}", cur, total),
            Self::Zh => format!("第 {} / {} 页", cur, total),
        }
    }

    pub fn studied_today_fallback(self) -> &'static str {
        match self {
            Self::En => "Studied 0 cards in 0 seconds today",
            Self::Zh => "今天在 0 秒内学习了 0 张卡片",
        }
    }

    // --- Review View ---
    pub fn review_default_title(self) -> &'static str {
        match self {
            Self::En => "Review",
            Self::Zh => "复习",
        }
    }

    pub fn btn_decks(self) -> &'static str {
        match self {
            Self::En => "< Decks",
            Self::Zh => "< 牌组",
        }
    }

    pub fn btn_undo(self) -> &'static str {
        match self {
            Self::En => "Undo",
            Self::Zh => "撤销",
        }
    }

    pub fn btn_show_answer(self) -> &'static str {
        match self {
            Self::En => "Show Answer",
            Self::Zh => "显  示  答  案",
        }
    }

    pub fn btn_back_to_decks(self) -> &'static str {
        match self {
            Self::En => "Back to Decks",
            Self::Zh => "返回牌组大厅",
        }
    }

    pub fn rating_labels(self) -> [&'static str; 4] {
        match self {
            Self::En => ["Again", "Hard", "Good", "Easy"],
            Self::Zh => ["重来", "困难", "良好", "简单"],
        }
    }

    // --- Review Action Menu Dialog ---
    pub fn menu_title(self) -> &'static str {
        match self {
            Self::En => "Card & Actions",
            Self::Zh => "当前卡片与操作",
        }
    }

    pub fn menu_subtitle(self) -> &'static str {
        match self {
            Self::En => "Choose an action:",
            Self::Zh => "请选择要执行的操作：",
        }
    }

    pub fn menu_bury(self) -> &'static str {
        match self {
            Self::En => "Bury Card (Skip Today)",
            Self::Zh => "搁置此卡 (今日跳过)",
        }
    }

    pub fn menu_suspend(self) -> &'static str {
        match self {
            Self::En => "Suspend Card (Remove)",
            Self::Zh => "暂停此卡 (永久移出复习)",
        }
    }

    pub fn menu_refresh(self) -> &'static str {
        match self {
            Self::En => "Full Screen Refresh (GC16)",
            Self::Zh => "全屏清残影 (GC16 刷新)",
        }
    }

    pub fn menu_settings(self) -> &'static str {
        match self {
            Self::En => "Font, Language & Account",
            Self::Zh => "字号、语言与账号设置",
        }
    }

    pub fn btn_cancel(self) -> &'static str {
        match self {
            Self::En => "Cancel",
            Self::Zh => "取消",
        }
    }

    // --- Settings View ---
    pub fn btn_back(self) -> &'static str {
        match self {
            Self::En => "< Back",
            Self::Zh => "< 返回",
        }
    }

    pub fn settings_title(self) -> &'static str {
        match self {
            Self::En => "Settings",
            Self::Zh => "账号与阅读设置",
        }
    }

    pub fn status_signed_in(self) -> &'static str {
        match self {
            Self::En => "Account: Signed in to AnkiWeb (token saved, ready to sync)",
            Self::Zh => "账号状态：已登录 AnkiWeb（安全 Token 已保存，可直接同步）",
        }
    }

    pub fn status_signed_out(self) -> &'static str {
        match self {
            Self::En => "Account: Not signed in (enter email & password below)",
            Self::Zh => "账号状态：未登录（请输入邮箱与密码登录）",
        }
    }

    pub fn lbl_email(self) -> &'static str {
        match self {
            Self::En => "AnkiWeb Email:",
            Self::Zh => "AnkiWeb 邮箱 (Email):",
        }
    }

    pub fn lbl_password(self) -> &'static str {
        match self {
            Self::En => "Password (exchanged for token, never saved):",
            Self::Zh => "密码 (仅换取安全 Token，明文不落盘):",
        }
    }

    pub fn lbl_server(self) -> &'static str {
        match self {
            Self::En => "Custom Sync Server (leave blank for AnkiWeb):",
            Self::Zh => "自定义同步服务器 (留空使用官方 AnkiWeb):",
        }
    }

    pub fn btn_sign_in(self) -> &'static str {
        match self {
            Self::En => "Sign In",
            Self::Zh => "登录并保存 Token",
        }
    }

    pub fn btn_save_server(self) -> &'static str {
        match self {
            Self::En => "Save Server",
            Self::Zh => "保存服务器",
        }
    }

    pub fn btn_sign_out(self) -> &'static str {
        match self {
            Self::En => "Sign Out",
            Self::Zh => "清除登录",
        }
    }

    pub fn lbl_font_size(self) -> &'static str {
        match self {
            Self::En => "Card Font Size (300 PPI):",
            Self::Zh => "卡片正文字号 (300 PPI):",
        }
    }

    pub fn lbl_language(self) -> &'static str {
        match self {
            Self::En => "Interface Language:",
            Self::Zh => "界面语言:",
        }
    }

    pub fn msg_signing_in(self) -> &'static str {
        match self {
            Self::En => "Connecting to server and verifying account...",
            Self::Zh => "正在连接服务器验证账号，请稍候...",
        }
    }

    pub fn msg_login_success(self) -> &'static str {
        match self {
            Self::En => "Signed in! Token saved, starting sync...",
            Self::Zh => "登录成功！安全 Token 已保存，即将自动同步集合...",
        }
    }

    pub fn msg_server_saved(self) -> &'static str {
        match self {
            Self::En => "Sync server address saved.",
            Self::Zh => "同步服务器地址已保存。",
        }
    }

    pub fn msg_signed_out(self) -> &'static str {
        match self {
            Self::En => "Local sign-in token cleared.",
            Self::Zh => "已清除本地登录 Token。",
        }
    }

    pub fn msg_login_required(self) -> &'static str {
        match self {
            Self::En => "Not signed in. Enter email and password first.",
            Self::Zh => "尚未登录 AnkiWeb，请先输入账号密码登录",
        }
    }

    // --- Banners, Sync & Dialogs ---
    pub fn banner_db_not_open(self) -> &'static str {
        match self {
            Self::En => "Collection not open. Please sync or check database.",
            Self::Zh => "本地数据库未打开，请先同步或检查测试库。",
        }
    }

    pub fn banner_deck_done(self) -> &'static str {
        match self {
            Self::En => "All due cards in this deck are finished today.",
            Self::Zh => "该牌组今日待复习卡片已全部完成",
        }
    }

    pub fn banner_syncing_normal(self) -> &'static str {
        match self {
            Self::En => "Syncing collection (DB only, no media)...",
            Self::Zh => "正在与云端同步集合（不下载媒体）...",
        }
    }

    pub fn banner_syncing_download(self) -> &'static str {
        match self {
            Self::En => "Downloading full collection from cloud...",
            Self::Zh => "正在从云端完整下载最新集合...",
        }
    }

    pub fn banner_syncing_upload(self) -> &'static str {
        match self {
            Self::En => "Uploading full collection to cloud...",
            Self::Zh => "正在上传本地集合至云端...",
        }
    }

    pub fn banner_sync_up_to_date(self) -> &'static str {
        match self {
            Self::En => "Already up to date.",
            Self::Zh => "已是最新，无需同步。",
        }
    }

    pub fn banner_sync_done(self) -> &'static str {
        match self {
            Self::En => "Sync complete. Progress is up to date.",
            Self::Zh => "同步完成，牌组进度已与云端保持一致。",
        }
    }

    pub fn banner_download_done(self) -> &'static str {
        match self {
            Self::En => "Downloaded latest collection from cloud.",
            Self::Zh => "已从云端完整下载最新集合。",
        }
    }

    pub fn banner_upload_done(self) -> &'static str {
        match self {
            Self::En => "Uploaded local collection to cloud.",
            Self::Zh => "已将本地集合完整上传至云端。",
        }
    }

    pub fn banner_full_sync_needed(self) -> &'static str {
        match self {
            Self::En => "Full sync required — choose direction.",
            Self::Zh => "需要全量同步，请选择方向",
        }
    }

    pub fn dialog_full_sync_title(self) -> &'static str {
        match self {
            Self::En => "Full Sync Required",
            Self::Zh => "需要全量同步",
        }
    }

    pub fn dialog_full_sync_body(self, server_message: &str) -> String {
        let trimmed = server_message.trim();
        match self {
            Self::En => {
                if trimmed.is_empty() {
                    "Local and cloud collections differ and require a one-way full sync (database only, no media downloaded):".to_string()
                } else {
                    format!("{}\n\nChoose full sync direction:", trimmed)
                }
            }
            Self::Zh => {
                if trimmed.is_empty() {
                    "本地与云端集合存在结构差异，必须执行单向全量同步（仅同步数据库，不下载媒体）：".to_string()
                } else {
                    format!("{}\n\n请选择全量同步方向：", trimmed)
                }
            }
        }
    }

    pub fn btn_full_download(self) -> &'static str {
        match self {
            Self::En => "Download from AnkiWeb",
            Self::Zh => "从云端下载覆盖本地",
        }
    }

    pub fn btn_full_upload(self) -> &'static str {
        match self {
            Self::En => "Upload to AnkiWeb",
            Self::Zh => "上传本地覆盖云端",
        }
    }

    pub fn err_login_thread(self) -> &'static str {
        match self {
            Self::En => "Sign-in worker stopped unexpectedly.",
            Self::Zh => "登录后台线程异常中断",
        }
    }

    pub fn err_sync_thread(self) -> &'static str {
        match self {
            Self::En => "Sync worker stopped unexpectedly.",
            Self::Zh => "同步后台线程异常中断",
        }
    }

    pub fn err_empty_credentials(self) -> &'static str {
        match self {
            Self::En => "Email and password cannot be empty.",
            Self::Zh => "账号和密码不能为空",
        }
    }

    pub fn err_invalid_server(self) -> &'static str {
        match self {
            Self::En => "Invalid sync server URL.",
            Self::Zh => "同步服务器地址无效",
        }
    }

    pub fn err_http_client(self) -> &'static str {
        match self {
            Self::En => "Failed to initialize HTTPS client.",
            Self::Zh => "创建 HTTPS 客户端失败",
        }
    }

    pub fn err_runtime(self) -> &'static str {
        match self {
            Self::En => "Failed to initialize network runtime.",
            Self::Zh => "初始化网络运行时失败",
        }
    }

    pub fn err_open_db(self) -> &'static str {
        match self {
            Self::En => "Could not open local collection database.",
            Self::Zh => "无法打开本地集合数据库，请查看日志",
        }
    }

    pub fn action_login_failed(self) -> &'static str {
        match self {
            Self::En => "Sign-in failed",
            Self::Zh => "登录失败",
        }
    }

    pub fn action_sync_failed(self) -> &'static str {
        match self {
            Self::En => "Sync failed",
            Self::Zh => "同步失败",
        }
    }

    pub fn action_download_failed(self) -> &'static str {
        match self {
            Self::En => "Download failed",
            Self::Zh => "云端下载失败",
        }
    }

    pub fn action_upload_failed(self) -> &'static str {
        match self {
            Self::En => "Upload failed",
            Self::Zh => "本地上传失败",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_language_is_english_and_codes_parse_cleanly() {
        assert_eq!(UiLang::default(), UiLang::En);
        assert_eq!(UiLang::from_code(""), UiLang::En);
        assert_eq!(UiLang::from_code("en"), UiLang::En);
        assert_eq!(UiLang::from_code("EN-US"), UiLang::En);
        assert_eq!(UiLang::from_code("zh"), UiLang::Zh);
        assert_eq!(UiLang::from_code("zh-CN"), UiLang::Zh);
        assert_eq!(UiLang::En.as_code(), "en");
        assert_eq!(UiLang::Zh.as_code(), "zh");
    }

    #[test]
    fn bilingual_labels_are_non_empty_and_distinct() {
        assert_eq!(UiLang::En.btn_show_answer(), "Show Answer");
        assert_eq!(UiLang::Zh.btn_show_answer(), "显  示  答  案");
        assert_eq!(UiLang::En.rating_labels(), ["Again", "Hard", "Good", "Easy"]);
        assert_eq!(UiLang::Zh.rating_labels(), ["重来", "困难", "良好", "简单"]);
        assert_eq!(UiLang::En.deck_due_summary(10), "Due 10");
        assert_eq!(UiLang::Zh.deck_due_summary(10), "待复习 10 张");
    }
}
