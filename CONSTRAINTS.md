# RAnki Project Constraints & Quality Bar (CONSTRAINTS.md)

> **Document Status**: Active Quality Contract (Synced to Approved Route B SPEC)
> **Authority**: Anchored on [docs/intent/kpw3-anki-daily-driver.md](docs/intent/kpw3-anki-daily-driver.md) & [SPEC-route-b-native-shell.md](SPEC-route-b-native-shell.md)
> **Last Updated**: 2026-11-20

---

## 一、 不可降低的底线铁律 (The Absolute Floor)

任何代码提交、重构 PR 或自动化 Agent **严禁**降低以下底线标准。违反任何一条，直接 CI 熔断或本地门禁拒绝：

1. **绝对数据安全红线**：
   - 严禁在开发/测试阶段读写或修改生产库 `D:\anki_data\`！
   - 所有调试、测试、单元测试、真机冒烟测试严格限定在 `D:\anki_data_test\`（经用户明确放行后方可切换主力库）。
   - 复习前后对比数据库，卡片原始字段内容（`notes.flds`）必须逐字节一致。
2. **换行符标准**：
   - 全仓库文件（Rust, C, Python, JS, HTML, CSS, Shell, Markdown）强制使用 **Unix LF (`\n`)**，严禁引入 Windows CRLF (`\r\n`)。
3. **零静默抑制与零硬编码密钥**：
   - 严禁新增任何全局编译警告抑制、`#[allow(warnings)]` 粗暴规避、ESLint disable 或忽略测试断言。
   - 严禁在仓库源码或部署脚本中硬编码任何 `hkey`、密码或个人账号信息。
4. **受控网络边界**：
   - 非用户主动点击同步/登录时，运行期间外部网络请求严格为 **0**；
   - 同步严格仅同步集合数据库（Normal / FullSync），**严禁下载或同步媒体文件**。

---

## 二、 机械四道自动化门禁 (Mechanical Hard Gates)

构建产物部署至 Kindle 前，必须 100% 通过以下四道自动化门禁（退出码必须为 0）：

| 门禁 ID | 门禁名称 | 阈值标准 | 验证命令 |
| :--- | :--- | :--- | :--- |
| **GATE-01** | **GLIBC 符号天花板** | 最高符号 $\le$ `GLIBC_2.20` (目标 $\le$ `GLIBC_2.18`) | `python tools/check_symbols.py build/ranki-armel` |
| **GATE-02** | **动态库依赖白名单** | 严格命中 Kindle OS 5.14.3 系统白名单库 | `python tools/check_dt_needed.py build/ranki-armel` |
| **GATE-03** | **静态符号隔离门禁** | `.dynsym` 中 `sqlite3_*`、`archive_*`、`json_*` 导出数 **严格为 0** | `python tools/check_dynsym.py build/ranki-armel` |
| **GATE-04** | **JavaScript ES5 & 2012 WebKit 门禁** | 严禁任何 ES6+ 语法及 2012 WebKit 缺失 API (`.closest`, `Promise`, `fetch`, `localStorage`, `requestAnimationFrame`) | `node tools/check_es5.js views/card.js` |

---

## 三、 性能与资源刚性量化指标 (Ratcheted Budgets)

| 维度 | 指标名称 | 刚性阈值 | 测量与验证方法 |
| :--- | :--- | :--- | :--- |
| **时延** | 题面 $\to$ 答案感知时延 | $\le 5\text{ms}$ | Card-ID 双缓冲原地 DOM 切换埋点日志 |
| **时延** | 预渲染命中切卡时延 | $\le 5\text{ms}$ | Card-ID 命中原地切换埋点日志 |
| **时延** | 冷启动首卡 / 未命中切卡耗时 | 预热 MathJax 单卡流畅渲染（切卡由 4 槽预渲染隐藏耗时） | MathJax 2.7.9 默认主力渲染 + 后台预渲染计时 |
| **能耗** | 无交互空闲 CPU 占用率 | **0.0%** | GTK/WebKit 事件主循环阻塞在 `epoll`（20s 计时仅看题时 1Hz 唤醒） |
| **内存** | 连续翻卡 100 张 VmRSS 净增 | $\le 5\text{MB}$ | `/proc/<pid>/status` 采样，无单调递增趋势 |
| **体积** | 单一二进制文件体积 | $\le 20\text{MB}$ | 纯 Rust Fat LTO + `arm-linux-gnueabi-strip` |

