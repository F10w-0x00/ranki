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
