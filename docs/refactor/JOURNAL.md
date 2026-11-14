# RAnki Refactoring Engineering Journal (工程日志)

> 规则：每次改动追加一条。缺任何一项必填字段，该条目不得标为 `verified-device`。

---

## M0 构建基线锁定日志

### 记录 001 | M0.1 稳定版物理基线固化
* **日期时间**：2026-11-14 06:16:00 UTC
* **里程碑与任务编号**：Milestone M0, Task 0.1
* **提交 SHA**：`f3c13d3c5c2db918cb6bcbb42f8b45bf93e638e1` (Target Baseline Tag `v0.2`)
* **变更内容**：提取真实硬件上运行的 `ranki-armel`，固化为永久备份 `ranki-armel.stable`，抽取 SHA256 与 ELF 结构证据。
* **构建命令全文**：`N/A (Upstream baseline capture)`
* **产物 sha256**：`f8e5a2815fb5914939fb62f741b4c07670922ac967e7679d1d5e0052ffe7c4a3` (27,349,404 字节)
* **`readelf -V` GLIBC 最高版本**：`GLIBC_2.18` (来自 `libc.so.6`)
* **部署位置**：`/mnt/us/extensions/ranki/ranki-armel.stable`
* **真机运行结果**：`evidence/m0/baseline_binary/`
* **状态**：`verified-device`


### 记录 002 | M0.2 源码基准版本比对与锁定
* **日期时间**：2026-11-14 06:17:30 UTC
* **里程碑与任务编号**：Milestone M0, Task 0.2
* **提交 SHA**：`f3c13d3c5c2db918cb6bcbb42f8b45bf93e638e1` (Tag `v0.2`)
* **变更内容**：通过真机 stdout 调试日志行号 (`review.vala:355/371`) 与二进制编译时间戳精确锁定源码提交为 `f3c13d3c`。
* **构建命令全文**：`N/A (Version audit)`
* **产物 sha256**：`evidence/m0/commit_match.txt`
* **`readelf -V` GLIBC 最高版本**：`GLIBC_2.18`
* **部署位置**：`N/A`
* **真机运行结果**：`evidence/m0/commit_match.txt`
* **状态**：`verified-device`

### 记录 003 | M0.3-M0.6 自动化云端交叉编译基线与门禁达成
* **日期时间**：2026-11-14 10:21:22 UTC
* **里程碑与任务编号**：Milestone M0, Tasks 0.3-0.6
* **提交 SHA**：`833f85f` (GitHub Actions Run `43584407380`)
* **变更内容**：实现完整 GitHub Actions 自动化 ARMEL 交叉编译流水线：解决 Debian Buster 多架构开发环境冲突，编译 Rust rslib 25.09.02，静态合流 json-glib 与 libarchive，通过 glibc_compat 垫片和 ld --wrap 拦截高版本符号，达成 <= GLIBC_2.20 与 Kindle OS 26 动态库白名单双门禁。
* **构建命令全文**：`docker run --rm -v $PWD:/workspace ranki-armel-builder /bin/bash /workspace/tools/ci_build_armel.sh`
* **产物 sha256**：`1157fe75e8da67fb68db82ead1d4f593fa00fcb1972c194960b506dc26f85dce` (38,739,824 字节)
* **`readelf -V` GLIBC 最高版本**：`GLIBC_2.18`
* **部署位置**：`build/ranki-armel` (GitHub Artifact `ranki-armel-m0`, Local: `scratch/build_output/ranki-armel`)
* **真机运行结果**：自动化门禁通过（GLIBC 2.18 <= 2.20 [PASS]，DT_NEEDED 16/16 全部白名单 [PASS]）
* **状态**：`verified-offline`
