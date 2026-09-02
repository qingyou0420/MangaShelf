import { useEffect, useState } from "react";
import { Library, LibraryBig, Settings } from "lucide-react";
import "./App.css";
import { LibraryView } from "./features/library/LibraryView";
import { SeriesView } from "./features/library/SeriesView";
import { ReaderView } from "./features/reader/ReaderView";
import { SettingsView } from "./features/settings/SettingsView";
import { useAppUpdate } from "./hooks/useAppUpdate";
import { useConfirm } from "./hooks/useConfirm";
import { useLibrarySession } from "./hooks/useLibrarySession";
import { useToast } from "./hooks/useToast";
import { listCoverCandidates, openPath, setComicCover } from "./lib/api";
import { cacheRootForPath, loadTheme, saveTheme } from "./lib/defaults";

type AppSection = "library" | "settings";

const navigation: Array<{
  id: AppSection;
  label: string;
  icon: typeof Library;
}> = [
  { id: "library", label: "书库", icon: Library },
  { id: "settings", label: "设置", icon: Settings },
];

function App() {
  const [activeSection, setActiveSection] = useState<AppSection>("library");
  const [theme, setTheme] = useState<"light" | "dark">(loadTheme);
  const { toastMessage, showToast } = useToast();
  const { confirm, dialog } = useConfirm();
  const {
    appVersion,
    availableAppUpdate,
    isOpeningAppInstaller,
    installAppUpdate,
  } = useAppUpdate();
  const {
    paths,
    comics,
    seriesComic,
    selectedComic,
    readerChapterId,
    seriesChapters,
    seriesChaptersMessage,
    statusMessage,
    isScanning,
    scanProgress,
    failedItems,
    bookshelfMissing,
    baselineCompleted,
    readerService,
    handleScanLibrary,
    handleCancelScan,
    handleToggleFavorite,
    handleSaveMetadata,
    handleOpenSeries,
    handleRescan,
    handleReadComic,
    handleCloseReader,
    handleReaderPrefs,
    applyPaths,
    handleDeleteComic,
    handlePickBookshelf,
    handleClearProgress,
    setSeriesComic,
    setStatusMessage,
    replaceComic,
  } = useLibrarySession(showToast, confirm);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  const isReading = Boolean(selectedComic?.localPath);
  const favoriteCount = comics.filter((comic) => comic.favorited).length;

  async function handleInstallAppUpdate() {
    try {
      const message = await installAppUpdate();
      if (!message || !availableAppUpdate) {
        return;
      }
      showToast(message);
      setStatusMessage(
        `正在安装 v${availableAppUpdate.version}（${availableAppUpdate.fileName}）`,
      );
    } catch (cause) {
      const message =
        cause instanceof Error ? cause.message : String(cause ?? "无法打开安装包");
      showToast(message);
      setStatusMessage(message);
    }
  }

  return (
    <main
      className={[
        isReading ? "app-shell reading-mode" : "app-shell",
        theme === "dark" ? "theme-dark" : "",
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {toastMessage && (
        <div className="app-toast" role="status" aria-live="polite">
          {toastMessage}
        </div>
      )}
      {dialog}
      {!isReading && (
        <aside className="sidebar" aria-label="主导航">
          <div className="brand-block">
            <div className="brand-icon">
              <LibraryBig size={20} aria-hidden="true" />
            </div>
            <div className="brand-text">
              <strong>MangaShelf</strong>
              <span className="brand-sub">漫画书架</span>
              <div className="brand-version-row">
                {appVersion && (
                  <span
                    className="brand-version"
                    data-testid="sidebar-app-version"
                  >
                    v{appVersion}
                  </span>
                )}
                {availableAppUpdate && (
                  <button
                    type="button"
                    className="sidebar-update-btn"
                    data-testid="sidebar-app-update"
                    disabled={isOpeningAppInstaller}
                    title={`发现云端版本 v${availableAppUpdate.version}，点击下载安装`}
                    onClick={() => void handleInstallAppUpdate()}
                  >
                    {isOpeningAppInstaller
                      ? "…"
                      : `更新 v${availableAppUpdate.version}`}
                  </button>
                )}
              </div>
            </div>
          </div>

          <nav className="nav-list">
            {navigation.map(({ id, label, icon: Icon }) => (
              <button
                className={
                  activeSection === id && !seriesComic ? "nav-item active" : "nav-item"
                }
                type="button"
                key={id}
                onClick={() => {
                  setSeriesComic(undefined);
                  setActiveSection(id);
                }}
              >
                <Icon size={17} aria-hidden="true" />
                <span>{label}</span>
              </button>
            ))}
          </nav>

          <div className="sidebar-stats" aria-label="摘要">
            <div>
              <span>书库</span>
              <strong>{comics.length}</strong>
            </div>
            <div>
              <span>收藏</span>
              <strong>{favoriteCount}</strong>
            </div>
          </div>

          <div className="sidebar-footer" role="status" aria-live="polite">
            {statusMessage}
          </div>
        </aside>
      )}

      <section className={isReading ? "content-shell reader-shell-host" : "content-shell"}>
        {isReading ? (
          <ReaderView
            comic={selectedComic}
            initialChapterId={readerChapterId}
            onBack={() => {
              handleCloseReader();
              setActiveSection("library");
            }}
            onComicChange={(comic) => void handleReaderPrefs(comic)}
            onToggleFavorite={(comic) => void handleToggleFavorite(comic)}
            service={readerService}
          />
        ) : seriesComic && activeSection === "library" ? (
          <SeriesView
            comic={seriesComic}
            chapters={seriesChapters}
            chaptersMessage={seriesChaptersMessage}
            onBack={() => setSeriesComic(undefined)}
            onRead={(comic, chapter) => handleReadComic(comic, "series", chapter)}
            onToggleFavorite={(comic) => void handleToggleFavorite(comic)}
            onSaveMetadata={(comic, draft) => void handleSaveMetadata(comic, draft)}
            onOpenFolder={(path) => {
              void openPath(path).catch((cause) => {
                showToast(cause instanceof Error ? cause.message : String(cause));
              });
            }}
            onRescan={(comic) => handleRescan(comic)}
            onListCovers={(comic) => listCoverCandidates(comic.localPath ?? "")}
            onClearProgress={(comic) => void handleClearProgress(comic)}
            onSetCover={(comic, sourcePath) => {
              void setComicCover({
                bookshelfRoot: cacheRootForPath(
                  comic.localPath ?? paths.bookshelfRoot,
                  paths,
                ),
                databasePath: paths.databasePath,
                comicId: comic.id,
                sourcePath,
              }).then((next) => {
                if (next) {
                  replaceComic(next);
                }
              });
            }}
          />
        ) : (
          <>
            {activeSection === "library" && (
              <LibraryView
                comics={comics}
                onOpenSeries={(comic) => {
                  handleOpenSeries(comic);
                  setActiveSection("library");
                }}
                onReadComic={(comic) => handleReadComic(comic, "library")}
                onScanLibrary={() => void handleScanLibrary()}
                onCancelScan={() => void handleCancelScan()}
                onToggleFavorite={(comic) => void handleToggleFavorite(comic)}
                onSaveMetadata={(comic, draft) =>
                  void handleSaveMetadata(comic, draft)
                }
                onPickBookshelf={() => void handlePickBookshelf()}
                onDeleteComic={(comic) => void handleDeleteComic(comic)}
                isScanning={isScanning}
                scanProgress={scanProgress}
                failedItems={failedItems}
                bookshelfMissing={bookshelfMissing}
                baselineCompleted={baselineCompleted}
              />
            )}
            {activeSection === "settings" && (
              <SettingsView
                paths={paths}
                onSavePaths={applyPaths}
                onNotify={showToast}
                appVersion={appVersion}
                theme={theme}
                onThemeChange={(next) => setTheme(saveTheme(next))}
              />
            )}
          </>
        )}
      </section>
    </main>
  );
}

export default App;
