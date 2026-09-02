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
import { Select } from "../../components/Select";
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
  onNotify?: (message: string) => void;
  /** When provided, skip loading version from backend (tests / shell). */
  appVersion?: string;
  theme?: "light" | "dark";
  onThemeChange?: (theme: "light" | "dark") => void;
}

export function SettingsView({
  paths,
  onSavePaths,
  onNotify,
  appVersion: appVersionProp,
  theme: themeProp,
  onThemeChange,
}: SettingsViewProps) {
  const [appVersion, setAppVersion] = useState(appVersionProp ?? "…");
  const [bookshelfRoot, setBookshelfRoot] = useState(paths.bookshelfRoot);
  const [databasePath, setDatabasePath] = useState(paths.databasePath);
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
    const sameRoot = next.bookshelfRoot === paths.bookshelfRoot;
    const sameDatabase = next.databasePath === paths.databasePath;
    const sameExtras =
      JSON.stringify(next.extraRoots) === JSON.stringify(paths.extraRoots ?? []);
    setBookshelfRoot(next.bookshelfRoot);
    setDatabasePath(next.databasePath);
    if (sameRoot && sameDatabase && sameExtras) {
      return;
    }
    onSavePaths?.(next);
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
      onNotify?.(
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

      <section className="panel settings-panel" aria-label="书架">
        <h2>书架</h2>
        <label className="settings-kv path-kv">
          <span>书架</span>
          <input
            value={bookshelfRoot}
            onChange={(event) => setBookshelfRoot(event.target.value)}
            onBlur={() => persistPaths(bookshelfRoot, databasePath)}
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
        </div>
        <details className="settings-advanced">
          <summary>高级：索引库路径</summary>
          <label className="settings-kv path-kv">
            <span>索引库</span>
            <input
              value={databasePath}
              onChange={(event) => setDatabasePath(event.target.value)}
              onBlur={() => persistPaths(bookshelfRoot, databasePath)}
              aria-label="索引库路径"
            />
          </label>
          <p className="settings-hint">
            首次导入只建立索引，不会标记更新。之后只有新书和新话会排在前面并在封面标出更新。
          </p>
        </details>
        <p className="settings-hint">仅读取本地文件夹，不会修改或删除文件。</p>
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

      <section className="panel settings-panel" aria-label="阅读">
        <h2>阅读</h2>
        <div className="settings-kv">
          <span>方向</span>
          <Select
            label="默认阅读方向"
            value={readerDefaults.readingDirection}
            options={[
              { value: "ltr", label: "左开" },
              { value: "rtl", label: "右开" },
            ]}
            onChange={(readingDirection: ReadingDirection) => {
              setReaderDefaults(
                saveReaderDefaults({ ...readerDefaults, readingDirection }),
              );
            }}
          />
        </div>
        <div className="settings-kv">
          <span>适配</span>
          <Select
            label="默认页面适配"
            value={readerDefaults.fitMode}
            options={[
              { value: "contain", label: "适应" },
              { value: "width", label: "宽度" },
              { value: "height", label: "高度" },
              { value: "original", label: "原图" },
            ]}
            onChange={(fitMode: FitMode) => {
              setReaderDefaults(saveReaderDefaults({ ...readerDefaults, fitMode }));
            }}
          />
        </div>
        <div className="settings-kv">
          <span>模式</span>
          <Select
            label="默认阅读模式"
            value={readerDefaults.readMode}
            options={[
              { value: "page", label: "翻页" },
              { value: "scroll", label: "滚动" },
              { value: "spread", label: "双页" },
            ]}
            onChange={(readMode: ReadMode) => {
              setReaderDefaults(saveReaderDefaults({ ...readerDefaults, readMode }));
            }}
          />
        </div>
        <p className="settings-hint">只作用于还没有阅读记录的新书。</p>
      </section>

      <section className="panel settings-panel" aria-label="外观与缓存">
        <h2>外观与缓存</h2>
        <div className="settings-kv">
          <span>外观</span>
          <Select
            label="界面外观"
            value={theme}
            options={[
              { value: "light", label: "浅色" },
              { value: "dark", label: "夜间" },
            ]}
            onChange={(next) => {
              const resolved = next === "dark" ? "dark" : "light";
              setTheme(saveTheme(resolved));
              document.documentElement.dataset.theme = resolved;
              onThemeChange?.(resolved);
            }}
          />
        </div>
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
        <h2>关于</h2>
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
                未找到更新包
              </p>
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
