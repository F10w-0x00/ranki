# Implementation Plan: RAnki Route B — Native GTK2/Cairo Shell + Ultra-Fast Resident Card Engine

Source spec: [SPEC-route-b-native-shell.md](../SPEC-route-b-native-shell.md)
Intent anchor: [docs/intent/kpw3-anki-daily-driver.md](../docs/intent/kpw3-anki-daily-driver.md)

## Overview

Rebuild RAnki for Kindle Paperwhite 3 (DP75SDI) using **Route B**: a pure-Rust direct `anki/rslib` host with a **native GTK2 + PangoCairo minimalist paper-book UI** (paginated Deck Browser, Review Top/Bottom Bars, isolated 52×52px 1 Hz Cairo 20s iPhone Duo-style circular focus timer, GDK-level zero-bar swipe/tap paging, native `GtkDialog`/`GtkEntry` Settings & Login with Kindle LIPC keyboard, non-blocking background Wi-Fi sync), paired with a **card-only resident WebKitGTK 1.0 canvas** running official `<div class="card kindle">` centered typography (`Anthropic Sans` + `SonyReader Ming`) and **ES5 KaTeX (`mhchem` + `\bbox` + `vlist-t2` fix)** with switchable MathJax 2.7.9 fallback and Card-ID-verified pre-rendering.

## Architecture Decisions

- **Direct `anki/rslib 25.09.02` in Rust**: Preserves 100% authentic SM-2 / FSRS scheduling, Cloze rendering, Undo/Bury/Suspend, and AnkiWeb sync with zero Vala/Protobuf bridge.
- **Native GTK2 + PangoCairo Chrome + Fontconfig App Fonts**: Eliminates WebKitGTK 1.0 chrome deadlocks, inertial scroll mud, and flaky HTML `<input>` keyboard focus; guarantees 52×52px isolated dirty-rect partial redraw for the 20s Duo circular timer and uniform `Anthropic Sans` + `SonyReader Ming` typography across the entire app.
- **Card-Only Resident WebView + ES5 KaTeX Primary**: Retains 100% HTML/CSS/`<ruby>`/`\ce`/`\bbox`/`<cite>`/`<font color="red">` fidelity and official `.card` centering while compressing cold formula latency from 500–1500 ms down to ~20–50 ms (and < 5 ms on flip/prefetch hit), with MathJax 2.7.9 kept as a switchable safety net.
- **GDK-Level Zero-Bar Swipe/Tap Paging**: Intercepts pointer press/motion/release on the read-only card `WebKitWebView` in Rust so finger dragging paints 0 intermediate frames (zero ghosting, zero text selection) and lifts straight into a crisp 85% viewport jump with no paging bar clutter.
- **Card-ID-Verified Pipeline & Async Sync**: Eliminates silent wrong-card rating bugs on `Again` / buried Cloze siblings / `Undo` / `Bury` / `Suspend`, and runs sync off the GTK main loop with explicit `FullSync` confirmation and zero media downloads.

## Dependency Graph

```text
[Phase 0: Task 1 — Security/Tooling Hygiene & Branch]
         │
         ├───────────────────────────────────────────┐
         ▼                                           ▼
[Phase 1: card-renderer (Local Host Testable)]  [Phase 2: anki-core (Unit Testable)]
  Task 2: ES5 KaTeX Assets + 838 Regression      Task 5: Safe config.rs + Schema
         │                                               │
  Task 3: card.html/css/js (Centered + Slots)    Task 6: Card-ID Pipeline + Undo/Bury/Suspend
         │                                               │
  Task 4: 1072x1448 Visual Preview Harness       Task 7: Background Async Sync (Normal + FullSync)
         │                                               │
  ★ Checkpoint 1: Visual & Formula Sign-Off      ★ Checkpoint 2: Core CI Tests Pass
         └───────────────────┬───────────────────────────┘
                             ▼
[Phase 3: native-shell (ARMEL Cross-Compile)]
  Task 8: Thin GTK2/Cairo/Pango/Fontconfig/WebKit FFI + LIPC/GC16
         │
  Task 9: PangoCairo Paginated Deck Browser (with Swipe)
         │
  Task 10: Isolated 52x52px Cairo 20s Duo Circular Focus Timer
         │
  Task 11: Review View (Top/Bottom Bars, GDK Swipe Paging, Card Bridge)
         │
  Task 12: Native Settings/Login/Sync Dialogs & App Wiring (main.rs)
         │
  ★ Checkpoint 3: ARMEL CI Build & 4 Mechanical Gates Pass
                             │
                             ▼
[Phase 4: KPW3 Hardware Validation]
  Task 13: Safe Dev Deploy + 3 Hardware Spikes + Stress Acceptance
         │
  ★ Checkpoint 4: Real-Device Daily-Driver Sign-Off
```

