import { useEffect, useRef, useState } from "react";
import {
  BookOpen,
  Bot,
  Gauge,
  Library,
  MonitorCog,
  Settings,
} from "lucide-react";
import "./App.css";
import { AutomationView } from "./features/automation/AutomationView";
import { Dashboard } from "./features/dashboard/Dashboard";
import { LibraryView } from "./features/library/LibraryView";
import { ReaderView } from "./features/reader/ReaderView";
import { SettingsView } from "./features/settings/SettingsView";
import {
  ensureMangaConRunning,
  getMangaConTaskStatus,
  importFavorites,
  loadImportedComics,
  queueMangaConUpdates,
  repairMangaConFailedTasks,
  resumeMangaConUnfinishedTasks,
  syncBookshelfMatches,
} from "./lib/api";
import { approvedDefaultPaths } from "./lib/defaults";
import type { EnsureMangaConRunningResult, MangaConFavorite } from "./lib/types";

type AppSection = "dashboard" | "library" | "automation" | "reader" | "settings";

const navigation: Array<{
  id: AppSection;
  label: string;
  icon: typeof Gauge;
}> = [
  { id: "dashboard", label: "仪表盘", icon: Gauge },
  { id: "library", label: "书库", icon: Library },
  { id: "automation", label: "自动化", icon: Bot },
  { id: "reader", label: "阅读器", icon: BookOpen },
  { id: "settings", label: "设置", icon: Settings },
];

