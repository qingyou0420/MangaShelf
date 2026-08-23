# 开发目录已迁移

- **新路径**: `D:\Grisia Studio\Manga Library`
- **旧路径**: `D:\Manga Library`（复制完成后可手动删除，若仍被占用请先关闭所有占用该目录的终端/IDE）

## 使用

```powershell
cd "D:\Grisia Studio\Manga Library"
npm install
npm test -- --run
npm run tauri dev
```

安装包产物仍在本仓库 `release\` 下；系统安装目录建议继续使用 `D:\Program Files\Manga Library`，与源码分离。
