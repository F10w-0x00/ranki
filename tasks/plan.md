# RAnki V2 Execution Plan: Anki-Authentic UI & Next-Card Pipeline (tasks/plan.md)

> **Authority**: SPEC-v2-anki-ui-and-pipeline-prerender.md (v2.2.0), HANDOFF.md v5.0.0  
> **Methodology**: Spec-Driven Development, Vertical Slicing, Continuous Gated Verification  
> **Target Device**: Amazon Kindle Paperwhite 3 (Model DP75SDI, 1072x1448 @ 300 PPI, 90.5mm x 122.0mm active area, WebKitGTK 1.0, Linux 3.0.35)  
> **Status**: Ready for Human Review  

---

## 1. 物理屏幕参数与排版几何标定 (Kindle PW3 300 PPI Calibration)

### 1.1 硬件光学与物理映射公式
- 屏幕对角线：6.0 英寸（152.4 mm），有效显示幅面 **$90.5\text{ mm} \times 122.0\text{ mm}$**。
- 物理像素：$1072 \times 1448$ 像素，像素密度 **300 PPI**（每毫米约 $11.81$ 像素点）。
- 物理换算公式：$\text{物理尺寸(mm)} = \frac{\text{CSS像素(px)}}{300} \times 25.4$。

### 1.2 字体字号科学阶梯（对齐实体教科书出版物五号/小四号标准）
| 界面区域 | 目标 CSS 字号 | 物理高度 (mm) | 出版物对照与视觉效果 | 人机工学与阅读体验设计依据 |
| :--- | :--- | :--- | :--- | :--- |
| **卡片正文 (Body)** | **44px** | **3.73 mm** | **标准五号字 (10.5pt)** | 经典出版物标准正文字号。双侧留白 40px 时，单行排布 **22~23 个汉字**，完全落在人眼单次扫视最舒适的黄金阅读区（20-26字/行），彻底消灭阅读串行与疲劳感。 |
| **正文行高 (Line-height)** | **1.6** (70px) | **5.93 mm** | — | 为公式上下标（如 $\ce{Fe2O3}$、$\frac{U^2}{R}t$）、分式留足垂直呼吸空间，确保公式与文字行间绝不发生碰撞重叠。 |
| **卡片标题 (Card Title)** | **52px** (加粗) | **4.40 mm** | **小四号/四号字 (12~14pt)** | 题眼、知识点名称一眼醒目定位，层级分明。 |
| **公式缩放 (MathJax Scale)**| **115%** | 上下标 $\ge 2.6\text{mm}$ | — | 自动平衡 MathJax TeX 符号与 44px 汉字的视觉重心，角标与分母在 300 PPI 墨水屏上清晰可辨，绝不糊成小黑点。 |
| **评分按键 (Rating Buttons)**| 上行: **24px**<br>下行: **32px** (加粗) | 上: 2.03 mm<br>下: 2.71 mm | — | **方案 A 双行**：上行小字预估间隔（如 `<10m`、`1d`），下行大字评级文字（如 `良好`、`Good`），单键高度 110px，墨水屏触控盲操极准。 |
| **顶部状态栏 (Header)** | 牌组名: **28px**<br>计数徽章: **26px** | 2.37 mm / 2.20 mm | — | 64px 极简顶栏，右侧显示三色复习数徽章（`[ 0 | 12 | 85 ]`）。 |

---

## 2. 纯净字模体系与资产管理 (Anthropic Sans + 索尼明朝体)

### 2.1 严格排除声明 (Strict Exclusions)
- **坚决彻底排除**：
  - ❌ 系统官方精修宋体（官方宋体横线过细、弱光易虚化）
  - ❌ `仓耳今楷01 W03`
  - ❌ `FZYueKaiZhenS M`（方正字库）
- **唯一法定正文字体**：
  - **西文 / 公式 / 数字**：`AnthropicSans_Roman`（**Anthropic Sans Web Text Regular**）+ `AnthropicSans_Italic`（**Anthropic Sans Web Text Regular Italic**）
  - **中文正文**：严格唯一指定为 **索尼明朝体 (`SonyReader Ming.ttf`)**

