# RAnki Refactor Tasks (tasks/todo.md)

- [ ] **Task 0: 闭环当前 M0 基线 (Run #39 验证与动态符号隔离门禁)**
  - [x] 1. 编辑 `tools/ci_build_armel.sh`，使用 `objcopy --redefine-syms` 前缀化 `librslib.a` 内部 sqlite 符号并追加 `-Wl,--exclude-libs`
  - [x] 2. 在 `tools/ci_build_armel.sh` 门禁执行阶段挂载 `python3 tools/check_dynsym.py build/ranki-armel`
  - [x] 3. 跟踪 `tools/check_dynsym.py`，确保其在仓库内已暂存且具备可执行权限
  - [x] 4. Commit 并 Push 到 `refactor/m0-baseline` 触发 GitHub Actions Run #39（全绿通过）
  - [x] 5. 下载 CI 构建产物 `ranki-armel`，在本地运行三道自动化门禁断言（GATE 1/2/3 全部 PASS）
  - [ ] 6. 待 Kindle 挂载后拷贝至 `D:\extensions\ranki-dev\ranki-armel`，真机运行 `RAnki Dev (M0)` 翻动 5 卡冒烟闭环

- [ ] **Task 1: 建立统一 Web-SPA 纯 Rust 宿主骨架 (消灭 Vala 窗口与 TreeView)**
  - [ ] 1. 创建纯 Rust 顶层工作区 `Cargo.toml` 与 `src/main.rs`
  - [ ] 2. 编写轻量 direct C-FFI 模块（~80 行，声明 GTK2/WebKit1 关键符号）
  - [ ] 3. 实现单一全屏 GtkWindow + 单一 WebKitWebView 创建逻辑
  - [ ] 4. 实现 `notify::title` 双向 IPC 信号监听器（支持单调递增 `token` 解析）
  - [ ] 5. 删除 Vala 窗口前端文件（`main.vala`, `spices.vala`, `lipstick.vala`, `ink_components.vala`）
  - [ ] 6. 验证纯 Rust 窗口在 armel 上的链接与真机启动

- [ ] **Task 2: 零拷贝直连 Anki 原生核心与 WAL 事务防护 (消灭 Protobuf 与 Vala 后端)**
  - [ ] 1. 链接 `anki/rslib`，通过 `Collection::open` 直读 `collection.anki2`
  - [ ] 2. 实现纯 Rust 牌组列表提取与 JSON 序列化向 WebView 派发
  - [ ] 3. 实现纯 Rust 卡片提取、HTML 模板渲染与 FSRS/SM-2 复习调度
  - [ ] 4. 实现评分后即时事务 Commit 与退出强制 WAL Checkpoint 防护
  - [ ] 5. 彻底删除 `backend.vala`, `ffi.h`, `ffi.rs`, `protobuf-vala/` 及 24 个 `.proto`
  - [ ] 6. 运行测试库单元测试验证 `notes.flds` 逐字节一致性

- [ ] **Task 3: 落实印刷级双缓冲 SPA 前端引擎 (离屏毫秒级预渲染与质感参数)**
  - [ ] 1. 编写纯 ES5 单页应用 `views/index.html` 与 `views/app.js`
  - [ ] 2. 在 `views/mathjax/` 归档 ~1.8MB MathJax 2.7.9 核心及 20 款 TeX `.woff` 矢量字体
  - [ ] 3. 实现 `#card-active` 与 `#card-buffer`（`left: -9999px; visibility: hidden`）双缓冲切换
  - [ ] 4. 落地时序队列锁（Queue Lock）解决快速翻卡竞态
  - [ ] 5. 注入印刷级质感参数（纯黑 `#000000`、22px 黄金字号、禁用 matchFontHeight）
  - [ ] 6. 确保评分按钮物理触控热区 $\ge 48\text{px} \times 48\text{px}$，彻底移除旗子与横屏按钮
  - [ ] 7. 运行 `tools/check_es5.js` 门禁并验证 38 道高难考题 100% 渲染

- [ ] **Task 4: CI/CD 现代化与物理机 100 张卡片全量压测交付**
  - [ ] 1. 重构 `.github/workflows/build-armel.yml`，剥离 Vala/Protobuf 构建链，转为纯 Cargo CI
  - [ ] 2. 部署最终纯 Rust 二进制至 Kindle PW3
  - [ ] 3. 连续翻阅 100 张复合公式卡片，核验感知时延 $\le 5\text{ms}$
  - [ ] 4. 采集 `/proc/<pid>/status` 核验 VmRSS 增长 $\le 5\text{MB}$
  - [ ] 5. 核验空闲 CPU 占用率 0.0%，单二进制体积 $\le 20\text{MB}$
  - [ ] 6. 核验生产库 `D:\anki_data\` 0 字节变动，签署正式交付报告
