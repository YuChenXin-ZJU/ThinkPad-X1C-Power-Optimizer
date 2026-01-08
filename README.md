# ThinkPad X1C Power Optimizer

<p align="center">
  <a href="#中文"><img alt="中文" src="https://img.shields.io/badge/%E4%B8%AD%E6%96%87-Docs-1677ff?style=for-the-badge"></a>
  <a href="#english"><img alt="English" src="https://img.shields.io/badge/English-Docs-2ea44f?style=for-the-badge"></a>
  <a href="https://github.com/YuChenXin-ZJU/ThinkPad-X1C-Power-Optimizer/releases/latest"><img alt="Release" src="https://img.shields.io/badge/Release-Download-f97316?style=for-the-badge"></a>
</p>

---

# 中文

## 概述
面向 ThinkPad X1 Carbon 的 Windows 桌面工具（Tauri），用于查看、备份和校正电源计划。重点是降低插电或模式切换后的调度波动，同时提供可回退操作路径。

本工具使用 Windows 自带命令（`powercfg`、`sc`、`schtasks` 等）执行，不需要联网。

## 功能
- 查看电源计划：列出所有方案并标记当前活动方案。
- 备份电源计划（需管理员权限）：导出 `.pow`，便于回滚与迁移。
- 插电调度对齐电池（推荐，需管理员权限）：将当前方案的 AC 值对齐 DC 值（跳过睡眠子组）。
- 插电参数对齐电池 + 停用电源策略服务（高级，需管理员权限）：
  - 对齐当前方案的 AC=DC，并尝试停止/禁用 Intel DTT、Lenovo ITS、Vantage 等服务。
  - 不存在的服务会显示“未安装”，不再报错。
- 启用自动回写（推荐，需管理员权限）：登录/唤醒/电源切换后自动对齐，并启动后台守护。
- AC/DC 核心一致性检查：对比处理器子组（SUB_PROCESSOR）AC/DC 值是否一致并输出差异。
- 恢复默认电源计划（需管理员权限）：还原 Windows 默认方案并尝试恢复电源策略服务。

## 机制简述
- 电源计划最终落在 Windows Power Framework，多方写入者并存，最后写入者生效。
- 因此工具采用“对齐 + 守护”，而不是一次性设置。

## 数据与文件
- 备份目录：`Desktop\ThinkPadX1PowerOptimize\`
- 自动回写脚本：`%USERPROFILE%\.Thinkpad_Power\`
- 电源策略服务备份：`Desktop\ThinkPadX1PowerOptimize\power-policy-services-backup.json`

## 运行环境
- Windows 10 / Windows 11（x64）
- 需要 WebView2 Runtime
- 修改电源计划或服务需管理员权限

## 免责声明
本软件为个人实验性工具，可能修改系统电源计划与相关服务配置。使用本软件所产生的任何风险与后果（包括但不限于数据丢失、系统异常、硬件损坏或其他损失）均由使用者自行承担，作者不承担责任。建议操作前先备份电源计划。

---

# English

## Overview
A Windows desktop utility (Tauri) for ThinkPad X1 Carbon that provides power plan visibility, backup/rollback, alignment, and a guardian mechanism to keep settings consistent.

It uses built-in Windows commands (`powercfg`, `sc`, `schtasks`, etc.) and does not require network access.

## Features
- List power plans and mark the active scheme.
- Backup power plans (Admin required): export `.pow` files.
- Smoother on AC (recommended, Admin required): align AC values to DC (skips Sleep subgroup).
- Align AC to battery + disable power policy services (advanced, Admin required):
  - Stops/disables likely writers (e.g., Intel DTT / Lenovo ITS / Vantage).
  - Services not installed are reported as ?Not installed? instead of errors.
- Auto reapply (recommended, Admin required): reapply on logon/resume/power change and keep a watchdog running.
- AC/DC processor alignment check: compare SUB_PROCESSOR AC/DC values and list differences.
- Reset + restore services (Admin required): restore default schemes and attempt to restore services.

## How it works
- Power plan settings ultimately land in the Windows Power Framework; multiple policy writers coexist and the last writer wins.
- The tool focuses on alignment plus continuous enforcement instead of a one-time change.

## Data & files
- Backups: `Desktop\ThinkPadX1PowerOptimize\`
- Auto reapply scripts: `%USERPROFILE%\.Thinkpad_Power\`
- Service backup: `Desktop\ThinkPadX1PowerOptimize\power-policy-services-backup.json`

## Requirements
- Windows 10 / 11 (x64)
- WebView2 Runtime
- Administrator privileges for operations that modify power plans or services

## Disclaimer
This is an experimental personal tool and may change Windows power plans and related service settings. You assume all risks and consequences (including data loss, instability, or hardware damage). Please back up your power plans before proceeding.
