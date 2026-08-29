import { useEffect, useMemo, useRef, useState } from "react";
import {
  allowAssetRoot,
  cancelLibraryScan,
  clearReadProgress,
  deleteLibraryComic,
  listChapterPages,
  listenScanProgress,
  loadLibrary,
  pathIsDirectory,
  peekChapterFirstPage,
  pickDirectory,
  saveReadProgress,
  scanLibrary,
  scanLocalChapters,
  setComicFavorite,
  setReaderPrefs,
  updateComicMetadata,
} from "../lib/api";
import {
  cacheRootForPath,
  databasePathFor,
  loadStoredPaths,
  saveStoredPaths,
} from "../lib/defaults";
import type {
  LibraryComic,
  LibraryPaths,
  LocalChapter,
  ScanProgress,
} from "../lib/types";
import type { ReaderService } from "../features/reader/ReaderView";

export type ReaderReturn = "library" | "series";

function mergePaths(
  current: LibraryPaths,
  patch: Partial<LibraryPaths>,
): LibraryPaths {
  return {
    bookshelfRoot: patch.bookshelfRoot || current.bookshelfRoot,
    databasePath: patch.databasePath || current.databasePath,
    extraRoots: patch.extraRoots ?? current.extraRoots ?? [],
  };
}

function allowRoots(paths: LibraryPaths) {
  void allowAssetRoot(paths.bookshelfRoot).catch(() => undefined);
  for (const extra of paths.extraRoots ?? []) {
    void allowAssetRoot(extra).catch(() => undefined);
  }
}

