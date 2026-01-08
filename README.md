# ThinkPad X1C Power Optimizer

<p align="center">
  <a href="#中文">中文</a> ·
  <a href="#english">English</a> ·
  <a href="https://github.com/YuChenXin-ZJU/ThinkPad-X1C-Power-Optimizer/releases/latest">Release 下载 / Downloads</a>
</p>

---

# 中文

## 简介
这是一个用于 Windows 的桌面工具（Tauri）。它主要面向 ThinkPad X1 Carbon（也可用于其他 Windows 设备），提供一组一键操作，帮助你：

- 查看当前机器的电源计划（Power Scheme）
- 将电源计划备份为 `.pow` 文件（便于回滚/迁移）
- 在不改变电池模式参数的前提下，让插电模式参数与电池模式保持一致（插电更流畅）
- 可选：禁用 Lenovo ITS Power Mode Control 服务，降低第三方服务覆写电源策略的概率（可恢复）
- 重置回 Windows 默认电源计划并尝试恢复 Lenovo ITS 服务

本工具通过调用 Windows 自带命令（`powercfg`、`sc`、`net`、`schtasks` 等）完成操作，不需要联网。

## 修改原理（工作机制）

### 1) 插电更流畅做了什么？
Windows 的每个电源计划（scheme）都包含大量电源设置。大多数设置会分别保存两份值：

- DC（电池供电 / Discharging）
- AC（插电供电 / Plugged in）

在一些机器/驱动组合里，AC 与 DC 的参数差异可能很大，导致切换到插电后出现明显的调度策略变化，从而产生体感上的卡顿、掉帧、响应变慢等问题。

本工具的插电更流畅（推荐）采用的策略是：

1. 读取当前正在使用的电源计划 GUID（`powercfg /getactivescheme`）
2. 读取该电源计划的所有电源设置（`powercfg /query <scheme_guid>`）
3. 从输出中解析每个设置的 Current DC Power Setting Index（当前 DC 值）
4. 对每个设置执行：把 AC 值改成同一个数值  
   `powercfg /setacvalueindex <scheme_guid> <subgroup_guid> <setting_guid> <dc_value>`
5. 最后重新激活一次该方案（`powercfg /setactive <scheme_guid>`），确保立即生效

也就是说：只把插电模式（AC）改成电池模式（DC）的数值，并不会修改 DC 本身。

为降低对睡眠/唤醒行为的影响，工具会跳过 Sleep 子组（Subgroup GUID：`238c9fa8-0aad-41ed-83f4-97be242c8f20`）下的设置项。

### 2) 禁用 Lenovo ITS 做了什么？为什么可能有用？
部分联想机型会安装并运行服务 Lenovo ITS Power Mode Control。该服务可能会根据联想策略动态调整电源相关设置，导致你手动修改电源计划后又被改回去。

当你选择插电更流畅 + 禁用 Lenovo ITS（高级）时：

1. 先执行插电更流畅的电源计划修改
2. 然后：
   - 读取该服务对应的服务名（`sc getkeyname "Lenovo ITS Power Mode Control"`）
   - 备份该服务原本的启动类型与运行状态到：  
     `Desktop\\ThinkPadX1PowerOptimize\\its-service-backup.json`
   - 尝试停止服务：`sc stop <service>`
   - 设置服务为禁用：`sc config <service> start= disabled`

对应的重置并恢复 Lenovo ITS 会尝试：

1. 恢复 Windows 默认电源计划：`powercfg /restoredefaultschemes`
2. 切回默认平衡方案（Balanced）：`powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e`
3. 如果存在 `its-service-backup.json`，按备份信息恢复 ITS 服务启动类型，并按需启动

## 电源体系与守护式逻辑（精简但无歧义）

从底到顶的真实层级：

1. 硬件/微码层（不可控）：PL1/PL2、C-State、温控等，软件无法越权。
2. ACPI/Firmware 层（只读感知）：提供允许的策略空间。
3. Windows Power Framework（真正生效层）：所有 GUID（SUB_PROCESSOR 等）最终落在这里。
4. Policy Clients（竞争写入层）：Windows 电源管理、Intel DTT/IPF、Lenovo ITS/Vantage、你的软件。

关键结论：

- 没有锁、没有 owner，最后写入者生效。
- 你的软件本质是参与竞争，确保自己最后写入。
- 所以一次性设置不稳定，必须守护式持续校正。

当前实现：

- 通过计划任务在登录/唤醒/电源切换后重写 AC=DC。
- 同时启动后台 Watchdog（常驻脚本），保证软件关闭后仍持续生效。

## 软件功能

### 查看本机电源计划
- 列出所有电源计划并标记当前活动计划
- 对常见内置计划 GUID（平衡/高性能/节能/卓越性能）显示简要解释

### 备份电源计划到桌面（需要管理员权限）
- 逐个执行 `powercfg /export` 导出 `.pow`
- 输出每个计划的导出结果与失败原因
- 备份目录为：  
  `Desktop\\ThinkPadX1PowerOptimize\\power-plans-backup-<timestamp>\\`

### 插电更流畅（推荐）（需要管理员权限）
- 把插电时的电源参数调整为与电池模式一致
- 跳过睡眠子组设置，减少对睡眠/唤醒行为的影响
- 软件输出面板会显示更新数量 / 失败数量 / 跳过数量

### 插电更流畅 + 禁用 Lenovo ITS（高级）（需要管理员权限）
- 在插电更流畅的基础上，禁用 Lenovo ITS Power Mode Control 服务（可恢复）

### 启用自动回写（推荐，需要管理员权限）
- 登录/唤醒/电源切换触发重写 AC=DC
- 启动后台守护脚本，软件关闭后仍持续校正

