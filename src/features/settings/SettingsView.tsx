import { useEffect, useState } from "react";
import {
  checkLocalInstallerUpdates,
  clearLibraryCache,
  getAppVersion,
  libraryCacheStats,
  openLocalInstaller,
  openPath,
  pickDirectory,
} from "../../lib/api";
import {
  databasePathFor,
  loadReaderDefaults,
  loadTheme,
  saveReaderDefaults,
  saveTheme,
} from "../../lib/defaults";
import type {
  CacheStats,
  FitMode,
  LibraryPaths,
  LocalUpdateCheckResult,
  ReadMode,
  ReaderDefaults,
  ReadingDirection,
} from "../../lib/types";

interface SettingsViewProps {
  paths: LibraryPaths;
  onSavePaths?: (paths: LibraryPaths) => void;
  /** When provided, skip loading version from backend (tests / shell). */
  appVersion?: string;
  theme?: "light" | "dark";
  onThemeChange?: (theme: "light" | "dark") => void;
}

export function SettingsView({
  paths,
  onSavePaths,
  appVersion: appVersionProp,
  theme: themeProp,
  onThemeChange,
}: SettingsViewProps) {
  const [appVersion, setAppVersion] = useState(appVersionProp ?? "…");
  const [bookshelfRoot, setBookshelfRoot] = useState(paths.bookshelfRoot);
  const [databasePath, setDatabasePath] = useState(paths.databasePath);
  const [pathMessage, setPathMessage] = useState<string>();
  const [isCheckingUpdate, setIsCheckingUpdate] = useState(false);
  const [updateResult, setUpdateResult] = useState<LocalUpdateCheckResult>();
  const [updateError, setUpdateError] = useState<string>();
  const [isOpeningInstaller, setIsOpeningInstaller] = useState(false);
  const [openInstallerMessage, setOpenInstallerMessage] = useState<string>();
  const [isPicking, setIsPicking] = useState(false);
  const [readerDefaults, setReaderDefaults] = useState<ReaderDefaults>(
    loadReaderDefaults,
  );
  const [cacheStats, setCacheStats] = useState<CacheStats>();
  const [cacheMessage, setCacheMessage] = useState<string>();
  const [isClearingCache, setIsClearingCache] = useState(false);
  const [theme, setTheme] = useState<"light" | "dark">(themeProp ?? loadTheme);
  const [extraRoots, setExtraRoots] = useState<string[]>(
    paths.extraRoots ?? [],
  );

  useEffect(() => {
    setBookshelfRoot(paths.bookshelfRoot);
    setDatabasePath(paths.databasePath);
    setExtraRoots(paths.extraRoots ?? []);
  }, [paths.bookshelfRoot, paths.databasePath, paths.extraRoots]);

  useEffect(() => {
    if (themeProp) {
      setTheme(themeProp);
    }
  }, [themeProp]);

  useEffect(() => {
    if (appVersionProp !== undefined) {
      setAppVersion(appVersionProp);
      return;
    }
    let cancelled = false;
    getAppVersion()
      .then((version) => {
        if (!cancelled) {
          setAppVersion(version);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setAppVersion("未知");
        }
      });
    return () => {
      cancelled = true;
    };
  }, [appVersionProp]);

  function persistPaths(
    nextRoot: string,
    nextDatabase: string,
    nextExtras = extraRoots,
  ) {
    const next = {
      bookshelfRoot: nextRoot.trim(),
      databasePath: nextDatabase.trim() || databasePathFor(nextRoot),
      extraRoots: nextExtras,
    };
    setBookshelfRoot(next.bookshelfRoot);
    setDatabasePath(next.databasePath);
    onSavePaths?.(next);
    setPathMessage("已保存本地路径，下次扫描将使用新位置。");
  }

  async function handlePickBookshelf() {
    setIsPicking(true);
    try {
      const picked = await pickDirectory();
      if (!picked) {
        return;
      }
      persistPaths(picked, databasePathFor(picked));
    } catch (error) {
      setPathMessage(
        error instanceof Error ? error.message : String(error ?? "无法选择文件夹"),
      );
    } finally {
      setIsPicking(false);
    }
  }

  async function handleCheckUpdate() {
    setIsCheckingUpdate(true);
    setUpdateError(undefined);
    setOpenInstallerMessage(undefined);
    try {
      const result = await checkLocalInstallerUpdates();
      setUpdateResult(result);
    } catch (error) {
      setUpdateResult(undefined);
      setUpdateError(
        error instanceof Error ? error.message : String(error ?? "检查失败"),
      );
    } finally {
      setIsCheckingUpdate(false);
    }
  }

  async function handleOpenInstaller(path: string) {
    setIsOpeningInstaller(true);
    setOpenInstallerMessage(undefined);
    try {
      await openLocalInstaller(path);
      setOpenInstallerMessage("已开始下载并启动安装程序，请按向导完成更新。");
    } catch (error) {
      setOpenInstallerMessage(
        error instanceof Error ? error.message : String(error ?? "无法打开安装包"),
      );
    } finally {
      setIsOpeningInstaller(false);
    }
  }

  const latest = updateResult?.latest;

  return (
    <section className="view" aria-labelledby="settings-title">
      <div className="view-header">
        <h1 id="settings-title">设置</h1>
      </div>

      <section className="panel settings-panel" aria-label="本地书架">
        <label className="settings-kv path-kv">
          <span>书架</span>
          <input
            value={bookshelfRoot}
            onChange={(event) => setBookshelfRoot(event.target.value)}
            aria-label="书架路径"
          />
        </label>
        <div className="settings-update-actions">
          <button
            type="button"
            className="compact-action primary-action"
            onClick={() => void handlePickBookshelf()}
            disabled={isPicking}
          >
            {isPicking ? "选择中…" : "选择文件夹"}
          </button>
          <button
            type="button"
            className="compact-action"
            onClick={() => void openPath(bookshelfRoot)}
          >
            打开文件夹
          </button>
          <button
            type="button"
            className="compact-action"
            onClick={() => persistPaths(bookshelfRoot, databasePath)}
          >
            保存路径
          </button>
        </div>
        {pathMessage && (
          <p className="settings-update-msg" role="status">
            {pathMessage}
          </p>
        )}
        <details className="settings-advanced">
          <summary>高级：索引库路径</summary>
          <label className="settings-kv path-kv">
            <span>索引库</span>
            <input
              value={databasePath}
              onChange={(event) => setDatabasePath(event.target.value)}
              aria-label="索引库路径"
            />
          </label>
        </details>
        <p className="settings-hint">
          书库只读取你指定的本地文件夹。首次「导入现有书库」会索引全部已有文件夹，不会出现在最近更新。之后启动时和回到窗口时会自动扫描，只有新书文件夹和新话文件夹记为更新。扫描不会删除漫画文件。
        </p>
        <div className="settings-update-actions">
          <button
            type="button"
            className="compact-action"
            onClick={() => {
              void pickDirectory().then((picked) => {
                if (!picked || extraRoots.includes(picked) || picked === bookshelfRoot) {
                  return;
                }
                const next = [...extraRoots, picked];
                setExtraRoots(next);
                persistPaths(bookshelfRoot, databasePath, next);
              });
            }}
          >
            添加额外书架
          </button>
        </div>
        {extraRoots.length > 0 && (
          <ul className="settings-extra-roots">
            {extraRoots.map((root) => (
              <li key={root}>
                <code>{root}</code>
                <button
                  type="button"
                  className="linkish-button"
                  onClick={() => {
                    const next = extraRoots.filter((item) => item !== root);
                    setExtraRoots(next);
                    persistPaths(bookshelfRoot, databasePath, next);
                  }}
                >
                  移除
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="panel settings-panel" aria-label="阅读默认">
        <div className="settings-kv">
          <span>方向</span>
          <select
            aria-label="默认阅读方向"
            value={readerDefaults.readingDirection}
            onChange={(event) => {
              const next = {
                ...readerDefaults,
                readingDirection: event.target.value as ReadingDirection,
              };
              setReaderDefaults(saveReaderDefaults(next));
            }}
          >
            <option value="ltr">左开</option>
            <option value="rtl">右开</option>
          </select>
        </div>
        <div className="settings-kv">
          <span>适配</span>
          <select
            aria-label="默认页面适配"
            value={readerDefaults.fitMode}
            onChange={(event) => {
              const next = {
                ...readerDefaults,
                fitMode: event.target.value as FitMode,
              };
              setReaderDefaults(saveReaderDefaults(next));
            }}
          >
            <option value="contain">适应</option>
            <option value="width">宽度</option>
            <option value="height">高度</option>
            <option value="original">原图</option>
          </select>
        </div>
        <div className="settings-kv">
          <span>模式</span>
          <select
            aria-label="默认阅读模式"
            value={readerDefaults.readMode}
            onChange={(event) => {
              const next = {
                ...readerDefaults,
                readMode: event.target.value as ReadMode,
              };
              setReaderDefaults(saveReaderDefaults(next));
            }}
          >
            <option value="page">翻页</option>
            <option value="scroll">滚动</option>
            <option value="spread">双页</option>
          </select>
        </div>
        <p className="settings-hint">只作用于还没有阅读记录的新书。</p>
        <div className="settings-kv">
          <span>外观</span>
          <select
            aria-label="界面外观"
            value={theme}
            onChange={(event) => {
              const next = event.target.value === "dark" ? "dark" : "light";
              setTheme(saveTheme(next));
              document.documentElement.dataset.theme = next;
              onThemeChange?.(next);
            }}
          >
            <option value="light">浅色</option>
            <option value="dark">夜间</option>
          </select>
        </div>
      </section>

      <section className="panel settings-panel" aria-label="缓存">
        <div className="settings-update-actions">
          <button
            type="button"
            className="compact-action"
            onClick={() => {
              void libraryCacheStats({
                bookshelfRoot,
                extraRoots,
              })
                .then(setCacheStats)
                .catch((error) => {
                  setCacheMessage(
                    error instanceof Error ? error.message : String(error),
                  );
                });
            }}
          >
            查看缓存
          </button>
          <button
            type="button"
            className="compact-action"
            disabled={isClearingCache}
            onClick={() => {
              setIsClearingCache(true);
              void clearLibraryCache({
                bookshelfRoot,
                extraRoots,
              })
                .then((stats) => {
                  setCacheStats(stats);
                  setCacheMessage(
                    `已清理 ${formatBytes(stats.freedBytes)}`,
                  );
                })
                .catch((error) => {
                  setCacheMessage(
                    error instanceof Error ? error.message : String(error),
                  );
                })
                .finally(() => setIsClearingCache(false));
            }}
          >
            {isClearingCache ? "清理中…" : "清理缓存"}
          </button>
        </div>
        {cacheStats && (
          <p className="settings-hint">
            zip 缓存 {formatBytes(cacheStats.bytes)} · {cacheStats.folders} 个目录
          </p>
        )}
        {cacheMessage && (
          <p className="settings-update-msg" role="status">
            {cacheMessage}
          </p>
        )}
      </section>

      <section className="panel settings-panel" aria-label="关于">
        <div className="settings-kv">
          <span>版本</span>
          <strong data-testid="app-version">v{appVersion}</strong>
        </div>
        <div className="settings-update-row">
          <button
            type="button"
            className="compact-action"
            onClick={() => void handleCheckUpdate()}
            disabled={isCheckingUpdate}
          >
            {isCheckingUpdate ? "检查中…" : "检查更新"}
          </button>
          <p className="settings-hint">
            从 GitHub 仓库{" "}
            <code>qingyou0420/MangaShelf</code> 获取最新安装包
          </p>
        </div>
        {updateError && (
          <p className="settings-update-msg error" role="alert">
            {updateError}
          </p>
        )}
        {updateResult && !updateError && (
          <div className="settings-update-result" aria-live="polite">
            {updateResult.hasUpdate && latest ? (
              <>
                <div className="settings-kv">
                  <span>可更新</span>
                  <strong data-testid="update-latest-version">
                    v{latest.version}
                  </strong>
                </div>
                <div className="settings-kv path-kv">
                  <span>安装包</span>
                  <code title={latest.path}>{latest.fileName}</code>
                </div>
                <div className="settings-update-actions">
                  <button
                    type="button"
                    className="compact-action primary-action"
                    onClick={() => void handleOpenInstaller(latest.path)}
                    disabled={isOpeningInstaller}
                  >
                    {isOpeningInstaller ? "下载中…" : "下载并安装"}
                  </button>
                </div>
              </>
            ) : updateResult.packages.length > 0 ? (
              <p className="settings-update-msg" data-testid="update-status">
                已是最新（v{updateResult.currentVersion}）
              </p>
            ) : (
              <p className="settings-update-msg" data-testid="update-status">
                未找到更新包。发布后会显示在{" "}
                <code>github.com/qingyou0420/MangaShelf/releases</code>
              </p>
            )}
            {updateResult.searchedDirs.length > 0 && (
              <details className="settings-searched-dirs">
                <summary>已扫描目录（{updateResult.searchedDirs.length}）</summary>
                <ul>
                  {updateResult.searchedDirs.map((dir) => (
                    <li key={dir}>
                      <code>{dir}</code>
                    </li>
                  ))}
                </ul>
              </details>
            )}
          </div>
        )}
        {openInstallerMessage && (
          <p className="settings-update-msg" role="status">
            {openInstallerMessage}
          </p>
        )}
      </section>
    </section>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
