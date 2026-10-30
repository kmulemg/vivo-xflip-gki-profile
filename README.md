# vivo X Flip (PD2256) GhostLock 5.10.233 profile builder

在 GitHub Actions 上编译 Android GKI `android12-5.10.233` 内核，打开
`CONFIG_DEBUG_INFO_BTF`，产出**结构体偏移**；再配合设备 `boot.img` 里的
kallsyms，拼出一份可用于 **vivo X Flip（V2256A / PD2256）** 的 GhostLock profile。

> ## ⚠️ 产出的 `profile.conf` 不能直接用
>
> artifact 里的 `profile.conf` 是拿**我们自己编译的 Image** 跑出来的，
> 它的 `offset{}`（`init_task` / `init_cred` / `root_task_group` /
> `selinux_enforcing` …）是**那份构建产物的符号偏移**，与设备真实内核
> **完全不同**（实测 133,004 vs 176,063 个符号，差值还不固定）。
>
> 直接导入执行 → 内核 panic / 重启。
>
> 正确做法：
> 1. **结构体偏移**（`task_struct.*` / `cred.*`）用本仓库产物（BTF 推导）；
> 2. **符号偏移**（`offset{}`）必须换成设备自己 `boot.img` 的 kallsyms 值
>    （`ghostlock-extract <设备 boot.img> --format json`）；
> 3. `cred.ref*_image` 也要换成设备 `init_cred` 里的真实指针。
>
> 详细流程见 issue：https://github.com/YuKongA/ghostlock-app/issues/723

## 为什么需要它

设备内核 `5.10.233-gki-g32ba094b20cd`：

- `# CONFIG_DEBUG_INFO_BTF is not set`，`/sys/kernel/btf` 不存在；
- `/proc/kallsyms` 对 shell / 应用都不可读；
- GhostLock 内置 profile 没有 5.10 版本，且 5.10 的 `pselect_waiter_shift`
  推导不出来。

所以提取器只能从 `boot.img` 恢复函数符号，拿不到 `task_struct` / `cred`
的结构体偏移，导出的 conf 是残缺的（应用 fail-closed 拒绝执行）。

本仓库用设备自己的 `/proc/config.gz` 编译同版本内核并打开 BTF，
让提取器能推导结构体偏移。

## 内容

```
configs/pd2256_defconfig      # 设备 /proc/config.gz 原样导出
extract_rs/                   # GhostLock tools/extract_rs 源码副本
.github/workflows/build.yml   # 编译 + 生成 profile + 导出 btf.bin
```

## 构建要点

- 源码：`android12-5.10.233_r00`（设备 config 对该分支 `gki_defconfig`
  缺失 0 个符号；`android13-5.10` 缺 19 个）
- 工具链：AOSP `clang-r416183b`（与设备 `Linux version ... clang 12.0.5 (r416183b)` 一致）
- 所有 `make` 都要带 `CROSS_COMPILE=aarch64-linux-gnu-`，否则 clang 会按宿主
  x86_64 编译（报 `register 'sp' unsuitable for global register variables`）
- 关闭 `LTO_CLANG*` / `CFI_CLANG`：不改结构体布局，但全量 LTO + DWARF
  在托管 runner 上跑不动
- 强制打开 `CONFIG_DEBUG_INFO_BTF=y`（依赖 `DEBUG_INFO` + `DEBUG_INFO_DWARF4`）
- `--phys 0xa8000000`：设备内核日志里 `kernel_phys_load=0xa8000000`（UEFI memory map）
- 生成后把 `release` 强制改写为设备实际的 `5.10.233-gki-g32ba094b20cd`

## 产物

| 文件 | 说明 |
| --- | --- |
| `profile.conf` | 结构体偏移可用，**符号偏移必须在设备侧替换**（见上方警告） |
| `btf.bin` | 从 `vmlinux` 的 `.BTF` 段 dump 出来，可直接用于其它 BTF 消费方 |
| `extract.log` | 提取器完整输出（含未推导出的字段） |
| `.config` | 实际使用的内核配置 |

## 使用

1. Actions → `build-gki-5.10.233-btf-profile` → Run workflow；
2. 下载 artifact `ghostlock-5.10.233-profile`；
3. 用**设备固件**的 `boot.img` 跑一遍提取器，拿到设备侧符号偏移：
   ```bash
   ghostlock-extract boot.img --format json --phys 0xa8000000 --out device.json
   ```
4. 把 `device.json` 里的 `off_*` 值覆盖进 `profile.conf` 的 `offset{}`；
5. 手机 GhostLock → 载入配置 → 导入 offsets conf。

## 产物自检

- `extract.log` 里不应再有 `credential template omitted` / `embedded BTF not found`；
- `profile.conf` 的 `task_struct` / `cred` 不应有 `null`；
- `release` 必须等于 `5.10.233-gki-g32ba094b20cd`；
- `offset.init_task`、`offset.init_cred`、`offset.root_task_group`、
  `offset.selinux_enforcing` 必须存在（应用导入时的必填项）；
- 可用 `init_task` 处的 `comm == "swapper"`、`tasks` 自引用、`cred` 指向
  `init_cred` 来验证结构体偏移（详见 issue #723）。

## 现状（2026-10）

profile 本身已验证正确（14 项镜像实证），但 GhostLock native 侧
**5.10 支持未完成**（`#721` draft），实测卡在 `prepare_kernel_page` 的偶字节
拒绝；`mm_struct_sz` 用 960 会直接 panic，1024 为“干净失败”值。

## 风险

内核提权操作，错误偏移可能导致 panic / 重启。临时 root 重启即失效，
不会修改系统分区。请先备份数据。
