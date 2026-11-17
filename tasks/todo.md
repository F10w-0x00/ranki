# RAnki Refactor Tasks (tasks/todo.md)

## V1: Pure-Rust Host & Zero-Copy Engine (COMPLETED & VERIFIED)
- [x] **Task 0: 闭环当前 M0 基线 (Run #39 验证与动态符号隔离门禁)**
- [x] **Task 1: 建立统一 Web-SPA 纯 Rust 宿主骨架 (消灭 Vala 窗口与 TreeView)**
- [x] **Task 2: 零拷贝直连 Anki 原生核心与 WAL 事务防护 (消灭 Protobuf 与 Vala 后端)**
- [x] **Task 3: 落实印刷级双缓冲 SPA 前端引擎 (离屏毫秒级预渲染与质感参数)**

---

## V2: Anki-Authentic UI & Next-Card Pipeline Pre-Rendering (COMPLETED & DEPLOYED)
- [x] **Task 1: 纯净字体转码注入、34-44px 纸书排版与方案 A 评分栏**
- [x] **Task 2: Rust 递归子牌组展开、流水线协议与 IPC 净化**
- [x] **Task 3: WebKit 4-Slot 乒乓缓冲切卡引擎与瞬时 DOM 切换**
- [x] **Task 4: CI 构建、4 重门禁检测与 Kindle PW3 真机部署验证**

---

## V3: AnkiDroid Modern Aesthetic, AnkiWeb Sync & Login/Settings (ACTIVE)
- [x] **Task 1: 后端配置管理与 AnkiWeb 云同步 / 登录集成**
  - [x] 1. 新增 `src/config.rs`：无第三方依赖读写 `config.ini`（自动匹配 `/mnt/us/...` 与 `D:/...`，维护 `hkey` 与 `sync_server`）
  - [x] 2. 在 `Cargo.toml` 中为 `anki` 启用 `rustls` 特性，提供零动态依赖纯静态安全 TLS 连接
  - [x] 3. 在 `src/anki/mod.rs` 中集成 `sync()`、`login()` 与 `studied_today()`（调用 Anki 原生 `col.studied_today()` 统计）
  - [x] 4. 在 `src/ipc.rs` 中实现 `SYNC_COLLECTION`、`SYNC_LOGIN`、`GET_CONFIG`、`SAVE_SETTINGS` 与 `DECK_EMPTY` 友好提示
  - [x] 5. 在 `src/main.rs` 中加载 `AppConfig` 并传入集合路径
- [x] **Task 2: 终结 1990 年代表格！仿 AnkiDroid 现代极简列表 UI 与折叠子牌组**
  - [x] 1. 彻底移除 `牌组 新卡 学习 复习` 1990 年代账本表头与垂直分隔线，采用 AnkiDroid 现代扁平流式列表
  - [x] 2. 彻底移除 DOS 风格 `├─ ` 树形连线，采用平滑缩进（`depth * 28px`）与动态折叠指示箭头（`▾`/`▸`）
  - [x] 3. 重构顶栏：左侧品牌与 `待复习: X 张`，右侧 `🔄 同步`、`⚙ 设置/登录`、`退出`
  - [x] 4. 底部显示原生 Anki 学习统计：`今天在 X 秒内学习了 Y 张卡片 (平均每张卡片 Z 秒)`
  - [x] 5. 点击空牌组弹出友好 Toast 提示，不再无响应或静默重载
- [x] **Task 3: WebKit 1.0 兼容的双层评分栏与 34px 黄金阅读排版**
  - [x] 1. 弃用 WebKit 1.0 不支持的现代弹性盒多级换行，采用高可靠 `table-layout: fixed` 布局
  - [x] 2. 评分按键内清晰分两行独立排版：上行 20px 预估时间（`<10分`、`1天`），下行 30px 加粗评级（`重来`、`良好`）
  - [x] 3. 正文字号精准标定为 34px（黄金阅读四号字），消除 44px 巨大标题效应，行高 1.55
- [x] **Task 4: CI 构建、门禁验证、Kindle 部署与真机实测**
  - [x] 1. 提交至 Git 分支 `refactor/m0-baseline` 并触发 GitHub Actions CI（Run ID: 44057975886 10/10 全绿通过）
  - [x] 2. 本地执行 4 道自动化门禁（GLIBC 2.18、DT_NEEDED、0 dynsym leak、ES5 全绿通过）
  - [x] 3. 使用 `tools/deploy_to_kindle.py` 部署至 `D:\extensions\ranki-dev\`（部署成功）
  - [x] 4. 生产数据库 `D:\anki_data\` 0 字节改动红线防护（指纹 `de6efa895198` 保持 100% 完整）
  - [x] 5. Kindle PW3 真机实测云同步、登录设置、现代牌组列表与卡片复习

---

## V4: E-ink Experience Hardening & Real-Device Bug Triage (COMPLETED & DEPLOYED)
- [x] **Issue A (方框乱码)**: 彻底消灭 Emoji（Kindle OS 缺少 Emoji 字形导致的 `□` 方块），全面替换为矢量内联 SVG 统一图标。
- [x] **Issue B (严重残影)**: 拔除 `-webkit-overflow-scrolling: touch` 与连续拖拽滚动，切换为墨水屏专属**分页浏览模式**（每页固定 10 牌组，底部 `< 上一页` / `下一页 >` / `[刷新屏幕]` 清除残影按键）。
- [x] **Issue C (无法唤起键盘)**: 在设置弹窗输入框绑定 `focus`/`blur` 事件，通过 Kindle LIPC 系统调用 `com.lab126.keyboard open/close` 唤起与关闭原生虚拟键盘。
- [x] **Issue D (点击卡死/黑条僵死)**: 修复 `DECK_EMPTY` 中错误调用 `innerHTML=''` 摧毁触摸节点导致的 WebKitGTK 指针死锁；消除 CSS `:active` 伪类残留黑条，改由健壮的 touch 事件安全管理。
- [x] **CI 构建与部署**: GitHub Actions Run ID `44063646713` 全绿通过，门禁验证 100% 通过，已安全部署至 `D:\extensions\ranki-dev\`。