---

## 四、 架构与排版质量标准 (Route B Architecture & Texture Specs)

1. **路线 B 架构收敛**：
   - 宿主保留 1 个全屏 `GtkWindow` + `GtkNotebook`（牌组页为纯 PangoCairo 硬分页界面；复习页为原生 GTK2/PangoCairo 顶/底栏 + 独立 `52×52px` Cairo 20s 圆形倒计时 + 1 个常驻单页 `WebKitWebView` 卡片正文画布）；
   - 设置与登录使用原生 `GtkDialog` + `GtkEntry` 联动 Kindle LIPC 虚拟键盘；网络同步严格运行于后台工作线程，绝不阻塞 GTK 主循环；
   - 卡片正文 WebView 启动仅加载一次，宿主持久通信携带单调递增 `token` 且强校验 `card_id` 杜绝错卡。
2. **印刷级纸书居中排版**：
   - 卡片内容必须包裹在 `<div class="card kindle">` 内，严格保持官方 Anki **居中对齐**；
   - 正文与公式锁定纯黑 `#000000`（杜绝灰阶发虚），字体锁定 `Anthropic Sans` + `SonyReader Ming`，默认 `42px`（300 PPI 下约 3.56 mm ≈ 五号字 10.1 pt，支持 `28px–54px` 调节），行高 `1.56`；
   - 彩色文本统一做高对比黑白语义映射（红/橙警示虚线底纹、其他彩色粗体点线、高亮底色浅灰），音视频/图片统一显示简洁 `[不支持...]` 占位提示，防乱码过滤不可见控制符与 Emoji；
   - 唯一公式引擎采用 **MathJax 2.7.9**（`TeX-AMS_HTML` + `mhchem3` + `bbox` + `AMSmath`/`AMSsymbols` + 22 款本地 WOFF 字模，KPW3 WebKitGTK 1.0 嵌套公式零错位），启动于牌组大厅即后台预热，配合 Card-ID 4 槽预渲染与单帧原子显隐消除切卡/翻面等待与双帧残影（已彻底移除在 KPW3 上不兼容的 KaTeX）；
   - 超长公式通过 `-webkit-transform` 自适应等比缩放，严禁横向滚动条撑爆页面；
   - 全面禁用 Unicode Emoji，全界面零豆腐块 `□`。
3. **墨水屏人机工学与零残影翻页**：
   - 所有触控按钮热区 $\ge 48\text{px} \times 48\text{px}$，间距 $\ge 12\text{px}$；
   - 彻底禁用 `-webkit-overflow-scrolling: touch` 与拖动过程连续重绘；长卡由 GTK2/GDK 层拦截滑动/上下缘轻点，抬手单帧干脆跳转 85% 屏高（无翻页条遮挡）；
   - 20s 仿 Duo 圆形倒计时仅局部刷新 `52×52px` 独立 Cairo 区域，1 Hz 离散步进，翻面即停并隐藏，到零静默视觉提示；
   - 支持手动及按设定张数（默认 20 张）自动触发真硬件 GC16 全屏刷新清残影。

---

## 五、 事务与异常恢复规约 (WAL & Data Integrity)

1. 每次卡片评分、Undo、Bury、Suspend 均直接提交官方 `anki/rslib` 原子事务，并严格校验 `card_id`。
2. 退出牌组、同步前后及退出程序时，强制执行 `PRAGMA wal_checkpoint(TRUNCATE);`。
3. 严禁使用 `localStorage`（WebKit 1.0 `file://` 抛 DOM Exception 18）。