export function useLibrarySession(showToast: (message: string) => void) {
  const [paths, setPaths] = useState<LibraryPaths>(loadStoredPaths);
  const [comics, setComics] = useState<LibraryComic[]>([]);
  const [seriesComic, setSeriesComic] = useState<LibraryComic>();
  const [selectedComic, setSelectedComic] = useState<LibraryComic>();
  const [readerChapterId, setReaderChapterId] = useState<string | null>(null);
  const [readerReturn, setReaderReturn] = useState<ReaderReturn>("library");
  const [seriesChapters, setSeriesChapters] = useState<LocalChapter[]>([]);
  const [seriesChaptersMessage, setSeriesChaptersMessage] = useState("加载中…");
  const [statusMessage, setStatusMessage] = useState("正在准备漫画书架…");
  const [isScanning, setIsScanning] = useState(false);
  const [scanProgress, setScanProgress] = useState<ScanProgress>();
  const [failedItems, setFailedItems] = useState<
    Array<{ title: string; error: string }>
  >([]);
  const [bookshelfMissing, setBookshelfMissing] = useState(false);
  const [baselineCompleted, setBaselineCompleted] = useState(false);
  const scanCancelledRef = useRef(false);
  const scanningRef = useRef(false);
  const lastScanAtRef = useRef(0);

  function replaceComic(next: LibraryComic) {
    setComics((current) =>
      current.map((comic) => (comic.id === next.id ? next : comic)),
    );
    setSelectedComic((current) =>
      current && current.id === next.id ? { ...current, ...next } : current,
    );
    setSeriesComic((current) =>
      current && current.id === next.id ? { ...current, ...next } : current,
    );
  }

  const readerService: ReaderService = useMemo(
    () => ({
      scanChapters: (comicId: string, comicDirectory: string, force?: boolean) =>
        scanLocalChapters({
          comicId,
          comicDirectory,
          databasePath: paths.databasePath,
          force,
        }),
      listPages: (chapterPath: string) =>
        listChapterPages({
          chapterPath,
          bookshelfRoot: cacheRootForPath(chapterPath, paths),
        }),
      peekFirstPage: async (chapterPath: string) =>
        (await peekChapterFirstPage(chapterPath)) ?? undefined,
      saveProgress: async (comicId: string, chapterId: string, page: number) => {
        const next = await saveReadProgress({
          databasePath: paths.databasePath,
          comicId,
          chapterId,
          page,
        });
        if (!next) {
          return;
        }
        setComics((current) =>
          current.map((comic) => (comic.id === next.id ? next : comic)),
        );
        setSelectedComic((current) => {
          if (!current || current.id !== next.id) {
            return current;
          }
          if (
            current.lastReadChapterId === next.lastReadChapterId &&
            current.lastReadChapterTitle === next.lastReadChapterTitle &&
            current.lastReadChapterOrdinal === next.lastReadChapterOrdinal &&
            current.readProgressPage === next.readProgressPage &&
            current.lastReadAt === next.lastReadAt
          ) {
            return current;
          }
          return {
            ...current,
            readProgressPage: next.readProgressPage,
            lastReadChapterId: next.lastReadChapterId,
            lastReadChapterTitle: next.lastReadChapterTitle,
            lastReadChapterOrdinal: next.lastReadChapterOrdinal,
            lastReadChapterPages: next.lastReadChapterPages,
            lastReadAt: next.lastReadAt,
          };
        });
        setSeriesComic((current) =>
          current && current.id === next.id ? { ...current, ...next } : current,
        );
      },
    }),
    [paths.databasePath, paths.bookshelfRoot, paths.extraRoots],
  );

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    let dropUnlisten: (() => void) | undefined;
    const initialPaths = loadStoredPaths();

    void listenScanProgress((progress) => {
      if (!cancelled) {
        setScanProgress(progress);
      }
    }).then((fn) => {
      unlisten = fn;
    });

    allowRoots(initialPaths);

    void pathIsDirectory(initialPaths.bookshelfRoot)
      .then((isDir) => {
        if (!cancelled) {
          setBookshelfMissing(!isDir);
        }
      })
      .catch(() => undefined);

    void import("@tauri-apps/api/webview")
      .then(({ getCurrentWebview }) =>
        getCurrentWebview().onDragDropEvent((event) => {
          if (event.payload.type !== "drop") {
            return;
          }
          const dropped = event.payload.paths[0];
          if (!dropped) {
            return;
          }
          void pathIsDirectory(dropped).then((isDir) => {
            if (!isDir) {
              showToast("请拖入文件夹作为书架根目录");
              return;
            }
            if (window.confirm(`将书架设为：\n${dropped}`)) {
              applyPaths({
                bookshelfRoot: dropped,
                databasePath: databasePathFor(dropped),
                extraRoots: loadStoredPaths().extraRoots,
              });
            }
          });
        }),
      )
      .then((unlistenDrop) => {
        dropUnlisten = unlistenDrop;
      })
      .catch(() => undefined);

    loadLibrary(initialPaths)
      .then(async (result) => {
        if (cancelled) {
          return;
        }
        setComics(result.comics);
        setBaselineCompleted(Boolean(result.baselineCompleted));
        const nextPaths = mergePaths(initialPaths, {
          bookshelfRoot: result.bookshelfRoot,
          databasePath: result.databasePath,
        });
        setPaths(nextPaths);
        setStatusMessage(
          result.comics.length > 0
            ? `已载入 ${result.comics.length} 部本地漫画`
            : result.baselineCompleted
              ? "书库为空，扫描本地书架即可开始"
              : "书库为空，先导入现有书库作为基准",
        );
        const exists = await pathIsDirectory(nextPaths.bookshelfRoot);
        if (cancelled || !exists) {
          return;
        }
        await runScan(nextPaths, true);
      })
      .catch((cause) => {
        if (cancelled) {
          return;
        }
        setStatusMessage(cause instanceof Error ? cause.message : String(cause));
      });

    function onVisibility() {
      if (document.visibilityState !== "visible") {
        return;
      }
      if (Date.now() - lastScanAtRef.current < 120_000) {
        return;
      }
      const stored = loadStoredPaths();
      void pathIsDirectory(stored.bookshelfRoot).then((exists) => {
        if (exists) {
          void runScan(stored, true);
        }
      });
    }
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      cancelled = true;
      unlisten?.();
      dropUnlisten?.();
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [showToast]);

  async function runScan(scanPaths: LibraryPaths, quiet = false) {
    if (scanningRef.current) {
      return;
    }
    scanningRef.current = true;
    setIsScanning(true);
    scanCancelledRef.current = false;
    setScanProgress({ scanned: 0, total: 0, currentTitle: "" });
    if (!quiet) {
      setStatusMessage("正在扫描本地书架…");
    }
    try {
      const result = await scanLibrary(scanPaths);
      lastScanAtRef.current = Date.now();
      setComics(result.comics);
      setBaselineCompleted(Boolean(result.baselineCompleted));
      setPaths((current) =>
        mergePaths(current, {
          bookshelfRoot: result.bookshelfRoot,
          databasePath: result.databasePath,
        }),
      );
      setSelectedComic((current) => {
        if (!current) {
          return undefined;
        }
        return result.comics.find((comic) => comic.id === current.id);
      });
      setSeriesComic((current) => {
        if (!current) {
          return undefined;
        }
        return result.comics.find((comic) => comic.id === current.id);
      });
      const failed = result.failed ? `，失败 ${result.failed}` : "";
      const failedHint = result.failedItems?.[0]
        ? `（${result.failedItems[0].title}: ${result.failedItems[0].error}）`
        : "";
      setFailedItems(result.failedItems ?? []);
      const changed = result.added + result.updated > 0;
      const summary = scanCancelledRef.current
        ? `扫描已停止：新增 ${result.added}，有变化 ${result.updated}`
        : result.establishedBaseline
          ? result.added > 0
            ? `已导入现有书库 ${result.added} 部，已作为基准。它们不会标成更新。`
            : "已建立书库基准。之后新增的书和话会排在前面，并在封面标出更新话数。"
          : quiet && changed
            ? `书架有更新：新书 ${result.added}，新内容 ${result.updated}${failed}${failedHint}`
            : `扫描完成：新增 ${result.added}，有变化 ${result.updated}，未变 ${result.unchanged ?? 0}，未匹配 ${result.missing}${failed}${failedHint}`;
      if (!quiet || changed || result.establishedBaseline || scanCancelledRef.current) {
        setStatusMessage(summary);
        showToast(summary);
      }
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      setStatusMessage(message);
      if (!quiet) {
        showToast(message);
      }
    } finally {
      scanningRef.current = false;
      setIsScanning(false);
      setScanProgress(undefined);
    }
  }

  async function handleScanLibrary() {
    await runScan(paths, false);
  }

  async function handleCancelScan() {
    scanCancelledRef.current = true;
    try {
      await cancelLibraryScan();
    } catch {
      // Best-effort; the scan loop also stops when the command returns.
    }
  }

  async function handleToggleFavorite(comic: LibraryComic) {
    try {
      const next = await setComicFavorite({
        databasePath: paths.databasePath,
        comicId: comic.id,
        favorited: !comic.favorited,
      });
      if (next) {
        replaceComic(next);
      }
    } catch (cause) {
      showToast(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleSaveMetadata(
    comic: LibraryComic,
    draft: { name: string; author: string; tags: string },
  ) {
    try {
      const tags = draft.tags
        .split(/[,，;；]/)
        .map((tag) => tag.trim())
        .filter(Boolean);
      const next = await updateComicMetadata({
        databasePath: paths.databasePath,
        comicId: comic.id,
        name: draft.name.trim() || comic.name,
        author: draft.author,
        tags,
      });
      if (next) {
        replaceComic(next);
        showToast("已保存元数据");
      }
    } catch (cause) {
      showToast(cause instanceof Error ? cause.message : String(cause));
    }
  }

  function loadSeriesChapters(comic: LibraryComic) {
    if (!comic.localPath) {
      setSeriesChapters([]);
      setSeriesChaptersMessage("文件夹已不在");
      return;
    }
    setSeriesChapters([]);
    setSeriesChaptersMessage("加载中…");
    void scanLocalChapters({
      comicId: comic.id,
      comicDirectory: comic.localPath,
      databasePath: paths.databasePath,
      force: false,
    })
      .then((chapters) => {
        setSeriesChapters(chapters);
        setSeriesChaptersMessage(chapters.length > 0 ? "" : "无章节");
      })
      .catch((cause) => {
        setSeriesChapters([]);
        setSeriesChaptersMessage(
          cause instanceof Error ? cause.message : String(cause),
        );
      });
  }

  function handleOpenSeries(comic: LibraryComic) {
    setSeriesComic(comic);
    loadSeriesChapters(comic);
  }

  function handleRescan(comic: LibraryComic) {
    if (!comic.localPath) {
      return;
    }
    setSeriesChaptersMessage("加载中…");
    void scanLocalChapters({
      comicId: comic.id,
      comicDirectory: comic.localPath,
      databasePath: paths.databasePath,
      force: true,
    })
      .then((chapters) => {
        setSeriesChapters(chapters);
        setSeriesChaptersMessage(chapters.length > 0 ? "" : "无章节");
      })
      .catch((cause) => {
        setSeriesChaptersMessage(
          cause instanceof Error ? cause.message : String(cause),
        );
      });
  }

  function handleReadComic(
    comic: LibraryComic,
    source: ReaderReturn,
    chapter?: LocalChapter,
  ) {
    setReaderReturn(source);
    setReaderChapterId(chapter?.id ?? comic.lastReadChapterId ?? null);
    setSelectedComic(comic);
    if (source === "series") {
      setSeriesComic(comic);
    }
  }

  function handleCloseReader() {
    setSelectedComic(undefined);
    setReaderChapterId(null);
    if (readerReturn === "series" && seriesComic) {
      loadSeriesChapters(seriesComic);
      return true;
    }
    setSeriesComic(undefined);
    return false;
  }

  async function handleReaderPrefs(comic: LibraryComic) {
    replaceComic(comic);
    try {
      const next = await setReaderPrefs({
        databasePath: paths.databasePath,
        comicId: comic.id,
        readingDirection: comic.readingDirection,
        fitMode: comic.fitMode,
        readMode: comic.readMode,
      });
      if (next) {
        replaceComic(next);
      }
    } catch {
      // Preference persistence is best-effort.
    }
  }

  function applyPaths(nextPaths: LibraryPaths) {
    const saved = saveStoredPaths(nextPaths);
    setPaths(saved);
    allowRoots(saved);
    setStatusMessage("已更新本地路径");
    showToast("已保存路径");
    void pathIsDirectory(saved.bookshelfRoot)
      .then((isDir) => setBookshelfMissing(!isDir))
      .catch(() => undefined);
    void loadLibrary(saved)
      .then(async (result) => {
        setComics(result.comics);
        setBaselineCompleted(Boolean(result.baselineCompleted));
        const merged = mergePaths(saved, {
          bookshelfRoot: result.bookshelfRoot,
          databasePath: result.databasePath,
        });
        setPaths(merged);
        setStatusMessage(
          result.comics.length > 0
            ? `已载入 ${result.comics.length} 部本地漫画`
            : result.baselineCompleted
              ? "书库为空，扫描本地书架即可开始"
              : "书库为空，先导入现有书库作为基准",
        );
        const exists = await pathIsDirectory(merged.bookshelfRoot);
        if (exists) {
          await runScan(merged, true);
        }
      })
      .catch((cause) => {
        setStatusMessage(cause instanceof Error ? cause.message : String(cause));
      });
  }

  async function handleDeleteComic(comic: LibraryComic) {
    if (!window.confirm(`从索引删除「${comic.name}」？不会删除磁盘上的文件。`)) {
      return;
    }
    try {
      await deleteLibraryComic({
        databasePath: paths.databasePath,
        comicId: comic.id,
      });
      setComics((current) => current.filter((item) => item.id !== comic.id));
      if (seriesComic?.id === comic.id) {
        setSeriesComic(undefined);
      }
      showToast(`已从索引删除「${comic.name}」`);
    } catch (cause) {
      showToast(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handlePickBookshelf() {
    try {
      const picked = await pickDirectory();
      if (!picked) {
        return;
      }
      applyPaths({
        bookshelfRoot: picked,
        databasePath: databasePathFor(picked),
        extraRoots: paths.extraRoots ?? [],
      });
    } catch (cause) {
      showToast(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleClearProgress(comic: LibraryComic) {
    if (!window.confirm(`清除「${comic.name}」的阅读进度？`)) {
      return;
    }
    try {
      const next = await clearReadProgress({
        databasePath: paths.databasePath,
        comicId: comic.id,
      });
      if (next) {
        replaceComic(next);
      }
    } catch (cause) {
      showToast(cause instanceof Error ? cause.message : String(cause));
    }
  }

  return {
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
    replaceComic,
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
  };
}
