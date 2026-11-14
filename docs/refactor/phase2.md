# Phase 2: 排版资产自构建与公式兼容性回归报告 (GATE 2)

**构建环境**：Node.js v22.23.4, Babel 7.29.0 (`@babel/preset-env` targets: `ie: 9`), Terser, Acorn 8.15.0  
**目标硬件**：Kindle Paperwhite 3 (WebKitGTK 1.0, JavaScriptCore 2012, ES5-only)  
**资产目录**：`build/ranki-assets/`  
**体积报告**：`docs/refactor/SIZE.md`  

---

## 一、 ES5 转译与 Acorn 门禁 (ES5 Verification)

针对 Phase 1 探针确立的铁律（`let`、`class`、箭头函数、模板字符串、`Promise` 等在 WebKitGTK 1.0 上必定触发 `SyntaxError` 导致整页脚本崩溃），我们对 KaTeX 官方发行产物进行了源码级转译与严格验证。

### 1. 官方原版 vs 自构建产物 Acorn 门禁对比
| 资产文件 | 官方发行版 Acorn 解析结果 | 自构建 Babel + Terser 产物 | 门禁判定 |
| :--- | :--- | :--- | :--- |
| `katex.js` | 失败：`The keyword 'class' is reserved (53:0)` | **100% 通过** (ecmaVersion: 5) | **PASS** |
| `auto-render.js` | 失败：`The keyword 'const' is reserved (91:0)` | **100% 通过** (ecmaVersion: 5) | **PASS** |
| `mhchem.min.js` | **100% 通过** (ecmaVersion: 5，原生 ES5) | **100% 通过** (保留原生实现) | **PASS** |
| `shell.html` (内联脚本) | - | **100% 通过** (ecmaVersion: 5) | **PASS** |

### 2. 运行时内置对象依赖扫描 (Built-ins Audit)
全产物扫描结果证明，转译产物中对环境敏感特性的引用数均为 **0**：
- `Promise` 引用数：`0`（字体加载无任何异步探测，规避 Promise ReferenceError 陷阱）
- `Map` / `Set` / `Symbol` 引用数：`0`（使用原生 Object 散列表）
- `Object.assign` 引用数：`0`
- `Array.prototype.includes` / `String.prototype.startsWith` 引用数：`0`

---

## 二、 字体全量保留与 CSS 优化 (Complete Font Family Strategy)

### 1. 字体需求评估与决策
卡片库当前公式主要集中在基础微积分/物理，但用户后续将学习并加入更高阶的高等数学（抽象代数、李代数 $\mathfrak{g}$、微分几何、泛函分析 $\mathscr{H}$、张量分析 $\mathsf{T}$、计算机复杂度 $\mathtt{O(n\log n)}$ 等）。
实测表明：全套 20 款 WOFF 字体的总大小仅为 **296.0 KB**，全套离线资产体积仅为 **635.25 KB (0.620 MB)**，不仅远低于 2.0 MB 硬限制，且依然远低于 900 KB 软目标（余量达 265 KB）。
**决策**：**全量保留全部 20 款官方 WOFF 字体族**，彻底实现 100% 完整的 LaTeX 排版表现力，避免后续进阶高数卡片遇到字形缺失。

### 2. 格式选型决策
Phase 1 探针证明：
- WOFF2：内核无解码器（失败，宽度回退为 570px）；
- WOFF：**完全支持**（渲染字符宽度 760px，比裸 TTF 节省约 45% 体积）；
- TTF：完全支持但体积冗余。

### 3. 全量字体清单 (共 20 款 WOFF，合计 296.0 KB)
- 主字体族 (Main)：`KaTeX_Main-Regular.woff` (30.1 KB), `KaTeX_Main-Bold.woff` (29.2 KB), `KaTeX_Main-Italic.woff` (19.2 KB), `KaTeX_Main-BoldItalic.woff` (19.0 KB)
- 数学字体族 (Math)：`KaTeX_Math-Italic.woff` (18.3 KB), `KaTeX_Math-BoldItalic.woff` (18.2 KB)
- 符号与重音 (AMS)：`KaTeX_AMS-Regular.woff` (32.7 KB)
- 括号与巨算符 (Size)：`KaTeX_Size1-Regular.woff` (6.3 KB), `KaTeX_Size2-Regular.woff` (6.0 KB), `KaTeX_Size3-Regular.woff` (4.3 KB), `KaTeX_Size4-Regular.woff` (5.8 KB)
- 书写体 (Caligraphic)：`KaTeX_Caligraphic-Regular.woff` (7.5 KB), `KaTeX_Caligraphic-Bold.woff` (7.5 KB)
- 哥特体 (Fraktur)：`KaTeX_Fraktur-Regular.woff` (12.9 KB), `KaTeX_Fraktur-Bold.woff` (13.0 KB)
- 无衬线体 (SansSerif)：`KaTeX_SansSerif-Regular.woff` (12.0 KB), `KaTeX_SansSerif-Bold.woff` (14.1 KB), `KaTeX_SansSerif-Italic.woff` (13.8 KB)
- 手写体 (Script)：`KaTeX_Script-Regular.woff` (10.3 KB)
- 等宽体 (Typewriter)：`KaTeX_Typewriter-Regular.woff` (15.7 KB)

