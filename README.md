# RAnki — Kindle 墨水屏纸书级 Anki 复习客户端

**[中文版](#-中文说明-chinese) | [English](#-english-documentation)**

> **真 Anki 官方内核 (`anki/rslib 25.09.02`) · 原生 GTK2/PangoCairo 极速壳层 · 300 PPI 纸书级 MathJax 2.7.9 理工科排版**
> 专为越狱 **Kindle Paperwhite 3 (KPW3, 1072×1448 @ 300 PPI Carta, ARMv7 `armel`)** 及同代 Kindle OS 5.14+ 设备打造。

---

## 📖 中文说明 (Chinese)

### 1. 项目简介与核心功能

**RAnki (Route B)** 是一款运行在越狱 Kindle 电子书阅读器上的轻量、极速、纸书质感 Anki 每日复习主力客户端。它不是对 Anki 算法的简化仿写，而是直接在单进程内静态链接官方 **`anki/rslib 25.09.02`** Rust 核心库，保证调度与同步行为与桌面版 Anki 100% 一致。

#### 核心特性
- **官方 Anki 原生调度与操作**：
  - 完整支持 **SM-2** 与 **FSRS** 算法、四档评分（重来 / 困难 / 良好 / 简单，实时显示预计间隔）；
  - 顶栏实时居中显示官方 **`新卡 + 学习中 + 复习`** 三队列计数，并对当前卡片所属队列自动加粗下划线；
  - 原生支持 **撤销上一步 (`Undo`)**、**搁置当前卡 (`Bury`)**、**暂停当前卡 (`Suspend`)**、跨天自动取消搁置；
  - 根牌组待复习总数自动去重汇总，所有子牌组默认**自动展开**（亦可点击左侧矢量三角折叠）。
- **300 PPI 纸书级理工科（STEM）排版**：
  - 严格遵循官方 `<div class="card kindle">` 居中纸书版式，内置 **Anthropic Sans** 与 **SonyReader Ming（索尼明体）**，默认字号 **`42px`**（300 PPI 下物理字高约 3.56 mm ≈ 五号字 10.1 pt，支持 `28px–54px` 步进调节）；
  - 内置完整离线 **MathJax 2.7.9**（22 款本地 TeX WOFF 矢量字模 + `mhchem3` 化学方程式 `\ce{...}` + `\bbox` 重点公式框 + `AMSmath`/`AMSsymbols`/`cancel`/`enclose`），在 2012 WebKitGTK 1.0 上深度嵌套分式、多层根号、矩阵、张量上下标 **零错位**；
  - **列表与超宽公式自适应**：针对 `<ul>/<ol><li>` 内超宽公式采用显式尺寸容器 `.eq-scale-wrap` 等比缩放，彻底解决公式折行到圆点 `•` 下方或右侧溢界的问题；
  - **多色文本墨水屏黑白归一化**：红/橙警示字自动转为「纯黑粗体 + 浅灰底纹 + 虚线下划线」，蓝/绿/紫等彩色字转为「纯黑粗体 + 点状下划线」，`<mark>` 高亮转为浅灰胶囊，杜绝彩色字在黑白屏上发虚变淡；
  - **防乱码与不支持媒体优雅兜底**：自动剥离 Anki 双向隔离控制符（`\u2068`/`\u2069`）与 Emoji（杜绝 `□` 豆腐块）；遇到音频/视频/图片自动显示简洁的 `[不支持音频]` / `[不支持视频/嵌入内容]` / `[不支持图片]` 标签。
- **墨水屏专属交互与零残影引擎**：
  - **无翻页条长卡手势翻页**：彻底移除占空间的底部翻页条；由 GTK2/GDK 底层直接拦截触控——**上滑或轻点卡片下缘 35%** 干脆跳转下一屏（85% 屏高），**下滑或轻点卡片上缘 35%** 跳回上一屏；拖动过程 **0 中间帧重绘 = 0 拖影**，多页长卡在顶栏低调显示 `1/2` 页码；
  - **独立 `54×54px` 20 秒圆形专注倒计时**：右上角仿 iPhone Duo 风格纯黑白扇形倒计时，仅在看题面时以 1 Hz 独立局部重绘 `54×54px` 区域（绝不触发 WebView 重排），20 秒归零后呈现干净空白空心圆（不自动翻面），点击「显示答案」立即隐藏；
  - **单帧原子显隐 + 防撞车硬件 GC16 清残影**：MathJax 排版完成后才单帧显现卡片（消除“先闪原始 LaTeX 再出公式”的双帧残影）；在冷启动首屏、进退牌组与关闭弹窗后等待 Xorg 落稳（450 ms）再执行单次硬件级 `eips -f` 全刷，连续复习切卡期间 **0 突然黑闪打断**（也可随时手动一键清残影）；
  - **非阻塞后台 Wi-Fi 同步与原生虚拟键盘**：登录与集合同步运行于独立后台线程（仅同步数据库，**零媒体文件下载**，支持 `FullSync` 全量上传/下载选择）；设置页使用原生 `GtkEntry` 自动唤起 Kindle 系统 LIPC 虚拟键盘。

---

### 2. 整体架构 (Route B: Native GTK2/PangoCairo Shell + Resident Card Canvas)

```text
+-----------------------------------------------------------------------+
|                     RAnki 单一原生进程 (ranki-armel)                   |
|                                                                       |
|  +-----------------------------------------------------------------+  |
|  | 1. Native GTK2 / PangoCairo / Fontconfig / LIPC 壳层 (src/ui/)  |  |
|  |  - Page 0: 纯 PangoCairo 牌组大厅 (10行/页硬分页, 子牌组自动展开) |  |
|  |  - Page 1: 复习顶栏 (居中 新+学+复 / 页码 / 撤销 / ··· / 54x54圆)|  |
|  |            + GDK 零中间帧手势拦截层 + 复习底栏 (显示答案/四档评分)|  |
|  |  - Page 2: 原生 GtkEntry 设置与登录页 (联动 Kindle LIPC 虚拟键盘)|  |
|  |  - 纵向全宽操作弹窗 + 防抖硬件 GC16 全屏刷新 (/usr/sbin/eips -f)  |  |
|  +-----------------------------------------------------------------+  |
|                   |                               ^                   |
|   execute_script  | (CardEngine.*)                | notify::title     |
|                   v                               | (RA:ACTION:TOKEN) |
|  +-----------------------------------------------------------------+  |
|  | 2. 常驻单页 WebKitGTK 1.0 卡片画布 (views/card.{html,css,js})    |  |
|  |  - 整个生命周期仅加载 1 次，于牌组大厅阶段后台预热 MathJax 2.7.9 |  |
|  |  - Card-ID 强校验 4 槽双缓冲预渲染 (Group A 正/反 + Group B 正/反)|  |
|  |  - 同模板 CSS 变更缓存 (跳过重复整页样式重算) + 单帧原子显隐      |  |
|  +-----------------------------------------------------------------+  |
|                   |                               ^                   |
|                   v                               |                   |
|  +-----------------------------------------------------------------+  |
|  | 3. 官方 Anki 核心引擎 (src/anki/ -> anki/rslib 25.09.02)         |  |
|  |  - 零 Protobuf / 零跨进程开销，直接调用 Collection & Scheduler   |  |
|  |  - 后台独立线程 Tokio 运行时执行 AnkiWeb HTTPS 同步 (0 媒体下载) |  |
|  |  - 关键节点强制 PRAGMA wal_checkpoint(TRUNCATE) 保障断电数据安全 |  |
|  +-----------------------------------------------------------------+  |
+-----------------------------------------------------------------------+
```

---

### 3. 相比原版 RAnki（及早期全屏 Web 方案）的核心改进

| 维度 | 原版 RAnki (`Vala + Protobuf`) / 早期全屏 Web-SPA | 本项目 (**Route B: 原生壳层 + 常驻卡片画布**) |
| :--- | :--- | :--- |
| **后端架构** | Vala 界面层 + C 胶水层 + Protobuf 序列化调用 `rslib` | **100% 纯 Rust 零拷贝直连 `anki/rslib 25.09.02`**，无 Vala/Protobuf 开销 |
| **防错卡安全** | 预渲染仅按顺序盲切，点 `Again`/撤销/搁置同源卡后易发生“看 A 评 B”静默错卡 | **Card-ID 强校验流水线 (`AdvanceResult`)**：预渲染槽位 ID 与调度器真实下一张不一致时自动降级即时渲染（`PREFETCH_MISS` 自愈） |
| **同模板切卡耗时** | 每次切卡重写 `<style>` 触发 2012 WebKit 全文档样式重算（耗费 200–1400 ms） | **CSS 指纹缓存 + 4 槽双缓冲预渲染**，同模板切卡与翻面跳过重算，瞬时切换 |
| **数学与化学公式** | 原版需手动下载 MathJax；早期改用 KaTeX 在 2012 WebKit 1.0 上深度嵌套公式错位或抛 JS 异常 | **内置完整离线 MathJax 2.7.9 + 22 款 WOFF 字体 + `mhchem3` + `\bbox`**，嵌套根式/连分数/化学反应零错位，列表宽公式自动包裹 `.eq-scale-wrap` 不折行 |
| **长卡翻页体验** | 传统滚动条或底部翻页条遮挡内容、手指拖拽触发连续重绘产生“文字泥潭”残影 | **无翻页条 + GDK 底层拦截滑动/上下缘 35% 轻点**，拖动 0 中间帧，单帧跳 85% 屏高 |
| **残影控制** | 拖拽残影严重；MathJax 先刷原始 LaTeX 再出公式产生双帧叠影；弹窗关闭后留灰框 | **单帧原子显隐 (`visibility: hidden -> visible`)** + 冷启动/进退牌组/关弹窗 450 ms 稳态防撞车 **GC16 硬件全刷**，连续切卡期间 0 突然闪屏 |
| **弹窗与输入法** | WebKit 内 `<input>` 在 Kindle 上无法自动弹出系统键盘；GTK 默认横向弹窗按钮溢出屏幕 | **原生 `GtkEntry` 自动联动 Kindle LIPC 键盘**（布局居于上半屏不被键盘遮挡）；弹窗改为**纵向全宽大按钮列表** |
| **同步与存储安全** | 同步会下载全部音频/图片塞满 Kindle 存储；静态 `sqlite3` 符号外泄易与系统 `libsoup` 冲突崩溃 | **仅同步集合数据库（0 媒体下载）**，支持 `FullSync` 双向选择；链接器重写并隐藏全部 `sqlite3_*` 符号（`.dynsym` 0 泄漏），退出自动 `WAL checkpoint` |

---

### 4. 详细安装与部署指南

#### 前置条件
1. 一台**已越狱 (Jailbroken)** 的 Kindle 设备（主力验证机型：**Kindle Paperwhite 3 / KPW3**，固件 `5.14.3` ~ `5.18.x`，已安装 **KUAL**）。
2. （推荐但非必须）在 Kindle 根目录 `/mnt/us/fonts/`（即 USB 盘符下的 `D:\fonts\`）放入 `SonyReader Ming.ttf`，可获得最佳中文衬线印刷观感；若未安装，程序也会自动使用内置的 `Anthropic Sans` 与系统宋体正常显示。

---

#### 方式一：通过 USB 一键安全部署（推荐）

当你将 Kindle 通过 USB 数据线连接到电脑（电脑识别出 Kindle 盘符，如 `D:\`）后，在项目根目录执行：

```bash
# 1. (可选) 从最新的 GitHub Actions CI 拉取编译好的 ranki-armel 二进制文件
python tools/download_artifact.py

# 2. 一键安全部署至 Kindle 的 /mnt/us/extensions/ranki-dev/
python tools/deploy_to_kindle.py
```

**部署脚本安全保障**：
- 自动检测 Kindle 盘符（`D:\` ~ `H:\`）；
- 部署前后自动对生产目录 `D:\anki_data` 计算 SHA-256 指纹校验，**保证 0 字节修改你的原有数据**；
- 自动更新 `D:\extensions\ranki-dev\ranki-armel`、`views/` 与 `ranki.sh`（严格使用 Unix `LF` 换行符）；
- **自动保留**设备上已有的 `D:\extensions\ranki-dev\config.ini`（不会覆盖你已登录的 AnkiWeb Token 和字号设置）。

---

#### 方式二：手动复制安装

如果你直接下载构建产物或手动打包安装，请将 Kindle 连接电脑，按以下目录结构放置文件：

```text
D:\ (即 Kindle 上的 /mnt/us/)
├── anki_data_test\                  # 默认隔离测试数据库目录（可放测试用 collection.anki2）
├── anki_data\                       # 你的正式主力 Anki 数据库目录
├── documents\
│   └── RAnki_Dev.sh                 # (可选) 书架快捷启动脚本，点击即可从 Kindle 图书馆直接打开
└── extensions\
    └── ranki-dev\
        ├── ranki-armel              # 主程序二进制文件
        ├── ranki.sh                 # 启动脚本 (必须为 Unix LF 换行符!)
        ├── config.ini               # 配置文件 (首次运行或部署自动生成)
        └── views\                   # 常驻卡片画布与离线字体/公式资源
            ├── card.html
            ├── card.css
            ├── card.js
            ├── fonts\anthropic\     # 内置 Anthropic Sans 字体
            └── mathjax\             # 完整离线 MathJax 2.7.9 + 22 款 TeX WOFF 字体
```

##### 书架启动脚本 `D:\documents\RAnki_Dev.sh`（可选）
如果你想直接在 Kindle 首页书架点击图标启动（无需每次先开 KUAL），可在 `D:\documents\RAnki_Dev.sh` 放入以下内容（注意使用 **Unix LF** 换行符）：

```bash
#!/bin/bash
# Name: RAnki
# Author: RAnki
# DontUseFBInk

/mnt/us/extensions/ranki-dev/ranki.sh
```

---

#### 配置文件 (`config.ini`) 与切换正式库说明

`/mnt/us/extensions/ranki-dev/config.ini` 结构极简：

```ini
[General]
collection_dir=/mnt/us/anki_data_test
hkey=
sync_server=
font_size_px=42
```

- **`collection_dir`**：
  - 默认指向隔离测试目录 `/mnt/us/anki_data_test`；
  - 当你在测试库验收满意、准备切换为日常主力复习时，只需将其改为 `/mnt/us/anki_data`（也可以在设置页登录 AnkiWeb 后直接将云端牌组同步至本地）。
- **`hkey`**：AnkiWeb 登录安全 Token。在软件内点击右上角「**设置**」输入邮箱与密码登录后会自动换取并保存 Token（**明文密码用完即销毁，绝不落盘**）。
- **`sync_server`**：自定义同步服务器地址（留空则使用官方 AnkiWeb）。
- **`font_size_px`**：卡片正文字号（默认 `42`，范围 `28–54`，可在软件「设置」中点击 `A-` / `A+` 实时调整并自动保存）。

---

### 5. 操作速查表

| 界面 / 场景 | 操作方式 | 说明 |
| :--- | :--- | :--- |
| **牌组大厅翻页** | 列表区**上下/左右滑动** 或 底部 `< 上一页` / `下一页 >` | 每页 10 行硬分页，零滚动条残影 |
| **折叠 / 展开子牌组** | 点击父牌组名称左侧的 **`▾` / `▸` 三角区域** | 默认全部自动展开 |
| **长卡片往下翻页** | 在卡片正文区 **向上滑动** 或 **轻点下缘 35% 区域** | 单帧向下跳转 85% 视口高度，顶栏显示 `1/2` |
| **长卡片往上翻页** | 在卡片正文区 **向下滑动** 或 **轻点上缘 35% 区域** | 单帧向上跳转 85% 视口高度 |
| **20s 圆形专注计时** | 进入卡片题面自动开始（右上角 54×54px 圆环） | 20 秒归零变为空心圆；点击「显示答案」立即停止并隐藏 |
| **撤销 / 搁置 / 暂停** | 复习页顶栏点 `撤销`，或点 `···` 打开纵向操作菜单 | 全部直接写入官方 `anki/rslib` 事务并支持撤销 |
| **清残影 (GC16)** | 冷启动/进退牌组/关弹窗自动稳态单刷，复习期间 0 突然黑闪，随时可手动点 `刷新` | 调用硬件级全屏刷新清除墨水屏电荷残留 |

---

### 6. 编译与四道自动化门禁 (CI & Mechanical Gates)

本项目通过 GitHub Actions + Docker (`Dockerfile.armel`, Debian Buster `armel` sysroot + Rust 1.89) 进行交叉编译，所有构建产物必须 100% 通过以下四道机械门禁：

```bash
# GATE-01: GLIBC 符号版本天花板 (要求 <= GLIBC_2.20，当前实测最高仅为 GLIBC_2.18)
python tools/check_symbols.py build/ranki-armel

# GATE-02: 动态库依赖白名单 (14/14 全部命中 Kindle OS 5.14.3 原生系统库)
python tools/check_dt_needed.py build/ranki-armel

# GATE-03: 静态符号隔离门禁 (.dynsym 中 sqlite3_* / archive_* / json_* 泄漏数严格为 0)
python tools/check_dynsym.py build/ranki-armel

# GATE-04: 前端 ES5 & 2012 WebKit 兼容性门禁 + CardEngine 单元测试 + 876 条公式回归门禁
node tools/check_es5.js views/card.js
node tools/test_card_engine.js
node tools/test_mathjax_bundle.js
```

---
---

## 🌐 English Documentation

### 1. Overview

**RAnki (Route B)** is a fast, paper-book aesthetic Anki flashcard client purpose-built for jailbroken **Kindle Paperwhite 3 (KPW3, 1072×1448 @ 300 PPI Carta E-ink, ARMv7 `armel` soft-float, Kindle OS 5.14.3+)** and compatible Kindle e-readers.

Instead of reimplementing Anki's scheduling algorithm or running a multi-language Vala/Protobuf bridge, RAnki links **directly against the official Rust `anki/rslib 25.09.02` engine** in a single native binary (`~15.1 MB`), combining:
1. **Native GTK2 / PangoCairo / Fontconfig / LIPC Shell** for zero-ghosting paginated deck browsing, centered active-queue review chrome (`New + Learn + Review`), an isolated `54×54px` 1 Hz 20-second iPhone Duo-style circular focus timer, GDK-level zero-bar swipe/tap paging, vertical full-width action menus, and native `GtkEntry` login/settings wired to Kindle's LIPC virtual keyboard;
2. **Resident Single-Page WebKitGTK 1.0 Card Canvas** (`views/card.html`, `card.css`, `card.js`) with official `<div class="card kindle">` centered paper-book typography (`Anthropic Sans` + `SonyReader Ming`, default `42px` @ 300 PPI), offline **MathJax 2.7.9** (`TeX-AMS_HTML` + `mhchem3` + `bbox` + 22 local WOFF fonts), and a **Card-ID-verified 4-slot double-buffered pre-rendering pipeline**.

### 2. Key Improvements over Upstream RAnki

- **Zero Vala, Zero Protobuf, Zero Desync**: Directly calls Rust `anki/rslib` in-process and verifies `card_id` on every prefetch swap (auto-healing via `PREFETCH_MISS` when `Again`, `Undo`, or Cloze sibling burying alters the queue).
- **10× Faster Same-Template Card Swaps**: Caches applied notetype CSS so WebKitGTK 1.0 skips redundant full-document style recalculations across pre-rendered slots.
- **Rock-Solid Nested Math & Chemistry on 2012 WebKit**: Pre-warmed offline **MathJax 2.7.9** renders deep continued fractions, multi-layer radicals, matrices, and `\ce{...}` chemistry with zero misalignment; wide inline equations inside `<li>` lists are wrapped in explicit `.eq-scale-wrap` layout boxes so bullets never break onto a separate line.
- **Zero-Bar Long-Card Paging**: GDK intercepts swipe-up / tap-bottom-35% (next page) and swipe-down / tap-top-35% (previous page) with zero intermediate drag frames and a subtle `1/2` indicator in the top bar.
- **Smart Hardware GC16 Ghosting Control**: Atomic single-frame MathJax reveal (`visibility: hidden -> visible`) plus settle-delayed (450 ms, zero EPDC collision) hardware GC16 refreshes (`eips -f`) on cold start, deck entry/exit, and dialog close — zero disruptive mid-review flashes, with manual refresh available anytime.
- **DB-Only Non-Blocking Wi-Fi Sync**: Runs on a background worker thread with zero UI freeze and **never downloads media files** to preserve Kindle storage.

### 3. Quick Installation

1. Connect your jailbroken Kindle via USB (mounted e.g. at `D:\`).
2. Run the safe deployer:
   ```bash
   python tools/download_artifact.py
   python tools/deploy_to_kindle.py
   ```
   *(The script verifies 0-byte modification on `/mnt/us/anki_data`, deploys `ranki-armel` and `views/` to `/mnt/us/extensions/ranki-dev/`, and preserves your existing `config.ini`.)*
3. Safely eject USB and launch **RAnki Dev** from your Kindle library (`/mnt/us/documents/RAnki_Dev.sh`) or KUAL.
4. To switch from the isolated test database (`/mnt/us/anki_data_test`) to your production collection (`/mnt/us/anki_data`), edit `collection_dir=/mnt/us/anki_data` in `/mnt/us/extensions/ranki-dev/config.ini`.