---

## Detailed Task Breakdown

### Phase 0: Governance, Security & Tooling Hygiene

#### Task 1: Branch Setup, Secret Removal & Upgraded Mechanical Gates
**Description:** Create a clean `feat/route-b-native-shell` branch from `refactor/m0-baseline`, remove the hardcoded `hkey` from `tools/deploy_to_kindle.py` (ensuring existing device `config.ini` tokens are preserved), fix `tools/download_artifact.py` to match the CI artifact name `ranki-armel`, and harden `tools/check_es5.js` to catch both ES6+ syntax and post-2012 WebKit 1.0 missing DOM APIs (`.closest(`, `Promise`, `fetch`, `localStorage`, `requestAnimationFrame`) plus Unicode Emoji literals.
**Acceptance criteria:**
- [ ] Branch `feat/route-b-native-shell` exists cleanly without modifying `refactor/m0-baseline`.
- [ ] `tools/deploy_to_kindle.py` contains zero hardcoded `hkey` and preserves existing `config.ini` on device (only writing a default template when missing).
- [ ] `tools/check_es5.js` rejects ES6+ syntax, `.closest(`, `localStorage`, `Promise`, `fetch`, `requestAnimationFrame`, and Unicode Emoji literals.
**Verification:**
- [ ] `python -m py_compile tools/deploy_to_kindle.py tools/download_artifact.py` exits 0.
- [ ] `node tools/check_es5.js` tested against a synthetic bad snippet (fails) and valid ES5 snippet (passes).
**Dependencies:** None
**Files likely touched:**
- `tools/deploy_to_kindle.py`
- `tools/download_artifact.py`
- `tools/check_es5.js`
**Estimated scope:** Small (3 files)

---

### Phase 1: Ultra-Fast Centered Card Renderer (`card-renderer`)

#### Task 2: Restore ES5 KaTeX Assets & 838-Formula Regression Harness
**Description:** Extract the verified ES5-transpiled KaTeX bundle (`katex.min.js` with `vlist-t2` WebKit 1.0 baseline fix, `mhchem.min.js`, `auto-render.min.js`, `katex.min.css`, and all 20 WOFF fonts totaling ~635 KB) from `refactor/katex-single-page` into `views/katex/`, and restore `tools/test_regression.js` + `tools/fixtures/collection_formulas.json` so all 838 real user formulas (including `\ce{}` and `\bbox` -> `\boxed`) are gated continuously.
**Acceptance criteria:**
- [ ] `views/katex/` contains `katex.min.js` (with `vlist-t2` patch), `mhchem.min.js`, `auto-render.min.js`, `katex.min.css`, and 20 WOFF fonts (total ≤ 900 KB).
- [ ] `node tools/test_regression.js` passes **838 / 838 (100%)** with `throwOnError: true`.
- [ ] All JS files in `views/katex/` pass `node tools/check_es5.js`.
**Verification:**
- [ ] `node tools/test_regression.js` exits 0 with 838/838 pass.
- [ ] `node tools/check_es5.js views/katex/katex.min.js` exits 0.
**Dependencies:** Task 1
**Files likely touched:**
- `views/katex/*`
- `tools/test_regression.js`
- `tools/fixtures/collection_formulas.json`
**Estimated scope:** Small (asset import + 2 test files)