### 2.2 字体转码与加载
- 使用 `fonttools` 将 Anthropic Sans 转为 WOFF 1.0（~146KB）与 TTF（~318KB），存入 `views/fonts/anthropic/`，供 WebKit 1.0 高性能加载。
- `SonyReader Ming` 通过 `@font-face` 直接挂载设备路径 `file:///mnt/us/fonts/SonyReader%20Ming.ttf` 与 `local("SonyReader Ming")`。
- CSS 栈纯净定义：
  ```css
  font-family: "Anthropic Sans Web", "SonyReader Ming", serif;
  ```

---

## 3. 牌组大厅极仿 Anki 经典居中架构 (Anki-Authentic Deck Browser)

### 3.1 视觉几何与居中布局
- 居中容器宽度：`width: 940px; margin: 40px auto;`，两端留出平衡美感的呼吸留白。
- 表头经典四列：`牌组 (Deck)` ── `新卡 (New)` ── `学习 (Learn)` ── `复习 (Due)`。
- 行高 $\ge 76\text{px}$，树形缩进：`padding-left: depth * 32px`，搭配树形连接符号。
- 底部居中统计：`今日已复习 N 张卡片` 与优雅胶囊退出按钮。
- 点击行具备墨水屏黑白即时反色按下态反馈。

### 3.2 树形递归展开
- 修改 `src/anki/mod.rs`：利用前序递归算法扁平化 `col.deck_tree(None)`，输出包含 `depth`、`full_name` 与三色到期数的 `DeckItem` 列表。
- 前端自动展开所有层级子牌组，用户可直接点击任意子牌组进入独立复习。

---

## 4. Architecture Dependency Graph

```
[Task 1: 纯净字体转码注入、44px 纸书排版与方案 A 评分栏]
       │ (Anthropic Sans + SonyReader Ming, 彻底清除官方宋体/楷体, 8px 圆角, 1.5px 边框)
       ▼
[Task 2: Rust 递归子牌组展开、流水线协议与 IPC 净化]
       │ (flatten_deck_tree 递归解开子牌组, SHOW_CARD_PIPELINE, RATE_AND_FETCH_NEXT, 静默 CARD_FLIPPED)
       ▼
[Task 3: 极仿 Anki 居中牌组大厅 & WebKit 4-Slot 乒乓缓冲切卡引擎]
       │ (940px 居中 Anki 表格, 子牌组自动展开, Group A/B DOM 四槽位, MathJax 异步排版, <=5ms 切卡, ES5 门禁)
       ▼
[Task 4: CI Build, 4 Automated Gates, Kindle PW3 Deployment & Real-Device Verification]
         (Local gates, GitHub Actions CI, deploy to D:\extensions\ranki-dev, on-device review)
```

---

## 5. Detailed Task Breakdown

### Task 1: 纯净字体转码注入、44px 纸书排版与方案 A 评分栏
* **Description**:
  1. 运行本地转码脚本，将 `C:\Users\Felyn\Documents\Storage\Font` 下的 `AnthropicSans_Roman` 与 `AnthropicSans_Italic` 转为 WOFF 1.0 / TTF 资产并置入 `views/fonts/anthropic/`。
  2. 重塑 `views/style.css`：
     - 正文基准字号提升至 **44px**（物理高度 $3.73\text{ mm}$，出版物五号字标准），行高 **1.6**，单行 22~23 字。
     - 注入 `@font-face` 声明：西文锁定 `Anthropic Sans Web`，中文严格唯一锁定 `SonyReader Ming`（彻底移除系统官方宋体、仓耳今楷与方正字体）。
     - 边框统一为 `1.5px` 精致黑线，圆角统一为 `border-radius: 8px`，彻底清除 3px 粗重方框。
     - 顶部 64px 极简顶栏（左侧 `<` 返回，中央牌组名，右侧三色复习计数徽章）。
     - 底部 110px 方案 A 双行胶囊评分栏（上行 24px 预估时间，下行 32px 评级文字，支持 `:active` 黑白反色）。
     - 正文卡片加入 `<hr id="answer">` 标准细线与高对比度细滚动条（`::-webkit-scrollbar`）。
