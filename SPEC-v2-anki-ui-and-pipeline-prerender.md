# Spec Delta: Anki-Authentic E-ink UI & Next-Card Pipeline Pre-Rendering

> **Document Status**: Proposed  
> **Source Baseline**: SPEC.md (v1.0.0), HANDOFF.md v5.0.0-Reviewed  
> **Target Device**: Amazon Kindle Paperwhite 3 (Model DP75SDI, 1072x1448 @ 300 PPI, 90.5mm x 122.0mm active area, WebKitGTK 1.0, Linux 3.0.35)  
> **Version**: v2.2.0-Draft  
> **Date**: 2026-11-16  

---

## 1. Objective

Address real-device user feedback from physical Kindle Paperwhite 3 testing:
1. **先说对的 (Validated Improvements)**:
   - 翻开卡片后双缓冲原地切换机制在真机上运行完美，公式答案秒开（$\le 5\text{ms}$），化学公式渲染正常。
2. **A. Anki-Authentic Deck Browser & Reviewer Redesign (牌组大厅与卡片全场景对齐 Anki 官方美学)**:
   - **牌组大厅极度还原官方 Anki**：
     - 界面完全居中排布（宽度 940px，居中自适应，告别全屏粗劣黑框）；
     - 顶部简洁状态栏与标题；
     - 经典四列式表格结构：`牌组名称 (Deck)` ── `新卡 (New)` ── `学习 (Learn)` ── `复习 (Due)`；
     - **全自动展开子牌组 (Auto-Unfold Subdecks)**：树形递归提取所有深度子牌组，通过层级缩进（`depth * 36px`）与树形前缀优雅呈现，可精准点击进入任意子牌组复习；
     - 各列到期数字右对齐对齐 Anki 经典账簿感，触控具备墨水屏反色按下态反馈；
     - 底部居中展示学习总结（如 `今日已复习 N 张卡片`）与极简退出按键。
   - **卡片复习器经典还原**：
     - 顶部 64px 极简顶栏（左侧 `<` 返回，中央牌组名，右侧三色复习计数徽章）；
     - 正文区 44px（五号字 3.73mm）黄金纸书排版，背面保留 `<hr id="answer">` 标准题面与答案分隔线；
     - 底部 110px 方案 A 双行胶囊评分栏（上行 24px 预估时间，下行 32px 评级文字，8px 圆角，1.5px 细线）。
3. **B. Next-Card Pipeline Pre-Rendering (下一张卡片流水线预渲染)**:
   - 四槽位离屏乒乓缓冲池，在用户研读卡片 $N$ 时，卡片 $N+1$ 后台 100% 排版完成。
   - 点击评分瞬间做 DOM 指针毫秒级切换（$\le 5\text{ms}$ 切卡），消灭 500ms 等待。
4. **C. Typography & Pure Font Assets (纯净字模体系)**:
   - **西文/公式/数字**：严格唯一锁定 `Anthropic Sans Web Text Regular` 与 `Italic`（无损转码为 WOFF 1.0 与 TTF 嵌入前端）；
   - **中文字体**：**彻底排除官方精修宋体、仓耳今楷与方正字体**，中文字体**严格唯一指定为索尼明朝体 (`SonyReader Ming.ttf`)**。
5. **D. Log Analysis & Boundary Hardening**:
   - 宿主静默吸收 `CARD_FLIPPED`，牌组完成切出结算卡片。

---

## 2. 牌组大厅视觉布局规范 (Anki-Authentic Deck Browser Spec)

### 2.1 线框与排版 ($1072 \times 1448$, 居中容器宽度 940px)

```
+-------------------------------------------------------------+  ^
|                        RAnki 牌组大厅                       |  | 70px 顶栏
+-------------------------------------------------------------+  v
|                                                             |
|   ┌─────────────────────────────────────────────────────┐   |
|   │ 牌组 (Deck)              新卡      学习      复习   │   |  | 表头 (Header)
|   ├─────────────────────────────────────────────────────┤   |  | 1px 细线
|   │ ▾ 物理核心题库             5        12        34    │   |  | 根牌组
|   │   ├─ 选修 3-1: 电磁学      3         8        20    │   |  | 一级子牌组
|   │   │  └─ 恒定电流与电功率   2         4        14    │   |  | 二级子牌组 (自动展开)
|   │   └─ 必修 1: 牛顿运动定律   2         4        14    │   |
|   │ ▾ 化学核心反应方程式       10        15        40    │   |
|   │   └─ 元素周期律与方程式    10        15        40    │   |
|   │ ▾ 高等数学               0         0         0    │   |
|   └─────────────────────────────────────────────────────┘   |
|                                                             |
|              今日已复习: 56 张卡片                          |  | 居中信息
|                                                             |
|                     [  退  出  RAnki  ]                     |  | 60px 胶囊按键
+-------------------------------------------------------------+  v
```

### 2.2 视觉与交互特性
1. **水平居中**：整个表格容器 `width: 940px; margin: 40px auto;`，两边留有舒适呼吸边距，沉浸大方。
2. **表头定义**：
   - `牌组 (Deck)`：左对齐，字号 26px，加粗；
   - `新卡 (New)`：右对齐（宽 100px），字号 24px；
   - `学习 (Learn)`：右对齐（宽 100px），字号 24px；
   - `复习 (Due)`：右对齐（宽 100px），字号 24px。
3. **行数据定义**：
   - 行高 $\ge 76\text{px}$，触控舒适；
   - 子牌组缩进：`padding-left: depth * 32px`，搭配树形连接符 `├─` 或 `└─`；
   - 墨水屏反色：点击行时整行黑白反色（背景黑、文字白），提供物理按键般的敲击感。

---

## 3. 字体资产与排版规范

### 3.1 纯净字体声明
```css
@font-face {
    font-family: "Anthropic Sans Web";
    src: url("fonts/anthropic/AnthropicSans-Regular.woff") format("woff"),
         url("fonts/anthropic/AnthropicSans-Regular.ttf") format("truetype");
    font-weight: normal;
    font-style: normal;
}

@font-face {
    font-family: "Anthropic Sans Web";
    src: url("fonts/anthropic/AnthropicSans-Italic.woff") format("woff"),
         url("fonts/anthropic/AnthropicSans-Italic.ttf") format("truetype");
    font-weight: normal;
    font-style: italic;
}

@font-face {
    font-family: "SonyReader Ming";
    src: local("SonyReader Ming"),
         url("file:///mnt/us/fonts/SonyReader%20Ming.ttf") format("truetype");
    font-weight: normal;
    font-style: normal;
}

body {
    background-color: #ffffff;
    color: #000000 !important;
    font-family: "Anthropic Sans Web", "SonyReader Ming", serif;
    font-size: 44px;
    line-height: 1.6;
}
```

---

## 4. 递归子牌组与卡片流水线架构

### 4.1 数据结构
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeckItem {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub depth: usize,
    pub is_leaf: bool,
    pub new_count: u32,
    pub learn_count: u32,
    pub review_count: u32,
    pub due_count: u32,
}
```

### 4.2 递归提取
- 递归扁平化 `col.deck_tree(None)`，将树状结构完整输出为前序遍历列表，所有子牌组自动展开，前端依 `depth` 呈现缩进，100% 还原 Anki 官方体验。
