# RAnki Project Constraints & Quality Bar (CONSTRAINTS.md)

> **Document Status**: Immutable Quality Contract  
> **Authority**: Anchored on HANDOFF.md v5.0.0-Reviewed  
> **Last Updated**: 2026-11-16  

---

## 一、 不可降低的底线铁律 (The Absolute Floor)

任何代码提交、重构 PR 或自动化 Agent **严禁**降低以下底线标准。违反任何一条，直接 CI 熔断或本地门禁拒绝：

1. **绝对数据安全红线**：
   - 严禁读写或修改生产库 `D:\anki_data\`！
   - 所有调试、测试、单元测试、真机冒烟测试严格限定在 `D:\anki_data_test\`。
   - 复习前后对比数据库，卡片原始字段内容（`notes.flds`）必须逐字节一致。
2. **换行符标准**：
   - 全仓库文件（Rust, C, Python, JS, HTML, CSS, Shell, Markdown）强制使用 **Unix LF (`\n`)**，严禁引入 Windows CRLF (`\r\n`)。
3. **零静默抑制**：
   - 严禁新增任何全局编译警告抑制、`#[allow(warnings)]` 粗暴规避、ESLint disable 或忽略测试断言。
4. **零网络请求**：
   - 运行期间外部网络请求严格为 **0**（纯离线电子纸设备环境）。

---

## 二、 机械三道自动化门禁 (Mechanical Hard Gates)

构建产物部署至 Kindle 前，必须 100% 通过以下三道自动化门禁（退出码必须为 0）：

| 门禁 ID | 门禁名称 | 阈值标准 | 验证命令 |
| :--- | :--- | :--- | :--- |
| **GATE-01** | **GLIBC 符号天花板** | 最高符号 $\le$ `GLIBC_2.18` (系统上限 2.20) | `python tools/check_symbols.py <binary>` |
| **GATE-02** | **动态库依赖白名单** | 严格命中 Kindle 固件 16 项系统白名单库 | `python tools/check_dt_needed.py <binary>` |
| **GATE-03** | **静态符号隔离门禁** | `.dynsym` 中 `sqlite3_*`、`archive_*`、`json_*` 导出数 **严格为 0** | `python tools/check_dynsym.py <binary>` |
| **GATE-04** | **JavaScript ES5 刚性门禁** | 严禁任何 ES6+ 语法 (`let`, `const`, `=>`, `class`, `Promise`) | `node tools/check_es5.js views/app.js` |

---

## 三、 性能与资源刚性量化指标 (Ratcheted Budgets)

| 维度 | 指标名称 | 刚性阈值 | 测量与验证方法 |
| :--- | :--- | :--- | :--- |
| **时延** | 题面 $\to$ 答案感知时延 | $\le 5\text{ms}$ | 离屏双缓冲原地 DOM 切换埋点日志 |
| **时延** | 后台静默排版耗时 (P95) | $\le 500\text{ms}$ | MathJax 静默渲染耗时打点（日常考题） |
| **能耗** | 无交互空闲 CPU 占用率 | **0.0%** | GTK/WebKit 事件主循环阻塞在 `epoll` |
| **内存** | 连续翻卡 100 张 VmRSS 净增 | $\le 5\text{MB}$ | `/proc/<pid>/status` 采样，无单调递增趋势 |
| **体积** | 单一二进制文件体积 | $\le 20\text{MB}$ | 纯 Rust Fat LTO + `arm-linux-gnueabi-strip` |

---

## 四、 架构与排版质量标准 (Architecture & Texture Specs)

1. **架构收敛**：
   - 宿主仅保留 1 个全屏 GtkWindow + 1 个 WebKitWebView；
   - 牌组选择列表与卡片复习界面全部收敛于同一 Web SPA，彻底消灭 GTK Widget 树与滚动文字泥潭；
   - 宿主通信仅使用标准 `notify::title`（必须携带单调递增 `token`）。
2. **印刷级字模质感**：
   - 正文与公式全部锁定纯黑 `#000000 !important;`（杜绝 EPDC 驱动灰度抖动网点）；
   - 锁定 MathJax 2.7.9 TeX 矢量字模（`availableFonts: ["TeX"]`, `preferredFont: "TeX"`），严禁模糊位图回退；
   - 禁用 `matchFontHeight`，基准字号固定 `22px`，行高 `1.5`；
   - 超长公式视口自适应等比缩放，严禁产生横向滚动条撑爆页面。
3. **墨水屏人机工学**：
   - 所有触控操作栏按钮物理热区尺寸 $\ge 48\text{px} \times 48\text{px}$，间距 $\ge 12\text{px}$；
   - 连续复习每 20 张卡，强制触发一次快速全屏刷新（GC16）清除微残影。

---

## 五、 事务与异常恢复规约 (WAL & Data Integrity)

1. 每次卡片评分后，调度器立即执行事务 Commit。
2. 退出复习或接收到系统休眠广播时，强制执行 `sqlite3_wal_checkpoint`。
3. 严禁使用 `localStorage`（WebKit 1.0 `file://` 抛 DOM Exception 18）。
