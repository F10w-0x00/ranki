# Phase 3: Vala 宿主单页常驻重构与真机部署验证报告 (GATE 3)

**目标分支**：`refactor/katex-single-page`  
**核心代码变更**：`views/review.vala` (Commit: `31690aa`)  
**工具链与测试**：`tools/test_js_quote.js`, `tools/build_dropin_shim.js`, `tools/deploy_to_kindle.py` (Commit: `82ac6be`)  

---

## 一、 宿主 SPA 架构改造核心（`views/review.vala`）

彻底铲除原有在 `next_card()` 与 `show_answer()` 中每次全量销毁重建 DOM 的 `card_content.load_html_string(...)` 调用，改造为工业级单页应用（SPA）增量驱动模型。

### 1. 启动期单次加载（Single Initial Load）
在 `ready()` 中只执行一次骨架装载：
```vala
// 检测 assets 中的 shell.html 模板，启动期仅加载一次
if (shell_template_content != "") {
    debug("Initializing WebKit single-page shell once with base_uri %s", base_uri);
    card_content.load_html_string(shell_template_content, base_uri);
}
```
`base_uri` 固定为 `file:///mnt/us/anki_data/collection.media/`，保证卡片中引用的所有本地图片与媒体能够自然解析，不产生断链。

### 2. 状态机防竞态队列（Pending Show Queue）
为规避引擎首次 boot 未完成前用户快速点击翻卡的竞态：
- 监听 `card_content.notify["load-status"]` 与 `card_content.title_changed`（`RA:READY` 信号）；
- 状态就绪前发起的翻卡请求自动压入 `pending_show`（只保留最新一条）；
- 状态就绪时自动调用 `flush_pending_card()` 刷入显示。

### 3. `js_quote` 字符转义实现与单元测试
LaTeX 源码中包含极其密集且复杂的反斜杠、双引号与特殊换行，任何字符转义缺陷都会导致 JS 解析报 `SyntaxError` 并静默失败。  
我们在 `views/review.vala` 中按高工规范采用 `unichar` 逐字迭代算法实现：
- `\` -> `\\`
- `"` -> `\"`
- `\n`, `\r`, `\t`, `\b`, `\f` -> 标准转义
- `<` -> `\u003c`（根除 `<script>` 闭合逃逸风险）
- `U+2028` / `U+2029` -> 显式 Unicode 转义（防止 JSCore 语法解析异常）
- 控制字符 `< 0x20` -> `\u00xx`
- 中文、Emoji、数学符号 -> 原生 Unicode 保留

配套自动化单元测试套件：`tools/test_js_quote.js`（覆盖 `\frac`、`\\`、中文、Emoji、U+2028、控制字符与矩阵结构），**8/8 测试用例 100% 全部通过**。

### 4. 翻卡统一驱动函数 `show_card`
```vala
private void show_card(string css, string html) {
    current_token++;
    uint64 token = current_token;
    t_flip_requested = GLib.get_monotonic_time();

    if (!is_shell_ready) {
        pending_show = PendingCardShow() { css = css, html = html, token = token };
        return;
    }

    string script = "window.RA.show(" + js_quote(css) + ", " + js_quote(html) + ", " + token.to_string() + ");";
    card_content.execute_script(script);
}
```

### 5. 毫秒级性能打点流水线（Latency Telemetry）
在宿主侧建立了从用户点击到墨水屏重绘完成的完整闭环埋点：
- 用户点击时刻：记录 `t_flip_requested = GLib.get_monotonic_time()`；
- 收到 `document.title` 的 `"RA:token:elapsed"` 回传：计算宿主到渲染完成时延 `T_render`；
- 挂载一次性 `expose-event` 回调：计算 GTK 重排与绘制时延 `T_paint`；
- 自动写入 `/mnt/us/ranki_logs/latency.csv`（格式：`timestamp,token,t_render_ms,t_paint_ms,total_ms`）。

---

## 二、 真机即时验证部署

为了让真机能够**立刻在当前设备上享受 Phase 2 的 KaTeX 编译成果**，我们同步构建了严格通过 Acorn ES5 门禁的 drop-in 引擎：
- 产物文件：`build/ranki-assets/MathJax.es5.js`（346.9 KB）
- 已部署路径：`D:\anki_data\.assets\MathJax-2.7.9\MathJax.js`（原始 MathJax 备份保存在 `MathJax.js.bak`）
- 字体文件：全量 20 款 WOFF 字体已在 `D:\anki_data\.assets\MathJax-2.7.9\katex-fonts\` 就绪。

---

## 三、 GATE 3 上机验证指南

用户现在可以安全安全弹出 Kindle 进行真机实测：
1. **安全弹出**：在电脑上弹出 `D:\` 盘符，拔下 USB 数据线；
2. **启动 RAnki**：在 Kindle 上通过 KUAL 启动 RAnki；
3. **日常复习翻卡**：
   - 翻看 5~10 张含公式的卡片（微积分导数、积分、分式、化学方程式 `\ce`）；
   - 观察白屏时间是否彻底缩短；
   - 观察公式是否正常渲染，是否存在任何乱码或 raw LaTeX 代码；
4. **日志回传**：复习若干张卡后，插回 USB，我们将自动读取 `/mnt/us/ranki_logs/ranki_stdout.log` 与 `latency.csv` 统计 $T_{render} + T_{paint}$ 的 P50/P95 指标！