#### Task 3: Build Resident Card Canvas (`views/card.html`, `card.css`, `card.js`)
**Description:** Create the dedicated resident single-page card canvas loaded once by `WebKitWebView`. Restore `<div class="card kindle">` official Anki centering, calibrated `Anthropic Sans` + `SonyReader Ming` paper-book styling (`28px` default, `setFontSize(20–38)`), crisp monochrome styling for `<font color="red">`, `<ruby>` (including math inside ruby), `<cite>`, `<hr>`, `.cloze`, `\bbox`, long-equation `-webkit-transform` auto-scaling, Card-ID-verified 4-slot pre-render engine, switchable MathJax 2.7.9 fallback, and discrete `±85%` viewport paging (`pageScroll`) reporting `PAGE_STATE` via monotonic `document.title` token.
**Acceptance criteria:**
- [ ] Card slots wrap HTML inside `<div class="card kindle">` with `text-align: center` default and zero `-webkit-overflow-scrolling: touch`.
- [ ] `window.CardEngine` supports `setConfig`, `renderImmediate`, `flipToAnswer`, `swapToPrefetched`, and `pageScroll` with strict `card_id` matching and timing telemetry.
- [ ] `views/card.js` passes `node tools/check_es5.js views/card.js` with 0 errors and 0 banned APIs/emojis.
**Verification:**
- [ ] `node tools/check_es5.js views/card.js` exits 0.
- [ ] Automated Node/DOM unit check validates `CardEngine` state transitions and `\bbox` / `\ce` preprocessing.
**Dependencies:** Task 2
**Files likely touched:**
- `views/card.html`
- `views/card.css`
- `views/card.js`
**Estimated scope:** Medium (3 files)

#### Task 4: Local 1072×1448 Real-Card Visual Preview Harness
**Description:** Build a standalone local preview page (`tools/preview_cards.html`) simulating the exact KPW3 `1072×1448` frame (including top bar + 52×52px 20s circular Duo timer mockup + centered `views/card.html` iframe + bottom 4-rating bar) populated with the user's real cards from `高等数学_常用微分规则_anki导入.txt` and `anki-card-maker` examples (`\ce`, `\bbox`, `<ruby>\(\nu\)<rt>nu</rt></ruby>`, `<font color="red">`, `<cite>`, multi-screen long proof).
**Acceptance criteria:**
- [ ] Opens cleanly in a desktop browser at 1072×1448 scale with Front/Flip/Next/Font-Size/Long-Card-Page controls.
- [ ] Renders all STEM card features centered cleanly with paper-book proportions.
**Verification:**
- [ ] Open in browser / capture screenshot for human visual sign-off on layout & typography.
**Dependencies:** Task 3
**Files likely touched:**
- `tools/preview_cards.html`
**Estimated scope:** Small (1 file)

### ★ Checkpoint 1: Card Renderer & Visual Aesthetics Sign-Off
- [ ] `node tools/test_regression.js` passes 838/838.
- [ ] `node tools/check_es5.js views/card.js` passes with zero ES6+ / banned WebKit 1.0 APIs.
- [ ] Human visually reviews `tools/preview_cards.html` at 1072×1448 and approves centered typography, default font size, red-emphasis monochrome style, and 20s Duo circle look.

---

### Phase 2: Rock-Solid `anki-core` & Background Async Sync

#### Task 5: Upgrade Safe Configuration Manager (`src/config.rs`)
**Description:** Extend `AppConfig` to manage minimal Route B settings (`collection_dir`, `hkey`, `sync_server`, `font_size_px`, `math_engine`) with clamped safe bounds, atomic write-and-rename saving, zero hardcoded secrets, and unit tests. (The 20s Duo circular focus timer and 20-card auto GC16 refresh use built-in constants out-of-the-box to stay radically minimalist.)
**Acceptance criteria:**
- [ ] Loads legacy `config.ini` cleanly with sensible defaults (`font_size_px=42`, `math_engine=katex`).
- [ ] Clamps invalid values (`font_size_px` 28..=54) and preserves unknown keys cleanly.
- [ ] Unit tests cover parse, clamp, and round-trip save in a temp directory.
**Verification:**
- [ ] `cargo test --lib config` passes (in CI / Rust toolchain).
**Dependencies:** Task 1
**Files likely touched:**
- `src/config.rs`
**Estimated scope:** Small (1 file)

