# RAnki 离线排版资源体积报告 (SIZE.md)

| 相对路径 | 大小 (Bytes) | 大小 (KB) | 类型 / 说明 |
| :--- | :--- | :--- | :--- |
| `MathJax.es5.js` | 349482 | 341.29 KB |  |
| `katex.min.js` | 283981 | 277.33 KB | KaTeX ES5 转译核心引擎 |
| `mhchem.min.js` | 33712 | 32.92 KB | 化学公式 \\ce 插件 |
| `fonts/KaTeX_AMS-Regular.woff` | 33516 | 32.73 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Main-Regular.woff` | 30772 | 30.05 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Main-Bold.woff` | 29912 | 29.21 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `katex.min.css` | 21178 | 20.68 KB | KaTeX 优化样式表 (纯 WOFF 引用) |
| `fonts/KaTeX_Main-Italic.woff` | 19676 | 19.21 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Main-BoldItalic.woff` | 19412 | 18.96 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Math-Italic.woff` | 18748 | 18.31 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Math-BoldItalic.woff` | 18668 | 18.23 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Typewriter-Regular.woff` | 16028 | 15.65 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_SansSerif-Bold.woff` | 14408 | 14.07 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_SansSerif-Italic.woff` | 14112 | 13.78 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Fraktur-Bold.woff` | 13296 | 12.98 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Fraktur-Regular.woff` | 13208 | 12.90 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_SansSerif-Regular.woff` | 12316 | 12.03 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Script-Regular.woff` | 10588 | 10.34 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Caligraphic-Bold.woff` | 7716 | 7.54 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Caligraphic-Regular.woff` | 7656 | 7.48 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `shell.html` | 7418 | 7.24 KB | SPA 宿主容器模板与 window.RA 接口 |
| `fonts/KaTeX_Size1-Regular.woff` | 6496 | 6.34 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Size2-Regular.woff` | 6188 | 6.04 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Size4-Regular.woff` | 5980 | 5.84 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `fonts/KaTeX_Size3-Regular.woff` | 4420 | 4.32 KB | KaTeX 裁剪字体 (WOFF 格式) |
| `auto-render.min.js` | 3843 | 3.75 KB | KaTeX 自动扫描与定界符渲染器 |

**总体积合计**：`1002730` 字节 (`979.23` KB / `0.956` MB)

### 预算达标判定
- **硬性上限**：≤ 2.0 MB (**达标 PASS**)
- **目标预算**：≤ 900 KB (**超标 FAIL**)
- **ES5 门禁**：acorn ecmaVersion: 5 (**100% 通过**)
