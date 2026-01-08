# ThinkPad X1 Carbon Gen 12 Power Optimizer

Tauri 桌面工具：用于查看/备份 Windows 电源计划，并提供“插电更流畅”等一键操作（需要管理员权限）。

## 开发

```bash
npm install
npm run tauri dev
```

## 构建

```bash
npm run tauri build -- --no-bundle
```

## 发布

推送 tag（例如 `v0.1.0`）后，GitHub Actions 会在 Windows 上自动构建并把 exe 附加到 Release。