#### Task 6: Card-ID-Verified Review Pipeline, Deduplicated Deck Counts & Undo/Bury/Suspend (`src/anki/mod.rs`)
**Description:** Refactor `AnkiService` to implement `AdvanceResult` (`PrefetchHit`, `RenderImmediate`, `DeckFinished`), strict `expected_cid` verification on `rate_card()`, accurate root-only `total_due` computation, `QueueCounts` with `ActiveQueue` (New/Learn/Review underline), native `undo()` (`col.undo()` + `col.undo_status()`), `bury_current()` / `suspend_current()` (`col.bury_or_suspend_cards()`), and WAL checkpointing. Remove unnecessary image media URL rewriting since images are out of scope. Add Rust unit tests against an isolated temporary SQLite collection verifying `Again`, sibling-buried Cloze, `Undo`, `Bury`, and `Suspend` never desync `current_card_id`.
**Acceptance criteria:**
- [ ] `get_deck_tree()` returns recursive flattened tree plus accurate deduplicated root `total_due`.
- [ ] `rate_card(expected_cid, rating, prefetched_cid)` returns `PrefetchHit` only when `next_queued[0].id == prefetched_cid`, and `RenderImmediate` otherwise; rejects mismatched `expected_cid`.
- [ ] `undo()`, `bury_current()`, and `suspend_current()` update `Collection` atomically and return exact refreshed state.
**Verification:**
- [ ] `cargo test --lib anki` passes on isolated temporary Anki collection.
**Dependencies:** Task 5
**Files likely touched:**
- `src/anki/mod.rs`
- `src/lib.rs` (expose testable core modules)
**Estimated scope:** Medium (2 files)

#### Task 7: Non-Blocking Background Sync Engine (`src/anki/sync.rs`)
**Description:** Implement thread-safe background sync worker (`std::thread::spawn` with isolated `tokio` current-thread runtime) supporting `sync_login` (returns `hkey`, clears password memory) and DB-only `normal_sync` (never invokes `sync_media`), explicitly detecting `SyncStatus::FullSync` and supporting user-confirmed `full_download` / `full_upload` with safe `Collection` take-and-reopen semantics. Wire CI `.github/workflows/build-armel.yml` to run host `cargo test --lib` before ARMEL cross-compilation.
**Acceptance criteria:**
- [ ] Sync and login never execute on the calling UI thread; progress and results are delivered via `mpsc` channel.
- [ ] `FullSync` is surfaced as `SyncOutcome::FullSyncRequired` rather than falsely reporting success.
- [ ] Zero calls to `sync_media` exist anywhere in the codebase.
**Verification:**
- [ ] `cargo test --lib` passes in CI (`build-armel.yml` host test step).
**Dependencies:** Task 6
**Files likely touched:**
- `src/anki/sync.rs`
- `src/anki/mod.rs`
- `.github/workflows/build-armel.yml`
**Estimated scope:** Medium (3 files)

### ★ Checkpoint 2: Core Backend Verified
- [ ] `anki-core` unit tests (config, Card-ID pipeline, Undo/Bury/Suspend, sync state machine) pass 100% in CI.
- [ ] Zero production DB paths referenced in any test.

---

### Phase 3: Native GTK2/PangoCairo Shell (`native-shell`)

