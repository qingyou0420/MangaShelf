import { useState } from "react";
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
import { importFavorites, queueMangaConUpdates } from "./lib/api";
import { approvedDefaultPaths } from "./lib/defaults";
import type { MangaConFavorite } from "./lib/types";

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

function App() {
  const [activeSection, setActiveSection] = useState<AppSection>("dashboard");
  const [favorites, setFavorites] = useState<MangaConFavorite[]>([]);
  const [selectedReaderComic, setSelectedReaderComic] = useState<MangaConFavorite>();
  const [favoriteUpdateStartToken, setFavoriteUpdateStartToken] = useState<number>();
  const [importMessage, setImportMessage] = useState("尚未导入漫画控收藏");
  const [isImporting, setIsImporting] = useState(false);
  const [isUpdatingFavorites, setIsUpdatingFavorites] = useState(false);
  const [queuedUpdateCount, setQueuedUpdateCount] = useState(0);

  async function handleImportFavorites() {
    setIsImporting(true);
    setImportMessage("正在导入漫画控收藏并扫描书架...");
    try {
      const summary = await importFavorites({
        favoritesJsonPath: approvedDefaultPaths.mangaConFavoritesJson,
        bookshelfRoot: approvedDefaultPaths.bookshelfRoot,
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
        `已导入 ${summary.imported} 条收藏，匹配 ${summary.matched} 本本地漫画`,
      );
    } catch (cause) {
      setImportMessage(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsImporting(false);
    }
  }

  function handleReadFavorite(favorite: MangaConFavorite) {
    setSelectedReaderComic(favorite);
    setActiveSection("reader");
  }

  async function handleUpdateFavorites() {
    setIsUpdatingFavorites(true);
    setImportMessage("正在写入漫画控数据库队列...");
    try {
      const result = await queueMangaConUpdates({
        mangaConDatabasePath: approvedDefaultPaths.mangaConDatabase,
        executablePath: approvedDefaultPaths.mangaConExecutable,
        maxUpdates: 500,
      });
      setQueuedUpdateCount(result.queued);
      setImportMessage(
        result.queued > 0
          ? `已加入漫画控下载队列 ${result.queued} 话，跳过已有任务 ${result.skippedExisting} 话`
          : `没有新的待加入任务，跳过已有任务 ${result.skippedExisting} 话`,
      );
    } catch (cause) {
      setImportMessage(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsUpdatingFavorites(false);
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
            isUpdating={isUpdatingFavorites}
            onImportFavorites={handleImportFavorites}
            onUpdateFavorites={handleUpdateFavorites}
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
