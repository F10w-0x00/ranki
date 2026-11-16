# RAnki Pure Rust Refactor Specification (SPEC.md)

> **Document Status**: Definitive Feature & Technical Specification  
> **Source of Truth**: HANDOFF.md v5.0.0-Reviewed  
> **Target Device**: Amazon Kindle Paperwhite 3 (DP75SDI, Kindle OS 5.14.3.0.1, Linux 3.0.35-lab126, armel)  
> **Last Updated**: 2026-11-16  

---

## 1. Objective

Refactor RAnki from a brittle hybrid architecture (86.7% Vala, 11.3% Rust, 2% C) into a **unified pure Rust host with a single WebKit-based Single Page Application (SPA)**.
- Eliminate Vala, Protobuf IPC, and GTK TreeView (which caused the "text quagmire" tearing on E-ink).
- Eliminate cross-allocator memory collisions (T01 SIGSEGV 139) by isolating static C/Rust libraries (`sqlite3_*`).
- Deliver instant card flipping ($\le 5\text{ms}$) via dual-buffer offscreen pre-rendering while preserving 100% LaTeX/AMS-TeX compatibility via offline MathJax 2.7.9.
- Achieve 300 PPI print-grade E-ink typography without dithering artifacts.
- Enforce absolute production data safety (`D:\anki_data\` strictly isolated).

---

## 2. Key Commands & Execution Pipeline

```bash
# === Local Slices (Windows Host / Python 3.13 / Node.js) ===
# Slice H1: Three Mechanical Gate Assertions
python tools/check_symbols.py build/ranki-armel
python tools/check_dt_needed.py build/ranki-armel
python tools/check_dynsym.py build/ranki-armel

# Slice H2: Frontend ES5 & Dual-Buffer Verification
node tools/check_es5.js views/app.js

# Slice H3: Pure Rust Backend Logic Unit Tests
cargo test --package ranki --lib

# === Cloud CI Build Pipeline (GitHub Actions / Dockerfile.armel) ===
# Triggered by push to origin/refactor/m0-baseline
# Builds: build/ranki-armel (ET_EXEC, arm-unknown-linux-gnueabi, GLIBC <= 2.20)

# === Slice H4: Kindle PW3 On-Device Smoke & Verification ===
# Copy build/ranki-armel -> D:\extensions\ranki-dev\ranki-armel
# Execute on Kindle via KUAL -> RAnki Dev (M0)
```

---

## 3. Project Structure

```
ranki/
├── Cargo.toml                  # Pure Rust workspace / root manifest
├── src/                        # Rust Host Source Code
│   ├── main.rs                 # Entry point, GtkWindow & WebKitWebView setup
│   ├── ffi/
│   │   ├── mod.rs              # Minimal direct C-FFI (~80 LOC) for GTK/WebKit
│   │   └── webkit.rs           # JS script evaluation & title notify hooks
│   ├── anki/
│   │   ├── mod.rs              # Direct rslib bridge (Collection::open)
│   │   ├── decks.rs            # Deck tree extraction
│   │   └── reviewer.rs         # Card queue & FSRS/SM-2 scheduling
│   └── ipc.rs                  # notify::title parser & window.RAnki dispatcher
├── views/                      # Unified Web SPA
│   ├── index.html              # Single HTML shell (#card-active, #card-buffer)
│   ├── app.js                  # ES5 State machine & Queue Lock flip controller
│   ├── style.css               # Print-grade CSS (pure black, 22px, 48px touch targets)
│   └── mathjax/                # Vendored minimal MathJax 2.7.9 & TeX .woff fonts (~1.8MB)
├── tools/                      # Build & Verification Tooling
│   ├── check_symbols.py        # GATE-01: GLIBC <= 2.20 symbol ceiling
│   ├── check_dt_needed.py      # GATE-02: 16 whitelist dynamic libraries
│   ├── check_dynsym.py         # GATE-03: 0 leaked static symbols
│   ├── check_es5.js            # GATE-04: ES5 syntax parser gate
│   ├── ci_build_armel.sh       # Containerized armel build script
│   └── glibc_compat.c          # Syscall & math wrapper shim
├── .github/workflows/
│   └── build-armel.yml         # Containerized CI workflow
├── CONSTRAINTS.md              # Quality contract & hard constraints
├── HANDOFF.md                  # Master architectural specification
└── SPEC.md                     # This specification
```

---

## 4. Code Style & Architecture Rules

1. **Rust Backend**:
   - Idiomatic Rust 2021, zero unhandled `unwrap()` in production paths.
   - Minimal C-FFI: no heavy C bindings crate; directly declare needed GTK2/WebKit1 extern functions.
   - SQLite transactions committed per card score; `sqlite3_wal_checkpoint` on deck change/exit.
2. **Frontend SPA**:
   - Strict ES5 syntax (no `let`, `const`, `()=>{}`, `class`, `Promise`, `async/await`).
   - Zero `localStorage` (causes DOM Exception 18 on `file://`).
   - Dual-buffer DOM swap: `#card-buffer` must remain geometrically measurable (`position: absolute; left: -9999px; visibility: hidden;`).
   - Queue Lock: Defer card flip if user taps "Show Answer" before MathJax finishes typesetting.
3. **Typography Standards**:
   - Pure black `#000000 !important;` for text and formulas.
   - TeX vector fonts only (`availableFonts: ["TeX"]`); no bitmap fallback.
   - Base font size `22px`, line-height `1.5`, touch targets $\ge 48\text{px} \times 48\text{px}$.

---

## 5. Testing & Verification Strategy

- **H1**: Python gate scripts checking ELF header, symbols, dependencies, and dynamic symbol isolation.
- **H2**: Node.js ES5 parsing and local browser layout/timing verification.
- **H3**: Rust unit testing on `D:\anki_data_test\` database reading and scheduling.
- **H4**: Kindle PW3 physical smoke test on `RAnki Dev (M0)` with 5 cards, followed by 100-card stress test in Task 4.

---

## 6. Boundaries & Invariants

- **Always**: Target `D:\anki_data_test\`; use Unix LF endings; assert all 4 gates before device deployment.
- **Ask First**: Any schema change or external toolchain modification.
- **Never**: Touch `D:\anki_data\`; introduce Vala/Protobuf code; use KaTeX; suppress compiler warnings.
