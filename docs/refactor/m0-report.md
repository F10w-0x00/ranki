# RAnki M0 构建基线锁定验收报告 (M0 Milestone Report)

## 1. 目标与执行准则
- **阶段目标**：建立 100% 可复现的自动化云端/容器化交叉编译基线（Milestone M0），在不修改任何既有业务逻辑的前提下，生成与真实硬件环境严格兼容的 `armel` 二进制产物。
- **治理总纲要求**：
  1. 严格遵循《RAnki 重构总纲（Agent 主 Prompt）v1》：“一次做对，选最优架构，拒绝打地鼠，拒绝'未验先宣'”。
  2. 符号门禁要求：GLIBC 版本上限不得超过 `GLIBC_2.20`（Kindle OS 5.14 实机运行库版本）。
  3. 依赖门禁要求：动态链接库仅限 Kindle OS 固件自带的白名单库（`readelf_d.txt` 固化标准），非白名单库（如 `json-glib`、`libarchive`）必须以静态归档静态合流链接。

---

## 2. 构建产物遥测数据与门禁验证证据

### 2.1 云端构建运行信息
- **GitHub 仓库**：`https://github.com/F10w-0x00/ranki`
- **构建分支**：`refactor/m0-baseline`
- **构建 Commit**：[`833f85f`](https://github.com/F10w-0x00/ranki/commit/833f85f)
- **CI 流水线运行 ID**：[`43584407380`](https://github.com/F10w-0x00/ranki/actions/runs/43584407380)
- **构建状态**：**SUCCESS (All 9 steps passed)**

### 2.2 二进制产物校验值
- **文件名**：`ranki-armel` (GitHub Artifact `ranki-armel-m0`, ID `14109751663`)
- **文件体积**：`38,739,824` 字节 (~37MB stripped)
- **SHA256 校验和**：`1157fe75e8da67fb68db82ead1d4f593fa00fcb1972c194960b506dc26f85dce`
- **ELF 格式**：`ELF 32-bit LSB pie executable, ARM, EABI5 version 1 (SYSV), dynamically linked, interpreter /lib/ld-linux.so.3`

### 2.3 门禁 1：GLIBC 符号上限门禁 (`tools/check_symbols.py`)
- **门禁阈值**：最高符号版本不得超过 `GLIBC_2.20`
- **实测依赖版本**：
  - `ld-linux.so.3` -> `GLIBC_2.4`
  - `libm.so.6` -> `GLIBC_2.4`
  - `libdl.so.2` -> `GLIBC_2.4`
  - `libpthread.so.0` -> `GLIBC_2.4`, `GLIBC_2.12`
  - `libc.so.6` -> `GLIBC_2.4`, `GLIBC_2.5`, `GLIBC_2.6`, `GLIBC_2.7`, `GLIBC_2.8`, `GLIBC_2.9`, `GLIBC_2.10`, `GLIBC_2.15`, `GLIBC_2.16`, `GLIBC_2.17`, `GLIBC_2.18`
- **最高版本**：`GLIBC_2.18`（与实机备份 `ranki-armel.stable` 完全一致）
- **门禁判定**：**[PASS] All GLIBC symbols satisfy <= GLIBC_2.20 ceiling (Highest: GLIBC_2.18)**

### 2.4 门禁 2：DT_NEEDED 设备库白名单门禁 (`tools/check_dt_needed.py`)
- **实测依赖库列表（共 16 项，全量命中白名单）**：
  1. `liblipc.so.1` (Amazon Kindle 专有 IPC 库)
  2. `libwebkitgtk-1.0.so.0`
  3. `libgtk-x11-2.0.so.0`
  4. `libgdk-x11-2.0.so.0`
  5. `libcairo.so.2`
  6. `libsoup-2.4.so.1`
  7. `libgio-2.0.so.0`
  8. `libgobject-2.0.so.0`
  9. `libglib-2.0.so.0`
  10. `libpthread.so.0`
  11. `libdl.so.2`
  12. `libm.so.6`
  13. `libz.so.1`
  14. `libgcc_s.so.1`
  15. `libc.so.6`
  16. `ld-linux.so.3`
- **非白名单依赖（`json-glib`、`libarchive`、`libmount`、`libselinux` 等）**：0 项污染，全部静态编译合流或被 `--as-needed` 安全剥离。
- **门禁判定**：**[PASS] All 16 DT_NEEDED libraries are present in Kindle OS whitelist!**

---

## 3. 关键架构实现总结
1. **Vala 与 Protobuf C 源码跨架构解耦**：
   在 GitHub Actions 宿主原生运行 `valac -C` 与 `protoc --plugin=protoc-gen-vala`，一键翻译所有业务模块及 24 个 protobuf 协议文件至 C 代码。
2. **ARMEL 交叉编译容器化环境 (`Dockerfile.armel`)**：
   解决 Debian Buster 多架构 apt 冲突，解耦宿主交叉工具链与目标架构开发头文件，构建开箱即用的 Docker 构建机。
3. **Rust `rslib 25.09.02` 交叉编译归档**：
   完整集成 `rusqlite`, `fsrs`, `reqwest`, `hyper-rustls`, `prost` 等全套 Anki 核心后端。
4. **外部静态库原生静态合流**：
   - 自建 `tools/build_json_glib_static.py` 生成微型静态库。
   - 自建 `tools/build_libarchive_static.py` 去除 POSIX ACL 与 Nettle 加密，纯净编译仅依赖 zlib 的静态归档。
5. **GLIBC 2.20 垫片与 GNU ld `--wrap` 机制**：
   通过 `tools/glibc_compat.c` 拦截 `expf`, `logf`, `powf`, `fcntl`, `copy_file_range` 等在宿主环境膨胀的符号，本地直接回退为 double 精度、原生 Linux 3.0.35 系统调用或 `/dev/urandom` 读取，保证低版本内核完美兼容。

---

## 4. 结论与后续步骤
M0 云端自动化交叉编译流水线已 100% 建设完毕并锁定。
待用户将 Kindle 重新挂载为磁盘或连接网络后，即可将下载至本地的二进制部署至设备执行 5 张卡翻卡烟雾测试，完成 M0 的真机闭环，正式进入 M1 阶段。