const MANGACON_REFRESH_WAIT_MS = 30_000;
const REPAIR_MONITOR_INTERVAL_MS = 30_000;
const REPAIR_MONITOR_MAX_CHECKS = 120;
const REPAIR_MAX_TASKS = 200;

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function App() {
  const [activeSection, setActiveSection] = useState<AppSection>("dashboard");
  const [favorites, setFavorites] = useState<MangaConFavorite[]>([]);
  const [selectedReaderComic, setSelectedReaderComic] = useState<MangaConFavorite>();
  const [favoriteUpdateStartToken, setFavoriteUpdateStartToken] = useState<number>();
  const [importMessage, setImportMessage] = useState("尚未导入漫画控收藏");
  const [isImporting, setIsImporting] = useState(false);
  const [isScanningBookshelf, setIsScanningBookshelf] = useState(false);
  const [isUpdatingFavorites, setIsUpdatingFavorites] = useState(false);
  const [isRepairingFailedTasks, setIsRepairingFailedTasks] = useState(false);
  const [isResumingUnfinishedTasks, setIsResumingUnfinishedTasks] = useState(false);
  const [queuedUpdateCount, setQueuedUpdateCount] = useState(0);
  const mangaConReadyAtRef = useRef(0);
  const ensureMangaConPromiseRef =
    useRef<Promise<EnsureMangaConRunningResult> | null>(null);
  const repairMonitorTimerRef = useRef<number | undefined>(undefined);
  const repairMonitorChecksRef = useRef(0);

  async function ensureMangaConReady() {
    if (!ensureMangaConPromiseRef.current) {
      ensureMangaConPromiseRef.current = ensureMangaConRunning({
        executablePath: approvedDefaultPaths.mangaConExecutable,
      }).finally(() => {
        ensureMangaConPromiseRef.current = null;
      });
    }

    const result = await ensureMangaConPromiseRef.current;
    if (result.launched) {
      mangaConReadyAtRef.current = Date.now() + MANGACON_REFRESH_WAIT_MS;
    } else if (mangaConReadyAtRef.current === 0) {
      mangaConReadyAtRef.current = Date.now();
    }
    return result;
  }

  useEffect(() => {
    let cancelled = false;

    loadImportedComics({
      databasePath: approvedDefaultPaths.databasePath,
    })
      .then((loadedFavorites) => {
        if (cancelled || loadedFavorites.length === 0) {
          return;
        }
        setFavorites(loadedFavorites);
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }
        setImportMessage(cause instanceof Error ? cause.message : String(cause));
      });

    ensureMangaConReady()
      .then((result) => {
        if (cancelled) {
          return;
        }
        setImportMessage(
          result.launched
            ? "漫画控已启动，正在检索收藏更新..."
            : "漫画控已运行，等待本体检索收藏更新...",
        );
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }
        setImportMessage(cause instanceof Error ? cause.message : String(cause));
      });

    return () => {
      cancelled = true;
      if (repairMonitorTimerRef.current !== undefined) {
        window.clearTimeout(repairMonitorTimerRef.current);
      }
    };
  }, []);

  async function handleImportFavorites() {
    setIsImporting(true);
    setImportMessage("正在导入漫画控收藏...");
    try {
      const summary = await importFavorites({
        favoritesJsonPath: approvedDefaultPaths.mangaConFavoritesJson,
        databasePath: approvedDefaultPaths.databasePath,
      });
      setFavorites(summary.favorites);
      setSelectedReaderComic((current) => {
        if (!current) {
          return undefined;
        }

        return summary.favorites.find((favorite) => favorite.id === current.id);
      });
      setImportMessage(
        `已导入 ${summary.imported} 条收藏，书架匹配稍后执行`,
      );
    } catch (cause) {
      setImportMessage(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsImporting(false);
    }
  }

  async function syncBookshelfLibrary() {
    setIsScanningBookshelf(true);
    setImportMessage("正在扫描本地书架并读取漫画控封面缓存...");
    try {
      const summary = await syncBookshelfMatches({
        bookshelfRoot: approvedDefaultPaths.bookshelfRoot,
        databasePath: approvedDefaultPaths.databasePath,
        mangaConDatabasePath: approvedDefaultPaths.mangaConDatabase,
      });
      setFavorites(summary.favorites);
      setSelectedReaderComic((current) => {
        if (!current) {
          return undefined;
        }

        return summary.favorites.find((favorite) => favorite.id === current.id);
      });
      setImportMessage(
        `书架扫描完成：收藏 ${summary.imported} 条，匹配 ${summary.matched} 条，缺失 ${summary.missing} 条，暂未匹配历史文件夹 ${summary.orphaned} 个`,
      );
    } catch (cause) {
      setImportMessage(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsScanningBookshelf(false);
    }
  }

  function handleReadFavorite(favorite: MangaConFavorite) {
    setSelectedReaderComic(favorite);
    setActiveSection("reader");
  }

  async function handleUpdateFavorites() {
    setIsUpdatingFavorites(true);
    setImportMessage("正在确认漫画控本体运行...");
    try {
      await ensureMangaConReady();
      const waitMs = Math.max(0, mangaConReadyAtRef.current - Date.now());
      if (waitMs > 0) {
        setImportMessage(
          `漫画控本体正在检索收藏更新，约 ${Math.ceil(waitMs / 1000)} 秒后写入队列...`,
        );
        await wait(waitMs);
      }

      setImportMessage("正在写入漫画控数据库队列...");
      const result = await queueMangaConUpdates({
        mangaConDatabasePath: approvedDefaultPaths.mangaConDatabase,
        executablePath: approvedDefaultPaths.mangaConExecutable,
        companionDatabasePath: approvedDefaultPaths.databasePath,
        maxUpdates: 500,
      });
      setQueuedUpdateCount(result.queued);
      setImportMessage(
        result.queued > 0
          ? `已加入漫画控下载队列 ${result.queued} 话，跳过已有任务 ${result.skippedExisting} 话，清理更新标记 ${result.clearedUpdateMarkers} 处`
          : `没有新的待加入任务，跳过已有任务 ${result.skippedExisting} 话，清理更新标记 ${result.clearedUpdateMarkers} 处`,
      );
      if (result.queued > 0 || result.skippedExisting > 0) {
        startRepairMonitor();
      }
    } catch (cause) {
      setImportMessage(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsUpdatingFavorites(false);
    }
  }

  function startRepairMonitor(delayMs = REPAIR_MONITOR_INTERVAL_MS) {
    if (repairMonitorTimerRef.current !== undefined) {
      window.clearTimeout(repairMonitorTimerRef.current);
    }
    repairMonitorTimerRef.current = window.setTimeout(() => {
      repairMonitorTimerRef.current = undefined;
      void monitorAndRepairFailedTasks();
    }, delayMs);
  }

  async function monitorAndRepairFailedTasks() {
    repairMonitorChecksRef.current += 1;
    try {
      const status = await getMangaConTaskStatus({
        mangaConDatabasePath: approvedDefaultPaths.mangaConDatabase,
      });
      if (status.activeTasks > 0) {
        if (repairMonitorChecksRef.current < REPAIR_MONITOR_MAX_CHECKS) {
          setImportMessage(
            `漫画控仍有 ${status.activeTasks} 个下载任务，完成后将自动检查失败图片`,
          );
          startRepairMonitor();
        }
        return;
      }

      repairMonitorChecksRef.current = 0;
      if (status.failedTasks > 0) {
        await repairFailedTasks("auto");
      } else {
        setImportMessage("本轮下载完成，没有失败图片");
      }
    } catch (cause) {
      setImportMessage(
        `自动检查失败图片失败：${cause instanceof Error ? cause.message : String(cause)}`,
      );
    }
  }

  async function handleRepairFailedTasks() {
    await repairFailedTasks("manual");
  }

  async function handleResumeUnfinishedTasks() {
    setIsResumingUnfinishedTasks(true);
    setImportMessage("正在唤醒漫画控已有未完成下载任务...");
    try {
      const result = await resumeMangaConUnfinishedTasks({
        mangaConDatabasePath: approvedDefaultPaths.mangaConDatabase,
        executablePath: approvedDefaultPaths.mangaConExecutable,
      });
      setQueuedUpdateCount(result.totalUnfinished);
      if (result.resumeConfigured) {
        setImportMessage(
          `已唤醒漫画控继续 ${result.totalUnfinished} 个未完成下载任务`,
        );
        startRepairMonitor();
      } else {
        setImportMessage("漫画控当前没有未完成下载任务");
      }
    } catch (cause) {
      setImportMessage(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsResumingUnfinishedTasks(false);
    }
  }

  async function repairFailedTasks(mode: "manual" | "auto") {
    setIsRepairingFailedTasks(true);
    setImportMessage(
      mode === "manual" ? "正在扫描失败图片..." : "检测到失败图片，正在重新加入修复队列...",
    );
    try {
      const result = await repairMangaConFailedTasks({
        mangaConDatabasePath: approvedDefaultPaths.mangaConDatabase,
        executablePath: approvedDefaultPaths.mangaConExecutable,
        maxTasks: REPAIR_MAX_TASKS,
      });
      setQueuedUpdateCount(result.requeued);
      if (result.requeued > 0) {
        setImportMessage(
          `已将 ${result.requeued} 个失败任务重新加入漫画控修复队列`,
        );
        startRepairMonitor();
      } else if (result.totalFailed > 0) {
        setImportMessage(
          `检测到 ${result.totalFailed} 个失败任务，但本轮没有重新入队`,
        );
      } else {
        setImportMessage("没有需要修复的失败图片");
      }
    } catch (cause) {
      setImportMessage(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsRepairingFailedTasks(false);
    }
  }

  return (
    <main className="app-shell">
      <aside className="sidebar" aria-label="主导航">
        <div className="brand-block">
          <div className="brand-icon">
            <MonitorCog size={22} aria-hidden="true" />
          </div>
          <div>
            <strong>漫画控伴侣</strong>
            <span>Windows 本地工具</span>
          </div>
        </div>

        <nav className="nav-list">
          {navigation.map(({ id, label, icon: Icon }) => (
            <button
              className={activeSection === id ? "nav-item active" : "nav-item"}
              type="button"
              key={id}
              onClick={() => setActiveSection(id)}
            >
              <Icon size={18} aria-hidden="true" />
              <span>{label}</span>
            </button>
          ))}
        </nav>

        <div className="sidebar-footer">
          <span>状态</span>
          <strong>{importMessage}</strong>
        </div>
      </aside>

      <section className="content-shell">
        {activeSection === "dashboard" && (
          <Dashboard
            paths={approvedDefaultPaths}
            favorites={favorites}
            pendingTasks={queuedUpdateCount}
            importMessage={importMessage}
            isImporting={isImporting}
            isScanning={isScanningBookshelf}
            isUpdating={isUpdatingFavorites}
            isRepairing={isRepairingFailedTasks}
            isResuming={isResumingUnfinishedTasks}
            onImportFavorites={handleImportFavorites}
            onScanBookshelf={syncBookshelfLibrary}
            onUpdateFavorites={handleUpdateFavorites}
            onRepairFailedTasks={handleRepairFailedTasks}
            onResumeUnfinishedTasks={handleResumeUnfinishedTasks}
          />
        )}
        {activeSection === "library" && (
          <LibraryView favorites={favorites} onReadFavorite={handleReadFavorite} />
        )}
        {activeSection === "automation" && (
          <AutomationView
            autoStartRecoveryToken={favoriteUpdateStartToken}
            onAutoStartRecoveryHandled={() => setFavoriteUpdateStartToken(undefined)}
          />
        )}
        {activeSection === "reader" && <ReaderView comic={selectedReaderComic} />}
        {activeSection === "settings" && <SettingsView paths={approvedDefaultPaths} />}
      </section>
    </main>
  );
}

export default App;