#### Task 8: Minimal GTK2/Cairo/Pango/Fontconfig/WebKit FFI & E-ink Hardware Helpers
**Description:** Modularize `src/ffi/` into clean thin C-FFI bindings (`gtk.rs`, `cairo.rs`, `webkit.rs`), update `build.rs` to link `gdk-x11-2.0`, `cairo`, `pango-1.0`, `pangocairo-1.0`, `fontconfig` (all Kindle OS whitelisted), implement `src/ui/theme.rs` (300 PPI monochrome layout constants & Fontconfig app-font registration for `Anthropic Sans` + `SonyReader Ming`), and implement `src/ui/eink.rs` (debounced Kindle LIPC keyboard open/close + hardware GC16 full refresh via `mxcfb` ioctl / `eips -f ""` fallback).
**Acceptance criteria:**
- [ ] All linked libraries are strictly within `tools/check_dt_needed.py` Kindle OS whitelist.
- [ ] `FcConfigAppFontAddFile` registers `AnthropicSans-Regular.ttf`, `AnthropicSans-Italic.ttf`, and `/mnt/us/fonts/SonyReader Ming.ttf` gracefully (non-fatal if absent on host).
- [ ] `eink.rs` provides debounced `open_keyboard`/`close_keyboard` and `trigger_full_refresh()`.
**Verification:**
- [ ] ARMEL cross-compile compiles cleanly with 0 warnings; `check_dt_needed.py` & `check_symbols.py` PASS.
**Dependencies:** Task 7
**Files likely touched:**
- `build.rs`
- `src/ffi/mod.rs`
- `src/ffi/gtk.rs`
- `src/ffi/cairo.rs`
- `src/ffi/webkit.rs`
- `src/ui/theme.rs`
- `src/ui/eink.rs`
**Estimated scope:** Medium-Large (split cleanly into FFI + `eink`/`theme` helpers; keep commit cohesive and thin)

#### Task 9: PangoCairo Minimalist Paginated Deck Browser (`src/ui/deck_view.rs`)
**Description:** Build Page 0 of the `GtkNotebook` entirely with PangoCairo `GtkDrawingArea`s: 76px header (`RAnki`, deduplicated `待复习 N 张`, crisp vector `同步`/`设置`/`刷新`/`退出` buttons), 960px centered deck list canvas (10 generous rows/page, `#D8D8D8` hairline dividers, all subdecks auto-expanded by default, `depth * 32px` indent, `▾`/`▸` optional collapse hit zone, right-aligned `新/学/复` tabular counts, instant monochrome invert tap feedback), 92px footer (`< 上一页  第 X / Y 页  下一页 >` + `studied_today`), and swipe gesture page turning.
**Acceptance criteria:**
- [ ] Renders full recursive deck tree with working subdeck collapse/expand and accurate root-only `total_due`.
- [ ] Supports both footer page buttons and swipe gestures (Up/Down or Left/Right) with zero scrollbars and zero tofu emoji.
- [ ] Tapping an empty deck shows clean inline/dialog notice without freezing or corrupting UI state.
**Verification:**
- [ ] Unit tests for row hit-testing, collapse filtering, and pagination math pass; ARMEL cross-compile succeeds cleanly.
**Dependencies:** Task 8
**Files likely touched:**
- `src/ui/deck_view.rs`
**Estimated scope:** Small (1 file)

#### Task 10: Isolated 52×52px Cairo 20s iPhone Duo Circular Focus Timer (`src/ui/focus_timer.rs`)
**Description:** Implement the standalone `52×52px` Cairo `GtkDrawingArea` focus countdown widget: starts on card Front (`remaining = focus_timer_secs`, default 20), ticks via `g_timeout_add(1000)` calling `gtk_widget_queue_draw_area(widget, 0, 0, 52, 52)` ONLY, draws a crisp 1-bit monochrome clockwise-shrinking filled circular sector (18° per second), shows a clean empty hollow monochrome ring at 0s (no center dot — gentle visual cue, never auto-flips/auto-rates), and stops/hides immediately on Flip or when disabled.
**Acceptance criteria:**
- [ ] Timer state transitions (`Hidden` → `Running(20..1)` → `Expired` → `Hidden`) covered by pure-Rust unit tests.
- [ ] Redraws strictly within its isolated 52×52px widget bounds; removes `g_timeout` source cleanly on Flip/Exit (0% idle CPU).
**Verification:**
- [ ] `cargo test --lib focus_timer` passes; ARMEL build succeeds cleanly.
**Dependencies:** Task 8
**Files likely touched:**
- `src/ui/focus_timer.rs`
**Estimated scope:** Small (1 file)