* **Acceptance Criteria**:
  - [ ] `views/fonts/anthropic/` 包含可被 WebKit 1.0 加载的 WOFF 1.0 / TTF 格式 Anthropic 字体。
  - [ ] CSS 正文字号严格标定为 44px，行高 1.6，卡片标题 52px。
  - [ ] 字体栈纯净为 `"Anthropic Sans Web", "SonyReader Ming", serif;`，无任何楷体或官方宋体声明。
  - [ ] 底部操作栏严格落地方案 A（双行文字：预估时间 + 评级名称），圆角 8px，边框 1.5px。
  - [ ] 顶部 Header 为 64px 极简条，右侧包含复习徽章。
* **Verification**:
  - 本地静态资源检查与字体解析验证。
* **Dependencies**: None
* **Files Touched**:
  - `views/fonts/anthropic/*`
  - `views/style.css`
  - `views/index.html`

---

### Task 2: Rust 递归子牌组展开、流水线协议与 IPC 净化
* **Description**:
  1. 递归展开牌组树：在 `src/anki/mod.rs` 中重构 `get_deck_list()`，编写 `flatten_deck_tree` 递归遍历整棵牌组树，输出包含深度 `depth`、`full_name` 与三色到期数的 `DeckItem` 列表，彻底解决子牌组被截断丢失的问题。
  2. 新增 `SHOW_CARD_PIPELINE` IPC 消息：一次性下发 `current` 卡片、预取 `next` 卡片以及牌组剩余统计（`new_count`, `learn_count`, `review_count`）。
  3. 新增 `RATE_AND_FETCH_NEXT` IPC 处理分支：原子提交 WAL 事务更新评分，并从调度器中预取下一张新卡片 $N+2$。
  4. 返回 `PREPARE_NEXT_CARD`：向前端下发后续卡片，通知前端后台离屏填充。
  5. 牌组结束支持：牌组复习完毕或进入空牌组时，下发 `DECK_FINISHED` 消息，携带已复习张数。
  6. 日志净化：在 `src/ipc.rs` 中明确吸收 `CARD_FLIPPED` 消息，彻底消除终端 `[IPC] Unhandled action: 'CARD_FLIPPED'` 噪点。
* **Acceptance Criteria**:
  - [ ] `get_deck_list()` 递归返回全部层级子牌组（包含正确 depth、full_name、三色计数）。
  - [ ] `src/ipc.rs` 支持流水线 IPC 协议并静默吸收 `CARD_FLIPPED`。
  - [ ] 调度器平滑提取下张卡片，SQLite 事务即时原子 Commit。
  - [ ] 牌组复习完毕时，正确发出 `DECK_FINISHED`，不产生后端恐慌（panic）。
* **Verification**:
  - `cargo check --target arm-unknown-linux-gnueabi`（CI/本地容器内）
* **Dependencies**: Task 1
* **Files Touched**:
  - `src/anki/mod.rs`
  - `src/ipc.rs`
  - `src/main.rs`

---

