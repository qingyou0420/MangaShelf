# 漫画控伴侣

漫画控伴侣是一个只面向 Windows 桌面的本地工具，用来辅助漫画控完成收藏更新下载，并管理 `E:\书架` 中已经下载的本地漫画。

## V1.0 功能

- 导入漫画控收藏 JSON，只导入当前收藏源，不强行匹配历史散落文件夹。
- 匹配 `E:\书架` 下的本地漫画目录，显示章节数、图片页数和缺失状态。
- 自动识别漫画控收藏页、详情页红点，并通过漫画控自身下载更新章节。
- 支持长跑更新日志、实时事件推送、卡死重启恢复和失败记录面板。
- 支持从书库打开本地漫画，扫描章节并按自然页序阅读图片。

## 默认路径

- 漫画控程序：`E:\漫画控\MangaCon.exe`
- 漫画控收藏导出：`E:\漫画控\20260528184624.mc3db.json`
- 本地书架：`E:\书架`
- 本地数据库：`E:\书架\mangacon-companion.sqlite`

## 开发与自检

```powershell
npm install
npm test -- --run
npm run build
cargo test --manifest-path src-tauri\Cargo.toml
cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets --all-features -- -D warnings
```

## Windows 打包

```powershell
npm run tauri build
```

V1.0 安装包会输出到 `src-tauri\target\release\bundle\nsis\`。
