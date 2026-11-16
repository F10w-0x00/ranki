# RAnki Pure Rust Refactor Execution Plan (tasks/plan.md)

> **Authority**: HANDOFF.md v5.0.0 & SPEC.md  
> **Methodology**: Constraint-Driven & Spec-Driven Development, Vertical Slicing  
> **Status**: Ready for Execution  

---

## Architecture Dependency Graph

```
[Task 0: M0 Baseline & Dynsym Gate]
             │
             ▼
[Task 1: Pure Rust Web-SPA Host Skeleton]
             │
             ▼
[Task 2: Zero-Copy Anki rslib & WAL Safety]
             │
             ▼
[Task 3: Print-Grade Dual-Buffer MathJax Engine]
             │
             ▼
[Task 4: CI/CD Modernization & 100-Card Stress Delivery]
```

---

## Detailed Task Breakdown

### Task 0: 闭环当前 M0 基线 (Run #38 验证与动态符号隔离门禁)
*   **Description**: 在现有混合代码基础上，向 `tools/ci_build_armel.sh` 注入链接器参数 `-Wl,--exclude-libs,librslib.a`（阻断 626 个 `sqlite3_*` 符号外溢），并将 `tools/check_dynsym.py` 挂载至构建自动化门禁。提交并推送到 GitHub 触发 CI Run #38，下载产物在本地通过三道门禁，拷贝至 Kindle PW3 验证 5 卡正常加载。锁定对比基线。
*   **Acceptance Criteria**:
    - [ ] `tools/ci_build_armel.sh` 包含 `-Wl,--exclude-libs,librslib.a` 并调用 `python3 tools/check_dynsym.py build/ranki-armel`。
    - [ ] GitHub Actions CI Run #38 构建成功（Exit Code 0）。
    - [ ] 本地执行 `python tools/check_dynsym.py build/ranki-armel` 返回 `[PASS]`（泄漏数 = 0）。
    - [ ] 产物部署到 `D:\extensions\ranki-dev\ranki-armel`，PW3 上通过 `RAnki Dev (M0)` 成功启动并翻阅 5 张卡。
*   **Verification**:
    - `python tools/check_symbols.py build/ranki-armel`
    - `python tools/check_dt_needed.py build/ranki-armel`
    - `python tools/check_dynsym.py build/ranki-armel`
    - Kindle 运行日志 `D:\ranki_logs\ranki_dev_stdout.log` 确认无 SIGSEGV 139。
*   **Dependencies**: None
*   **Files Touched**:
    - `tools/ci_build_armel.sh`
    - `tools/check_dynsym.py`

---

### Task 1: 建立统一 Web-SPA 纯 Rust 宿主骨架 (消灭 Vala 窗口与 TreeView)
*   **Description**: 创建纯 Rust 顶层工作区，实现直接轻量 C-FFI（~80 行，声明 `gtk_init`, `gtk_window_new`, `webkit_web_view_new` 等必要符号）。创建单一全屏 GtkWindow 和单一 WebKitWebView，加载 `views/index.html`。实现 Rust 与 WebKit 之间的 `notify::title` 双向 IPC 监听（带单调自增 `token`）。
*   **Acceptance Criteria**:
    - [ ] 根目录 `Cargo.toml` 与 `src/main.rs` 纯 Rust 工程建立，彻底解绑 Vala。
    - [ ] 实现 `notify::title` 信号绑定，测试发送 `RA:ready:1:{}` 能够被 Rust 宿主捕获。
    - [ ] 删除 `main.vala`, `spices.vala`, `lipstick.vala`, `ink_components.vala`。
    - [ ] CI 构建生成纯 Rust `ranki-armel`，在 Kindle 实机上成功启动全屏 WebKit 视窗。
*   **Verification**:
    - `cargo check --target arm-unknown-linux-gnueabi` (CI 内)
    - `python tools/check_symbols.py build/ranki-armel`
    - Kindle PW3 实机拉起单页视窗，无崩溃。
*   **Dependencies**: Task 0
*   **Files Touched**:
    - `Cargo.toml`
    - `src/main.rs`
    - `src/ffi/mod.rs`
    - `src/ipc.rs`
    - `views/index.html`
    - 删除 `main.vala`, `spices.vala`, `lipstick.vala`, `ink_components.vala`

---