### 4. `katex.min.css` 优化
将所有 20 款字族的 `@font-face` 声明中的 `src` 纯净化为 `url(fonts/*.woff) format("woff")`，彻底剔除内核不支持的 `woff2` 与冗余 `ttf` 探测，避免多余 404 网络探测与格式回退开销，样式表大小优化至 20.7 KB。

---

## 三、 卡片公式全量兼容性回归测试 (Regression Test)

测试脚本：`tools/test_regression.js`，运行模式：`throwOnError: true`（严格模式，任何未知宏或语法错误均抛出异常）。

### 1. 用户卡片库 838 个公式回归结果
- **测试总公式数**：838 条（509 个独立 LaTeX 表达式）
- **首次回归通过数**：836 条（通过率 99.76%）
- **失败项定位**：2 处使用了 MathJax 专属扩展命令 `\bbox[yellow,5px,border:2px solid red]{...}`
- **宏映射解决方案**：
  在 `shell.html` 的 KaTeX `macros` 中注入动态展开函数：
  ```javascript
  macros: {
    '\\bbox': function(context) {
      if (context.future().text === '[') {
        context.popToken();
        while (context.future().text !== ']' && context.future().text !== 'EOF') {
          context.popToken();
        }
        if (context.future().text === ']') context.popToken();
      }
      return '\\boxed';
    }
  }
  ```
- **注入宏后全量回归结果**：
  - **通过数**：**838 / 838**
  - **失败数**：**0**
  - **最终通过率**：**100.00%**！

### 2. 化学公式 `\ce` 支持验证
通过内置 `contrib/mhchem.min.js`，验证全部测试样例均渲染正常：
- `\ce{MnO4- > Cl2}`: **PASS**
- `\ce{2H2 + O2 -> 2H2O}`: **PASS**
- `\ce{Fe^{2+} + 2OH- -> Fe(OH)2 v}`: **PASS**
- `\ce{CO2 + C ->[\Delta] 2CO}`: **PASS**
- `\ce{CH4 + 2O2 -> CO2 + 2H2O}`: **PASS**

---

## 四、 离线资产体积与预算达标审计 (Budget Audit)

| 资产类型 | 资产明细 | 文件大小 (KB) | 预算占比 (基准 2.0MB) |
| :--- | :--- | :--- | :--- |
| **KaTeX ES5 核心** | `katex.min.js` | 277.33 KB | 13.5% |
| **化学扩展** | `mhchem.min.js` | 32.92 KB | 1.6% |
| **自动渲染器** | `auto-render.min.js` | 3.75 KB | 0.2% |
| **样式表** | `katex.min.css` | 20.68 KB | 1.0% |
| **宿主容器模板** | `shell.html` | 4.56 KB | 0.2% |
| **WOFF 字体集合** | 完整全量 20 款 WOFF 字体 | 296.01 KB | 14.5% |
| **合计** | **全部 25 个离线资产文件** | **635.25 KB (0.620 MB)** | **31.0%** |

### 预算达成判定
1. **硬性上限**：≤ 2.0 MB（实际 **0.620 MB**，裕量 1.38 MB）-> **达标 PASS**
2. **目标预算**：≤ 900 KB（实际 **635.25 KB**，裕量 265 KB）-> **达标 PASS**
3. **ES5 门禁**：所有 JS 产物通过 Acorn `ecmaVersion: 5` 校验 -> **100% PASS**
4. **公式兼容**：838/838 真实公式 `throwOnError: true` 零失败 -> **100% PASS**
5. **高数扩展**：全量支持 `\mathfrak`、`\mathsf`、`\mathtt`、`\mathscr` 等进阶字族 -> **100% 支持**

---

## 五、 GATE 2 结论
Phase 2 排版资产自构建全部达标完成，无任何遗留语法或格式风险。  
请用户审查并批准进入 **Phase 3：Vala 宿主 SPA 单页常驻重构 (`review.vala`)**。
