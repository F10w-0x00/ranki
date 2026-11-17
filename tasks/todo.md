# RAnki Refactor Tasks (tasks/todo.md)

## V1: Pure-Rust Host & Zero-Copy Engine (COMPLETED & VERIFIED)
- [x] **Task 0: 闭环当前 M0 基线 (Run #39 验证与动态符号隔离门禁)**
- [x] **Task 1: 建立统一 Web-SPA 纯 Rust 宿主骨架 (消灭 Vala 窗口与 TreeView)**
- [x] **Task 2: 零拷贝直连 Anki 原生核心与 WAL 事务防护 (消灭 Protobuf 与 Vala 后端)**
- [x] **Task 3: 落实印刷级双缓冲 SPA 前端引擎 (离屏毫秒级预渲染与质感参数)**

---

## V2: Anki-Authentic UI & Next-Card Pipeline Pre-Rendering (ACTIVE)

- [x] **Task 1: 纯净字体转码注入、44px 纸书排版与方案 A 评分栏**
  - [x] 1. 将 `C:\Users\Felyn\Documents\Storage\Font` 下的 Anthropic Sans (Text Regular & Italic) 无损转码为 WOFF 1.0 与 TTF 并置入 `views/fonts/anthropic/`
  - [x] 2. 注入 `@font-face` 声明，绑定西文 Anthropic Sans 与中文 SonyReader Ming（彻底移除官方宋体、仓耳今楷与方正字体）
  - [x] 3. 重塑 `views/style.css`，将正文字号精准标定为 44px（五号字 3.73mm 黄金阅读），行高 1.6，单行 22~23 字
  - [x] 4. 重塑顶部状态栏为 64px 极简顶栏（左侧 `<` 返回，中央牌组名，右侧三色复习计数徽章）
  - [x] 5. 重构底部操作栏为方案 A（未翻卡时为全宽圆角胶囊按键；翻卡后为 4 颗胶囊按键，上行 24px 预估时间，下行 32px 评级文字）
  - [x] 6. 在背面卡片注入 `<hr id="answer">` 标准细线分隔题面与答案，添加高对比度细滚动条

- [x] **Task 2: Rust 递归子牌组展开、流水线协议与 IPC 净化**
  - [x] 1. 在 `src/anki/mod.rs` 中实现 `flatten_deck_tree` 递归算法，支持多层级子牌组展开，返回包含 depth、full_name 与三色到期数的 `DeckItem`
  - [x] 2. 在 `src/ipc.rs` 中新增 `SHOW_CARD_PIPELINE` 消息下发定义（携带 `current` 与 `next` 卡片及计数统计）
  - [x] 3. 在 `src/ipc.rs` 中实现 `RATE_AND_FETCH_NEXT` 处理分支，原子提交 WAL 事务并返回 `PREPARE_NEXT_CARD`
  - [x] 4. 在 `src/anki.rs` 中实现调度器流水线卡片提取逻辑（支持平滑提取下一张卡 $N+2$）
  - [x] 5. 在 `src/ipc.rs` 中静默吸收 `CARD_FLIPPED` 消息，消除终端 `Unhandled action` 警告
  - [x] 6. 实现牌组结束时的 `DECK_FINISHED` 消息派发与 celebration 祝贺面板

- [x] **Task 3: 极仿 Anki 居中牌组大厅 & WebKit 4-Slot 乒乓缓冲切卡引擎**
  - [x] 1. 在 `views/index.html` 与 `views/style.css` 中重构 `#deck-view` 为 940px 水平居中容器，彻底告别方框
  - [x] 2. 实现 Anki 经典四列结构：`牌组 (Deck)` ── `新卡 (New)` ── `学习 (Learn)` ── `复习 (Due)`，全自动展开所有子牌组（带 `depth * 32px` 缩进与树形连接符）
  - [x] 3. 牌组行支持墨水屏反色按下态反馈，底部居中显示 `今日待复习卡片总数` 统计与胶囊退出按键
  - [x] 4. 在 `views/index.html` 中重构卡槽结构为 Group A (`slot-a-front/back`) 与 Group B (`slot-b-front/back`)
  - [x] 5. 在 `views/app.js` 中实现乒乓缓冲调度状态机与 MathJax 异步排版队列，实现 $\le 5\text{ms}$ 瞬时切卡
  - [x] 6. 落地时序状态锁（`isNextCardReady`），防止极速连击产生竞态；实现屏幕上/下 1/3 区域点击触发顺畅步进翻页
  - [x] 7. 牌组结束展示“🎉 今日复习完成”祝贺卡片；运行 `node tools/check_es5.js` 确保 100% 严格 ES5

- [x] **Task 4: CI Build, 4 Automated Gates, Kindle PW3 Deployment & Real-Device Verification**
  - [x] 1. 提交至 Git 分支 `refactor/m0-baseline` 并触发 GitHub Actions CI 交叉编译构建（Run ID: 44047432642 全绿通过）
  - [x] 2. 本地执行 4 道自动化门禁（GLIBC 2.18、DT_NEEDED、0 dynsym leak、ES5）确保全绿通过
  - [x] 3. 使用 `tools/deploy_to_kindle.py` 将产物与 views 部署至 Kindle `D:\extensions\ranki-dev\`（部署成功）
  - [x] 4. 绝不触碰生产数据库 `D:\anki_data\`（生产指纹 `de6efa895198` 100% 未触碰）
  - [ ] 5. Kindle PW3 真机复习 50+ 张公式卡片，核验 Anki 经典牌组大厅（居中+子牌组展开）、44px 纸书排版与零等待切卡
  - [ ] 6. 核验 `D:\ranki_logs\ranki_dev_stdout.log`，确认 0 `Unhandled action` 警告，WAL 正常落盘