### Task 2: 零拷贝直连 Anki 原生核心与 WAL 事务防护 (消灭 Protobuf 与 Vala 后端)
*   **Description**: 在 Rust 宿主中直接依赖 `anki/rslib`，通过 `Collection::open` 直连 `collection.anki2`。实现纯 Rust 牌组列表查询、卡片抽取、HTML 题面合成与 FSRS/SM-2 复习调度。设立每卡评分事务提交与退出强制 WAL Checkpoint。
*   **Acceptance Criteria**:
    - [ ] Rust 宿主内存直调 `rslib`，彻底删除 Protobuf IPC 序列化与 24 个 `.proto` 文件。
    - [ ] 删除 `backend.vala`, `ffi.h`, `ffi.rs`, `protobuf-vala` 整个子模块。
    - [ ] 针对 `D:\anki_data_test\` 运行单元测试，卡片调度打分后事务成功落盘，`notes.flds` 逐字节一致。
*   **Verification**:
    - `cargo test --package ranki --lib` (针对测试库执行)
    - 对比复习前后 `collection.anki2` 验证字段不变性
*   **Dependencies**: Task 1
*   **Files Touched**:
    - `src/anki/mod.rs`
    - `src/anki/decks.rs`
    - `src/anki/reviewer.rs`
    - 删除 `backend.vala`, `ffi.h`, `ffi.rs`, `protobuf-vala/`

---

### Task 3: 落实印刷级双缓冲 SPA 前端引擎 (离屏毫秒级预渲染与质感参数)
*   **Description**: 实现 `views/index.html` 与 `views/app.js`（纯 ES5 语法）。将裁切至 ~1.8MB 的 MathJax 2.7.9 核心及 20 款 TeX 矢量 `.woff` 字体归档入 `views/mathjax/`。实现 `#card-active` 与离屏 `#card-buffer`（`left: -9999px; visibility: hidden`）双缓冲架构与时序队列锁（Queue Lock）。注入 3.4 节印刷级排版秘方（纯黑 `#000000`、TeX 矢量字模、22px 黄金字号、48px 触控热区）。
*   **Acceptance Criteria**:
    - [ ] `views/app.js` 100% 通过 `node tools/check_es5.js` 门禁。
    - [ ] 双缓冲原地 DOM 切换感知时延 $\le 5\text{ms}$，时序队列锁有效拦截快速翻卡闪烁。
    - [ ] 38 道高难压测与真实考题在 MathJax 2.7.9 下 100% 渲染通过（0 崩溃、0 符号缺失）。
    - [ ] 底部评分触控热区 $\ge 48\text{px} \times 48\text{px}$，彻底移除旗子标记与横屏切换。
*   **Verification**:
    - `node tools/check_es5.js views/app.js` (GATE-04)
    - 本地浏览器离屏双缓冲切换打点测试
    - Kindle 墨水屏目视检验纯黑矢量字模与无微抖动网点
*   **Dependencies**: Task 2
*   **Files Touched**:
    - `views/index.html`
    - `views/app.js`
    - `views/style.css`
    - `views/mathjax/`
    - `tools/check_es5.js`

---

### Task 4: CI/CD 现代化与物理机 100 张卡片全量压测交付
*   **Description**: 重构 `.github/workflows/build-armel.yml`，剥离 Vala/Protobuf/automake 依赖，实现纯 Cargo 极速构建。部署最终纯 Rust 二进制至 Kindle PW3，执行连续 100 张复合公式卡片连续翻卡压测。验证各项指标并签署验收报告。
*   **Acceptance Criteria**:
    - [ ] CI 流水线纯 Rust 构建通过，三道机械门禁自动化全绿。
    - [ ] Kindle PW3 连续翻阅 100 张复合公式卡片，VmRSS 净增长 $\le 5\text{MB}$，无单调上升。
    - [ ] 空闲状态 CPU 占用率 0.0%。
    - [ ] 单一可执行文件体积 $\le 20\text{MB}$。
    - [ ] 生产库 `D:\anki_data\` 0 字节修改，交付报告归档。
*   **Verification**:
    - `python tools/check_symbols.py build/ranki-armel`
    - `python tools/check_dt_needed.py build/ranki-armel`
    - `python tools/check_dynsym.py build/ranki-armel`
    - 采集 Kindle `/proc/<pid>/status` 内存曲线并出具交付报告
*   **Dependencies**: Task 3
*   **Files Touched**:
    - `.github/workflows/build-armel.yml`
    - `Dockerfile.armel`
    - `tools/ci_build_armel.sh`
    - `docs/refactor/final-report.md`
