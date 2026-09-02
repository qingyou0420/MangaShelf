# MangaShelf（漫画书架）

**MangaShelf** 是 Windows 本地漫画书库与阅读器。数据只来自你指定的本地文件夹，可完全离线阅读，不依赖任何在线漫画平台。

仓库：https://github.com/qingyou0420/MangaShelf

## 功能

- 书库：扫描本地目录、封面、标题/作者/标签、收藏、搜索与筛选、继续阅读、额外书架根；默认按最近更新排序，封面标出更新话数（仅基准导入之后的新书/新话文件夹）
- 阅读：翻页 / 滚动 / 双页、进度记忆、跨话、左开/右开、页面适配、作品页目录、全屏
- 设置：选择书架文件夹、夜间外观、阅读默认、zip 缓存、从 GitHub 检查并下载更新

## 默认路径

- 本地书架：`E:\书架`
- 索引库：`E:\书架\manga-library.sqlite`  
  若只有旧版 `mangacon-companion.sqlite`，启动时会复制一份为新库名，不覆盖旧文件。

## 支持的本地结构

- 书架根目录下每个文件夹是一部漫画
- 漫画文件夹内按子文件夹分话，或直接放图片（视为全一册）
- `zip` / `cbz` 可作为一部漫画或一话；阅读时解压到本地缓存，不改动原文件
- `rar` / `cbr`：已安装 7-Zip 或 WinRAR 时可以解压阅读；否则会提示缺少解压工具

扫描可重复执行且幂等：不会删除漫画文件；文件夹消失只会把书目标为「未匹配」。

首次完整扫描会把当前书架标为基准：已有的书和话只进入索引，不会标成更新。之后自动或手动扫描只把新出现的书文件夹、新出现的话/卷文件夹记为更新，并按最近更新排在主目录前面、在封面标出更新了几话；改文件、刷新封面或触碰旧文件夹不会刷屏。

## 版本与更新

- 版本号与 `package.json` / `src-tauri/Cargo.toml` / `src-tauri/tauri.conf.json` 保持一致（当前 `2.5.3`）。
- 启动时向 GitHub Releases 查询是否有更高版本。
- 发现更新时：侧栏版本号旁显示「更新 vX.Y.Z」；设置页可「检查更新」并下载安装。

发版方式：推送标签 `vX.Y.Z`（GitHub Actions 会打包），或把 `MangaShelf_*_x64-setup.exe` 上传到该标签的 Release。

## 安装路径建议

源码在 `D:\Grisia Studio\Manga Library` 时，**不要**装到同一目录，以免与开发文件混在一起。

| 目标路径 | 是否需要管理员 | 说明 |
|---------|----------------|------|
| `D:\Program Files\MangaShelf` | **需要** | 系统目录，安装包须「以管理员身份运行」 |
| `D:\Apps\MangaShelf` 等普通目录 | 一般不需要 | 推荐：与源码分离、无 UAC 摩擦 |
| `%LOCALAPPDATA%\MangaShelf` | 不需要 | 安装器「仅当前用户」默认位置 |

## 开发

```powershell
npm install
npm test -- --run
npm run build
cargo test --manifest-path src-tauri\Cargo.toml --lib
npm run tauri build
```

## 架构

```text
MangaShelf UI  ──invoke──► Rust 后端
                            ├─ 本地书库 SQLite 索引
                            ├─ 扫描文件夹 / zip / cbz
                            ├─ GitHub Release 更新
                            └─ 阅读进度与元数据（仅本地）
```