### Task 3: 极仿 Anki 居中牌组大厅 & WebKit 4-Slot 乒乓缓冲切卡引擎
* **Description**:
  1. **极仿 Anki 居中牌组大厅**：
     - 在 `views/index.html` 与 `views/style.css` 中重构 `#deck-view`：居中 940px 容器，大方优雅。
     - 呈现 Anki 官方经典的四列结构：`牌组 (Deck)` ── `新卡 (New)` ── `学习 (Learn)` ── `复习 (Due)`。
     - 自动展开所有多级子牌组，通过层级缩进（`depth * 32px`）清晰展现父子嵌套关系。
     - 到期数字右对齐，点击行触发墨水屏反色按下态反馈，可直接点击任意子牌组开始复习。
     - 底部居中展示学习进度统计（`今日已复习 N 张卡片`）与优雅胶囊退出按钮。
  2. **DOM 4 槽位乒乓缓冲与预排版引擎**：
     - DOM 维持 4 个物理容器：Group A (`slot-a-front`, `slot-a-back`) 与 Group B (`slot-b-front`, `slot-b-back`)。
     - 接收 `SHOW_CARD_PIPELINE` 时，Group A 立即展示，Group B 置于离屏（`position: absolute; left: -9999px`）。
     - `MathJax.Hub.Queue` 后台静默排版 Group B，在用户阅读卡片 $N$ 期间后台 100% 完成排版。
     - 点击评分瞬间切换活跃 Group 指针（**$\le 5\text{ms}$ 瞬时切卡**，消灭 500ms 等待）。同时向 Rust 发送 `RATE_AND_FETCH_NEXT`。
     - 接收 `PREPARE_NEXT_CARD` 时，将退回后台的旧 Group 槽位清空并注入新卡，触发后台静默 Typeset。
     - 牌组结束时：展示“🎉 恭喜完成！今日无更多待复习卡片”结算面板与返回按钮。
     - 快速翻页手势：点击屏幕上 1/3 向上滚动 200px，点击屏幕下 1/3 向下滚动 200px。
     - 门禁合规：全量 JavaScript 必须 100% 符合严格 ES5 规范。
* **Acceptance Criteria**:
  - [ ] 牌组大厅 940px 居中排布，严格呈现经典 Anki 四列结构（牌组 / 新卡 / 学习 / 复习）。
  - [ ] 牌组界面全自动展开所有子牌组，缩进与层级层次分明。
  - [ ] 翻卡与切卡时延均 $\le 5\text{ms}$。
  - [ ] 时序锁保证：即使极端快速点击评分，若后台预排版未完会自动挂起并在就绪瞬间切出，不白屏、不报错。
  - [ ] 点击屏幕上/下 1/3 触发长文本顺畅平滑翻页。
  - [ ] `node tools/check_es5.js` 门禁断言 100% PASS。
* **Verification**:
  - `node tools/check_es5.js`
* **Dependencies**: Task 1, Task 2
* **Files Touched**:
  - `views/app.js`
  - `views/index.html`
  - `views/style.css`

---

### Task 4: CI Build, 4 Automated Gates, Kindle PW3 Deployment & Real-Device Verification
* **Description**:
  提交代码至 GitHub 触发 CI 构建，本地执行 4 道自动化门禁，部署至 Kindle PW3 验证：
  1. 提交至 Git 分支 `refactor/m0-baseline`，触发 GitHub Actions CI 交叉编译构建。
  2. 本地执行 4 道自动化门禁：
     - GATE-01: GLIBC 符号天花板 $\le 2.20$（实测 GLIBC_2.18）
     - GATE-02: DT_NEEDED 动态依赖全部命中 Kindle 16 项白名单
     - GATE-03: 静态库私有符号泄漏数严格为 0
     - GATE-04: 前端严格 ES5 验证 100% PASS
  3. 使用一键部署脚本安全部署至 Kindle `D:\extensions\ranki-dev\`（**绝对红线：严禁触碰生产库 `D:\anki_data\`**）。
  4. Kindle PW3 真机核验：
     - 牌组大厅是否完全居中、高度神似 Anki、完整自动展开多级子牌组；
     - 44px 纸书排版与 Anthropic Sans + 索尼明朝体纯净字形呈现；
     - 切卡是否达到 $\le 5\text{ms}$ 无缝零等待；
     - 退出时 WAL 正常 Checkpoint。
  5. 检查 `D:\ranki_logs\ranki_dev_stdout.log`，确认 0 `Unhandled action` 警告。
* **Acceptance Criteria**:
  - [ ] CI 构建流程全绿通过。
  - [ ] 4 道自动化门禁全部打印 `[PASS]`，退出码均为 0。
  - [ ] Kindle PW3 实测子牌组完整展开，大厅居中极像 Anki，排版如实体书，切卡无停顿。
  - [ ] 生产数据库 `D:\anki_data\` 0 字节改动。
* **Verification**:
  - 4 道门禁自动化脚本
  - Kindle 物理日志与现场审查
* **Dependencies**: Task 1, Task 2, Task 3
* **Files Touched**:
  - `D:\extensions\ranki-dev\` (部署目标)