#### Task 11: Reviewer Chrome, GDK Zero-Bar Swipe/Tap Paging & Card WebView Bridge (`src/ui/review_view.rs`)
**Description:** Build Page 1 of the `GtkNotebook`: 76px `GtkHBox` Review Top Bar (`< 牌组`, truncated deck name, official Anki `新 + 学 + 复` underlined active queue, `撤销`, compact `···` menu for Bury/Suspend/Refresh/Font-Size, subtle `1/2` long-card indicator + embedded `focus_timer`), resident `WebKitWebView` loading `views/card.html` once with **GDK-level `button-press`/`motion-notify`/`button-release` gesture interception** (zero drag frames, swipe Up/Down or tap bottom/top 35% jumps 85% viewport cleanly with zero paging bar), and 122px PangoCairo Bottom Action Bar (`显示答案` / 4-tier `interval + rating` buttons).
**Acceptance criteria:**
- [ ] WebView loads `views/card.html` exactly once at startup and communicates via monotonic-token `notify::title` + `execute_script`.
- [ ] GDK gesture handler consumes drag motion (0 intermediate repaints) and triggers `CardEngine.pageScroll` cleanly on release/tap.
- [ ] Undo, Bury, Suspend, Rating, and Flip all update top queue counts, intervals, focus timer, and auto-GC16 counter accurately.
**Verification:**
- [ ] Unit tests for GDK swipe/tap gesture classification and button hitboxes pass; ARMEL build succeeds cleanly.
**Dependencies:** Task 3, Task 6, Task 8, Task 10
**Files likely touched:**
- `src/ui/review_view.rs`
**Estimated scope:** Medium (1–2 files)

#### Task 12: Native Settings, Login, Sync Dialogs & Full App Wiring (`src/ui/settings_dialog.rs`, `src/ui/mod.rs`, `src/main.rs`)
**Description:** Build native `GtkDialog` screens for minimalist Settings & AnkiWeb Login (`GtkEntry` email, masked password, custom sync server with debounced Kindle LIPC keyboard; Font Size `28–54px` default `42px`; Math Engine `KaTeX/MathJax` — keeping 20s focus timer & 20-card GC16 refresh built-in with zero setting clutter), non-blocking Sync Progress & `FullSync` choice dialog polled via `g_idle_add`, remove obsolete `src/ipc.rs` / old `views/index.html` SPA coupling, and wire everything in `src/ui/mod.rs` and `src/main.rs` with clean WAL checkpoint on destroy/exit.
**Acceptance criteria:**
- [ ] Clicking Sync/Login opens non-blocking progress dialog, updates UI on completion via `g_idle_add`, and prompts explicitly on `FullSyncRequired`.
- [ ] Changing font size, timer settings, or math engine persists to `config.ini` and applies immediately without restart.
- [ ] Zero references to legacy `views/index.html` or blocking `rt.block_on` on GTK main loop remain.
**Verification:**
- [ ] Push to `feat/route-b-native-shell` → GitHub Actions CI passes host unit tests + ARMEL cross-compile + all 4 mechanical gates.
**Dependencies:** Task 7, Task 9, Task 11
**Files likely touched:**
- `src/ui/settings_dialog.rs`
- `src/ui/mod.rs`
- `src/main.rs`
- `src/ipc.rs` (remove obsolete full-SPA IPC)
**Estimated scope:** Medium (4 files)

### ★ Checkpoint 3: ARMEL CI Binary & 4 Mechanical Gates
- [ ] GitHub Actions CI passes 100% (`cargo test --lib` + ARMEL cross-compile).
- [ ] `python tools/check_symbols.py build/ranki-armel` → `[PASS]` (`GLIBC <= 2.18`).
- [ ] `python tools/check_dt_needed.py build/ranki-armel` → `[PASS]` (100% Kindle OS whitelist).
- [ ] `python tools/check_dynsym.py build/ranki-armel` → `[PASS]` (0 leaked static symbols).
- [ ] `node tools/check_es5.js views/card.js` → `[PASS]`.
- [ ] Binary size ≤ 20 MB.

