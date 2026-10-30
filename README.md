# vivo X Flip (PD2256) GhostLock 5.10.233 profile builder

在 GitHub Actions 上编译 Android GKI `android12-5.10.233` 内核，产出带 BTF 的
`Image`，再用 GhostLock 的 `ghostlock-extract` 生成一份完整的
`profile.conf`，用于给 **vivo X Flip（V2256A / PD2256）** 做临时 root。

## 为什么需要它

设备内核 `5.10.233-gki-g32ba094b20cd`：

- `# CONFIG_DEBUG_INFO_BTF is not set`，`/sys/kernel/btf` 不存在；
- `/proc/kallsyms` 对 shell 不可读；
- GhostLock 内置 profile 没有 5.10 版本。

因此提取器只能从 `boot.img` 里恢复函数符号，拿不到 `task_struct` / `cred`
等结构体偏移，导出的 conf 是残缺的，应用 fail-closed 拒绝执行。

这里用设备自己的 `/proc/config.gz`（`configs/pd2256_defconfig`）重新编译同版本
内核并打开 `CONFIG_DEBUG_INFO_BTF`，让提取器可以从 `Image` 内嵌的 BTF 推导偏移。

## 内容

```
configs/pd2256_defconfig      # 设备 /proc/config.gz 原样导出
extract_rs/                   # GhostLock tools/extract_rs 源码副本
.github/workflows/build.yml   # 编译 + 生成 profile
```

## 构建要点

- 源码：`android12-5.10.233_r00`（设备 config 对该分支 `gki_defconfig`
  缺失 0 个符号；`android13-5.10` 缺 19 个）
- 工具链：AOSP `clang-r416183b`（与设备 `Linux version ... clang 12.0.5 (r416183b)` 一致）
- 关闭 `LTO_CLANG*` / `CFI_CLANG`：结构体布局不受影响，但全量 LTO + DWARF
  在托管 runner 上跑不动
- 强制打开 `CONFIG_DEBUG_INFO_BTF=y`（依赖 `DEBUG_INFO` + `DEBUG_INFO_DWARF4`）
- `--phys 0xa8000000`：设备内核日志里 `kernel_phys_load=0xa8000000`（UEFI memory map）
- 生成后把 `release` 强制改写为设备实际的 `5.10.233-gki-g32ba094b20cd`

## 使用

1. Actions → `build-gki-5.10.233-btf-profile` → Run workflow；
2. 下载 artifact `ghostlock-5.10.233-profile`；
3. 手机 GhostLock → 载入配置 → **导入 offsets conf** → 选择 `profile.conf`；
4. 执行 GhostLock（临时 root，重启失效）。

## 产物自检

- `extract.log` 里不应再出现 `credential template omitted`、`embedded BTF not found`；
- `profile.conf` 里 `task_struct` / `cred` / `offset` 不应有 `null`；
- `release` 必须等于 `5.10.233-gki-g32ba094b20cd`；
- `offset.init_task`、`offset.init_cred`、`offset.root_task_group`、
  `offset.selinux_enforcing` 必须存在（应用导入时的必填项）。

## 风险

内核提权操作，错误偏移可能导致 panic / 重启。临时 root 重启即失效，
不会修改系统分区。请先备份数据。