### 重置并恢复 Lenovo ITS（需要管理员权限）
- 恢复 Windows 默认电源计划
- 切回默认平衡方案
- 尝试恢复 Lenovo ITS 服务

## 运行环境

### 支持的系统
- Windows 10 / Windows 11（x64）
- 需要 WebView2 Runtime

### 权限要求
- 查看本机电源计划通常不需要管理员权限
- 备份 / 插电更流畅 / 禁用 ITS / 重置 / 自动回写等涉及修改电源计划或服务配置的操作需要管理员权限
- 软件内提供 `⚠️ 以管理员身份重新启动` 按钮用于提权重启

### 数据与文件位置
- 本工具不会上传任何数据
- 备份文件与 ITS 备份文件默认写入桌面目录：  
  `Desktop\\ThinkPadX1PowerOptimize\\`
- 自动回写脚本默认写入：  
  `%USERPROFILE%\\.Thinkpad_Power\\`

## 免责声明
本软件为个人实验性质工具，可能修改系统电源计划与相关服务配置。使用本软件所产生的任何风险与后果（包括但不限于数据丢失、系统异常、硬件损坏或其他损失）均由使用者自行承担，作者不对此承担责任。

---

# English

## Overview
This is a Windows desktop utility (Tauri). It targets ThinkPad X1 Carbon and can work on other Windows PCs. It provides one-click actions to:

- List power plans
- Backup power plans to `.pow`
- Keep AC values consistent with DC values
- Optionally disable Lenovo ITS Power Mode Control (restorable)
- Reset power schemes and attempt to restore Lenovo ITS

The app uses built-in Windows commands (`powercfg`, `sc`, `net`, `schtasks`, etc.) and does not require network access.

## How It Works (Principle)

### 1) What does Smoother on AC do?
Each Windows power plan (scheme) contains many power settings. Most settings keep two independent values:

- DC: on battery (discharging)
- AC: plugged in

On some systems, AC/DC values differ significantly. When switching to AC, the policy change can feel like stutter, sluggishness, or inconsistent responsiveness.

The Smoother on AC (recommended) action:

1. Gets the active scheme GUID (`powercfg /getactivescheme`)
2. Queries all settings (`powercfg /query <scheme_guid>`)
3. Parses `Current DC Power Setting Index` for each setting (the DC value)
4. Sets the AC value to match DC for each setting  
   `powercfg /setacvalueindex <scheme_guid> <subgroup_guid> <setting_guid> <dc_value>`
5. Re-activates the scheme (`powercfg /setactive <scheme_guid>`)

In short: it updates AC values only; DC values remain unchanged.

To reduce impact on sleep behavior, it skips the Sleep subgroup (Subgroup GUID: `238c9fa8-0aad-41ed-83f4-97be242c8f20`).

### 2) What does Disable Lenovo ITS do?
Some Lenovo setups run a background service called Lenovo ITS Power Mode Control. It may override power plan values dynamically, which can undo manual changes.

The Smoother on AC + disable Lenovo ITS (advanced) action:

1. Performs Smoother on AC
2. Then:
   - Resolves the service name (`sc getkeyname "Lenovo ITS Power Mode Control"`)
   - Backs up start type and running state to:  
     `Desktop\\ThinkPadX1PowerOptimize\\its-service-backup.json`
   - Stops it (`sc stop <service>`)
   - Disables it (`sc config <service> start= disabled`)

The Reset + restore Lenovo ITS action:

1. Restores default Windows power schemes (`powercfg /restoredefaultschemes`)
2. Switches to the default Balanced plan
3. Restores the ITS service state using the backup file if available

## Power Stack & Contention (Concise)
Real-world hierarchy from low to high:

1. Hardware/microcode (uncontrollable): PL1/PL2, C-states, thermal caps.
2. ACPI/Firmware (read-only to OS): defines allowed policy space.
3. Windows Power Framework (effective layer): all GUIDs end up here.
4. Policy clients (writers): Windows PM, Intel DTT/IPF, Lenovo ITS/Vantage, this tool.

Key point:

- No lock, no owner, last writer wins.
- Therefore the app must act as a policy guardian, not a one-time config tool.

Current implementation:

- Scheduled tasks re-apply AC=DC on logon/resume/power change.
- A background watchdog keeps enforcing even after the app closes.

## Features

- List power plans: show all schemes and mark the active one; includes brief notes for common built-in GUIDs.
- Backup power plans (Admin required): exports `.pow` files to `Desktop\\ThinkPadX1PowerOptimize\\power-plans-backup-<timestamp>\\`.
- Smoother on AC (Admin required): makes plugged-in values match battery values (skips sleep subgroup); the output panel shows updated/failed/skipped counts.
- Smoother on AC + disable Lenovo ITS (Admin required): additionally disables Lenovo ITS Power Mode Control (restorable).
- Auto reapply (Admin required): re-applies on logon/resume/power change and starts a background watchdog to keep values consistent after the app closes.
- Reset + restore Lenovo ITS (Admin required): restores defaults and attempts to restore ITS service.

## Requirements

- Windows 10 / Windows 11 (x64)
- WebView2 Runtime (usually present on Windows 11; may require installation on some Windows 10 systems)
- Administrator privileges for actions that change power plan values or Windows services

## Data & Files

- No data is uploaded.
- Backups and ITS backups are written to: `Desktop\\ThinkPadX1PowerOptimize\\`
- Auto reapply scripts are written to: `%USERPROFILE%\\.Thinkpad_Power\\`

## Disclaimer
This is an experimental personal tool and may change Windows power plans and related service settings. You assume all risks and consequences from using this software (including but not limited to data loss, system instability, hardware damage, or any other losses). The author is not liable for any damages. It is recommended to back up your power plans before proceeding.