---

### Phase 4: KPW3 Real-Device Validation (3 Hardware Spikes + Stress Acceptance)

#### Task 13: Safe Dev Deployment & Real-Device Hardware Verification
**Description:** Download `build/ranki-armel`, deploy safely to `D:\extensions\ranki-dev\` targeting `D:\anki_data_test\` (asserting 0-byte modification to `D:\anki_data\`), and guide/verify the 3 hardware spikes plus daily-driver smoke acceptance on physical KPW3:
1. **Spike 1**: ES5 KaTeX visual crispness & timing logs (`cold <= 80ms`, `flip <= 5ms`, `hit next <= 5ms`) on real STEM cards;
2. **Spike 2**: 52×52px Cairo 1 Hz Duo timer partial redraw (zero full-screen flicker/ghosting) + true GC16 full refresh + GDK swipe/tap zero-bar paging feel;
3. **Spike 3**: Native `GtkEntry` Kindle keyboard + non-blocking Wi-Fi AnkiWeb sync + Undo/Bury/Suspend + 50–100 card VmRSS stability.
**Acceptance criteria:**
- [ ] `D:\anki_data\` SHA256 fingerprint unchanged (0 bytes modified).
- [ ] All 3 hardware spikes verified on physical KPW3 via `/mnt/us/ranki_logs/ranki_dev_stdout.log` and user inspection.
- [ ] User confirms daily-driver readiness before any switch to production `/mnt/us/anki_data`.
**Verification:**
- [ ] `python tools/deploy_to_kindle.py` passes prod-data fingerprint assertion.
- [ ] Device stdout log telemetry & user physical confirmation.
**Dependencies:** Task 12
**Files likely touched:**
- `docs/refactor/JOURNAL.md` (record verified-device evidence)
**Estimated scope:** Small (verification & evidence logging)

### ★ Checkpoint 4: Daily-Driver Production Sign-Off
- [ ] All Success Criteria in [SPEC-route-b-native-shell.md](../SPEC-route-b-native-shell.md) checked off on physical KPW3.
- [ ] User decides whether to promote config/launcher to production `/mnt/us/anki_data`.

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
| :--- | :--- | :--- |
| ES5 KaTeX visual edge case on 2012 WebKitGTK 1.0 (e.g. `<ruby>` wrapping math or deep nested fractions) | High | Verified early in Checkpoint 1 (`1072×1448` preview) & Task 13 Spike 1; switchable MathJax 2.7.9 fallback already built into `card-renderer`. |
| GTK2/Cairo/Pango FFI struct or signature mismatch on 32-bit `armel` ABI (`c_int`/`c_long` = 32-bit) | High | Use `std::ffi::{c_int, c_uint, c_long, c_double}` strictly, verify exact GTK2 2.24 / Cairo 1.16 / Pango 1.42 C headers inside Debian Buster container, and compile in CI at Task 8 before building higher views. |
| `mxcfb` ioctl vs `eips -f ""` behavior variance on Kindle OS 5.14.3.0.1 | Medium | Implement safe fallback chain (`eips -f ""` verified shell utility + optional `mxcfb` ioctl) and verify cleanly in Spike 2 without crashing. |
| Kindle `awesome`/`pillow` window manager covering `GtkDialog` with virtual keyboard | Medium | Use proven Lab126 top-aligned dialog window title (`L:D_N:dialog_ID:ranki.settings_PC:N` / top-positioned dialog) modeled on upstream RAnki's `settings.vala` & verify in Spike 3. |
| Host Windows machine lacks local `cargo`/`docker` | Low | Local Node 22 + Python 3.13 run all frontend/formula/preview/gate checks instantaneously; GitHub Actions CI (`Dockerfile.armel`) runs `cargo test --lib` + ARMEL cross-compilation cleanly per push. |
