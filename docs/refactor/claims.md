# Claims & Hypotheses Register (事实 / 假设登记表)

> 规则：状态只能是 `hypothesis` / `verified-device` / `verified-offline` / `refuted`。
> 标为 `hypothesis` 的条目，未在 M1/M2 用真机证据验证前，不得作为设计依据，亦不得在汇报中作为事实陈述。

---

## 1. 已有直接证据的事实 (Facts)

| 编号 | 描述 | 状态 | 证据路径 / 详情 |
| :--- | :--- | :--- | :--- |
| **F1** | 现行二进制 27,349,404 字节，2026-06-16 构建，ELF32 ARM，soft-float，符号表已剥离，`libc` 符号版本最高为 `GLIBC_2.18`。 | `verified-device` | `evidence/m0/baseline_binary/` (`sha256.txt`, `readelf_V.txt`, `file.txt`) |
| **F2** | `next_card` 只把题面拼入 HTML，答案指针留在宿主堆；`show_answer` 调用 `webkit_web_view_load_html_string` 整页重载。 | `verified-device` | 反汇编地址 `0x23028a` (`next_card`) 与 `0x22fbe0` (`show_answer`) |
| **F3** | 旧二进制翻卡耗时 `milliseconds_taken` 为 1335 / 3228 / 3109 / 1447 ms 等（含 MathJax 冷启动）。 | `verified-device` | `/mnt/us/ranki_logs/ranki_stdout.log` (PID 22850 & 1633) |
| **F4** | `localStorage.setItem` 在当前 WebKitGTK 1.0 上下文抛 `SECURITY_ERR: DOM Exception 18`。 | `verified-device` | `/mnt/us/ranki_logs/ranki_stdout.log:210` |
| **F5** | 此前“已落成的功能”从未被编译进真机二进制（真机长期运行 2026-06-16 旧版）。 | `verified-device` | 二进制 mtime `2026-06-16 19:48:52`，代码内缺少 SPA / `window.RA` 字符串 |

---

## 2. 待验证的假设 (Hypotheses)

| 编号 | 假设 | 当前状态 | 验证方法 | 计划里程碑 |
| :--- | :--- | :--- | :--- | :--- |
| **H1** | MathJax WebFont 没有真正生效，公式在用系统字形拼凑。 | `hypothesis` | 字体宽度探针；对 38 题截图检查积分号、根号、大括号是否断裂。 | M2 |
| **H2** | 字体失效的根因是跨源：页面源是 `file://`，MathJax 与字体来自 `http://127.0.0.1:port`，WebKit 1 对跨源 `@font-face` 拦截。 | `hypothesis` | 对照实验：(a) 现状；(b) 用 `load_uri("http://127.0.0.1:port/shell.html")` 让页面与字体同源，并由同一本地服务代理 `collection.media`；对比字体宽度探针。 | M2 |
| **H3** | 日志里的 `fonts/TeX/AMS/Regular/Main.js` 请求是字体失败的证据。 | `hypothesis` | **存疑**：该文件是 MathJax HTML-CSS 字符度量数据，按需加载是正常行为，不能单凭它断定降级。除非 H1/H2 实验独立证明，否则不得引用。 | M2 |
| **H4** | 连续翻 100 张卡会因 MathJax 对象残留导致 OOM。 | `hypothesis` | 用 VmRSS 采样先测旧路径基线；测到前不得定性为“必遭 OOM”。 | M1 |
| **H5** | 300PPI 下 `font-size:20px` 导致细线过细、断续。 | `hypothesis` | 先测 `window.devicePixelRatio`、实际 CSS 像素与物理像素换算，再看实拍/截图；禁止预先盲目改字号。 | M2 |
| **H6** | `title_changed` 在内容重复时不触发。 | `hypothesis` | 发两条相同标题验证；设计上不依赖该行为，一律带单调序号。 | M2 |
| **H7** | MathJax 排版期间主线程占满导致触摸无响应。 | `hypothesis` | 在排版期间触发点击并记录响应延迟；再试 `processUpdateTime` / `EqnChunk` 参数。 | M2 / M3 |
| **H8** | `file://` 页面不能访问 `collection.media` 以外资源，而 `http` 源页面不能加载 `file://` 图片。 | `hypothesis` | 实测。若 H2(b) 同源方案成立，本地服务必须同时代理 `/media/` 路径。 | M2 |
| **H9** | JSCore 无 JIT（纯解释）。 | `hypothesis` | 仅作背景参考，不影响设计；不必专门投入验证。 | — |
