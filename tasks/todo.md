# RAnki Tasks (tasks/todo.md)

## Route B (Active): Native GTK2/Cairo Shell + Ultra-Fast Resident Card Engine
> Source Spec: `SPEC-route-b-native-shell.md` | Plan: `tasks/plan.md` | Intent: `docs/intent/kpw3-anki-daily-driver.md`

### Phase 0: Governance, Security & Tooling Hygiene
- [x] **Task 1**: Branch `feat/route-b-native-shell`, remove hardcoded `hkey` from `tools/deploy_to_kindle.py`, fix `tools/download_artifact.py` artifact name, harden `tools/check_es5.js` (ES5 + banned WebKit 1.0 APIs + zero Emoji)

### Phase 1: Ultra-Fast Centered Card Renderer (`card-renderer`)
- [x] **Task 2**: Restore verified ES5 KaTeX bundle (`views/katex/`, 635 KB, 20 WOFF fonts, `vlist-t2` patch) + `tools/test_regression.js` (838/838 formulas PASS)
- [ ] **Task 3**: Build `views/card.html`, `views/card.css`, `views/card.js` (`<div class="card kindle">` official centering, `Anthropic Sans` + `SonyReader Ming`, calibrated `28px` + adjustable `20–38px`, monochrome `<font color="red">`, `<ruby>`, `<cite>`, `\bbox`, Card-ID 4-slot buffer, MathJax 2.7.9 fallback, discrete `±85%` paging)
- [ ] **Task 4**: Build `tools/preview_cards.html` (`1072×1448` local desktop preview with user's real STEM cards)
- [ ] **Checkpoint 1**: 838/838 formula regression PASS, `check_es5.js` PASS, human visually approves `1072×1448` centered paper-book typography

### Phase 2: Rock-Solid `anki-core` & Background Async Sync
- [ ] **Task 5**: Upgrade `src/config.rs` (safe Route B settings schema, clamped values, atomic save, zero hardcoded secrets, unit tests)
- [ ] **Task 6**: Refactor `src/anki/mod.rs` (Card-ID-verified `AdvanceResult`, deduplicated root `total_due`, `QueueCounts` + `ActiveQueue` underline, `undo()`, `bury_current()`, `suspend_current()`, WAL checkpoint, unit tests)
- [ ] **Task 7**: Implement `src/anki/sync.rs` (background worker thread + `g_idle_add`, DB-only `normal_sync`, explicit `FullSyncRequired` + `full_download`/`full_upload`, zero media download) & add host `cargo test --lib` to CI
- [ ] **Checkpoint 2**: `anki-core` builds & passes all unit tests in CI

### Phase 3: Native GTK2/PangoCairo Shell (`native-shell`)
- [ ] **Task 8**: Build thin FFI layer (`src/ffi/{gtk,cairo,webkit}.rs`), update `build.rs` (whitelisted GTK2/Cairo/Pango/Fontconfig), `src/ui/theme.rs` (Fontconfig app fonts), and `src/ui/eink.rs` (debounced LIPC keyboard + hardware GC16 full refresh)
- [ ] **Task 9**: Build `src/ui/deck_view.rs` (100% PangoCairo minimalist paginated Deck Browser, 10 rows/page, subdeck fold/unfold, swipe paging, zero tofu)
- [ ] **Task 10**: Build `src/ui/focus_timer.rs` (isolated `52×52px` Cairo 1 Hz discrete 20s iPhone Duo circular focus timer, pure monochrome, gentle 0s cue, stop/hide on Flip)
- [ ] **Task 11**: Build `src/ui/review_view.rs` (Review Top Bar with `新+学+复` underline, Undo, `···` overflow menu, subtle `1/2` indicator, embedded `focus_timer`; resident card WebView with GDK-level zero-bar swipe/tap paging; PangoCairo Bottom 4-Rating Bar)
- [ ] **Task 12**: Build `src/ui/settings_dialog.rs` (native `GtkDialog` + `GtkEntry` + LIPC keyboard + font/timer/refresh/engine controls + non-blocking Sync/`FullSync` dialogs) and wire `src/ui/mod.rs` + `src/main.rs`
- [ ] **Checkpoint 3**: GitHub Actions ARMEL CI build PASS + all 4 mechanical gates PASS (`GLIBC <= 2.18`, `DT_NEEDED`, 0 `.dynsym` leaks, ES5 + banned APIs, ≤ 20 MB)

### Phase 4: KPW3 Real-Device Validation
- [ ] **Task 13**: Safe dev deploy to `D:\extensions\ranki-dev\` (`D:\anki_data_test\`, 0-byte `D:\anki_data\` guarantee) + 3 hardware spikes (KaTeX visual/speed, 52×52px Duo timer + GC16 + GDK swipe paging, native keyboard + Wi-Fi sync + Undo/Bury/Suspend) + 50–100 card stress test
- [ ] **Checkpoint 4**: Real-device daily-driver sign-off & user decision on production switch

---

## Historical Archive (Previous Engineer V1–V4 — Superseded by Route B)
- [x] V1: Pure-Rust Host & Zero-Copy Engine
- [x] V2: Anki-Authentic UI & Next-Card Pipeline Pre-Rendering
- [x] V3: AnkiDroid Modern Aesthetic, AnkiWeb Sync & Login/Settings
- [x] V4: E-ink Experience Hardening & Real-Device Bug Triage
