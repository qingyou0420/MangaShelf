import { ChevronLeft, MoveLeft, MoveRight } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState, type MouseEvent, type WheelEvent } from "react";
import { listChapterPages, listenExtractProgress, scanLocalChapters } from "../../lib/api";
import { loadReaderDefaults } from "../../lib/defaults";
import type {
  ExtractProgress,
  FitMode,
  LibraryComic,
  LocalChapter,
  ReadMode,
  ReadingDirection,
} from "../../lib/types";

export interface ReaderService {
  scanChapters: (
    comicId: string,
    comicDirectory: string,
    force?: boolean,
  ) => Promise<LocalChapter[]>;
  listPages: (chapterPath: string) => Promise<string[]>;
  peekFirstPage?: (chapterPath: string) => Promise<string | undefined>;
  saveProgress?: (
    comicId: string,
    chapterId: string,
    page: number,
  ) => Promise<void>;
}

interface ReaderViewProps {
  comic?: LibraryComic;
  service?: ReaderService;
  toImageSrc?: (path: string) => string;
  onBack?: () => void;
  onComicChange?: (comic: LibraryComic) => void;
  onToggleFavorite?: (comic: LibraryComic) => void;
  initialChapterId?: string | null;
}

const defaultReaderService: ReaderService = {
  scanChapters: (comicId, comicDirectory, force) =>
    scanLocalChapters({
      comicId,
      comicDirectory,
      force,
    }),
  listPages: (chapterPath) =>
    listChapterPages({
      chapterPath,
    }),
};

type PendingPage = "resume" | "start" | "end";

export function ReaderView({
  comic,
  service = defaultReaderService,
  toImageSrc = convertFileSrc,
  onBack,
  onComicChange,
  onToggleFavorite,
  initialChapterId,
}: ReaderViewProps) {
  const defaults = loadReaderDefaults();
  const useDefaults = !comic?.lastReadAt;
  const [chapters, setChapters] = useState<LocalChapter[]>([]);
  const [selectedChapter, setSelectedChapter] = useState<LocalChapter>();
  const [pages, setPages] = useState<string[]>([]);
  const [pageIndex, setPageIndex] = useState(0);
  const [readerMessage, setReaderMessage] = useState("从书库打开漫画");
  const [chaptersOpen, setChaptersOpen] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const [nextChapterPreview, setNextChapterPreview] = useState<string>();
  const [chromeVisible, setChromeVisible] = useState(true);
  const [imageFailed, setImageFailed] = useState(false);
  const [reloadKey, setReloadKey] = useState(0);
  const [extractProgress, setExtractProgress] = useState<ExtractProgress>();
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [direction, setDirection] = useState<ReadingDirection>(
    useDefaults ? defaults.readingDirection : (comic?.readingDirection ?? "ltr"),
  );
  const [fitMode, setFitMode] = useState<FitMode>(
    useDefaults ? defaults.fitMode : (comic?.fitMode ?? "contain"),
  );
  const [readMode, setReadMode] = useState<ReadMode>(
    useDefaults ? defaults.readMode : (comic?.readMode ?? "page"),
  );
  const [spreadCover, setSpreadCover] = useState(true);
  const serviceRef = useRef(service);
  const resumeChapterIdRef = useRef<string | null>(
    initialChapterId ?? comic?.lastReadChapterId ?? null,
  );
  const restoredRef = useRef(false);
  const pendingPageRef = useRef<PendingPage>("resume");
  const hideTimerRef = useRef<number | undefined>(undefined);
  const hoveringChromeRef = useRef(false);
  const pageIndexRef = useRef(0);
  const pagesRef = useRef<string[]>([]);
  const chaptersRef = useRef<LocalChapter[]>([]);
  const selectedRef = useRef<LocalChapter | undefined>(undefined);
  const chromeRef = useRef(true);
  const directionRef = useRef(direction);
  const readModeRef = useRef(readMode);
  const spreadCoverRef = useRef(spreadCover);
  const comicIdRef = useRef(comic?.id);
  const wheelLockRef = useRef(false);
  const heightsRef = useRef<Map<number, number>>(new Map());
  const [heightTick, setHeightTick] = useState(0);
  serviceRef.current = service;
  pageIndexRef.current = pageIndex;
  pagesRef.current = pages;
  chaptersRef.current = chapters;
  selectedRef.current = selectedChapter;
  chromeRef.current = chromeVisible;
  directionRef.current = direction;
  readModeRef.current = readMode;
  spreadCoverRef.current = spreadCover;
  comicIdRef.current = comic?.id;

  useEffect(() => {
    if (useDefaults) {
      return;
    }
    setDirection(comic?.readingDirection ?? "ltr");
    setFitMode(comic?.fitMode ?? "contain");
    setReadMode(comic?.readMode ?? "page");
  }, [comic?.id, comic?.readingDirection, comic?.fitMode, comic?.readMode, useDefaults]);

  useEffect(() => {
    let cancelled = false;
    restoredRef.current = false;
    pendingPageRef.current = "resume";
    resumeChapterIdRef.current =
      initialChapterId ?? comic?.lastReadChapterId ?? null;
    setChapters([]);
    setSelectedChapter(undefined);
    setPageIndex(0);
    setImageFailed(false);

    if (!comic?.localPath) {
      setReaderMessage("从书库打开漫画");
      return;
    }

    const comicId = comic.id;
    const localPath = comic.localPath;
    setReaderMessage("加载中…");
    void serviceRef.current
      .scanChapters(comicId, localPath, false)
      .then((nextChapters) => {
        if (cancelled) {
          return;
        }
        setChapters(nextChapters);
        const resumeId = resumeChapterIdRef.current;
        const restored =
          nextChapters.find((chapter) => chapter.id === resumeId) ??
          nextChapters.find((chapter) => chapter.readProgressPage > 0) ??
          nextChapters[0];
        setSelectedChapter(restored);
        setReaderMessage(nextChapters.length > 0 ? "" : "无章节");
      })
      .catch((cause) => {
        if (!cancelled) {
          setReaderMessage(cause instanceof Error ? cause.message : String(cause));
        }
      });

    return () => {
      cancelled = true;
    };
  }, [comic?.id, comic?.localPath, initialChapterId]);

  useEffect(() => {
    let cancelled = false;
    const chapterId = selectedChapter?.id;
    const chapterPath = selectedChapter?.path;
    const resumePage = selectedChapter?.readProgressPage ?? 0;
    if (!chapterId || !chapterPath) {
      return;
    }

    const archive = /\.(zip|cbz)$/i.test(chapterPath);
    setReaderMessage(archive ? "正在解压…" : "加载中…");
    setImageFailed(false);
    void serviceRef.current
      .listPages(chapterPath)
      .then((nextPages) => {
        if (cancelled) {
          return;
        }
        setPages(nextPages);
        heightsRef.current = new Map();
        const pending = pendingPageRef.current;
        if (pending === "end") {
          setPageIndex(lastSpreadStart(nextPages.length));
        } else if (pending === "start") {
          setPageIndex(0);
        } else if (!restoredRef.current) {
          setPageIndex(
            alignSpreadIndex(
              Math.min(resumePage, Math.max(nextPages.length - 1, 0)),
            ),
          );
        }
        restoredRef.current = true;
        pendingPageRef.current = "resume";
        setExtractProgress(undefined);
        setReaderMessage(nextPages.length === 0 ? "无图片" : "");
      })
      .catch((cause) => {
        if (!cancelled) {
          setReaderMessage(cause instanceof Error ? cause.message : String(cause));
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedChapter?.id, selectedChapter?.path]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listenExtractProgress((progress) => {
      setExtractProgress(progress);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    const index = chapters.findIndex((chapter) => chapter.id === selectedChapter?.id);
    const next = index >= 0 ? chapters[index + 1] : undefined;
    if (!next) {
      setNextChapterPreview(undefined);
      return;
    }
    let cancelled = false;
    const peek = serviceRef.current.peekFirstPage;
    const request = peek
      ? peek(next.path)
      : Promise.resolve(undefined);
    void request
      .then((first) => {
        if (!cancelled) {
          setNextChapterPreview(first);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setNextChapterPreview(undefined);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [chapters, selectedChapter?.id]);

  useEffect(() => {
    const comicId = comic?.id;
    const chapterId = selectedChapter?.id;
    if (!comicId || !chapterId || pages.length === 0 || !serviceRef.current.saveProgress) {
      return;
    }
    const timer = window.setTimeout(() => {
      void serviceRef.current.saveProgress?.(comicId, chapterId, pageIndex);
    }, 350);
    return () => {
      window.clearTimeout(timer);
    };
  }, [comic?.id, selectedChapter?.id, pageIndex, pages.length]);

  useEffect(() => {
    return () => {
      const comicId = comicIdRef.current;
      const chapterId = selectedRef.current?.id;
      const page = pageIndexRef.current;
      if (comicId && chapterId && pagesRef.current.length > 0) {
        void serviceRef.current.saveProgress?.(comicId, chapterId, page);
      }
      if (hideTimerRef.current !== undefined) {
        window.clearTimeout(hideTimerRef.current);
      }
    };
  }, []);

  function scheduleHideChrome() {
    if (hideTimerRef.current !== undefined) {
      window.clearTimeout(hideTimerRef.current);
    }
    hideTimerRef.current = window.setTimeout(() => {
      if (!hoveringChromeRef.current) {
        setChromeVisible(false);
        chromeRef.current = false;
      }
    }, 2_400);
  }

  function revealChrome() {
    setChromeVisible(true);
    chromeRef.current = true;
    scheduleHideChrome();
  }

  function selectChapter(chapter: LocalChapter, page: PendingPage) {
    pendingPageRef.current = page;
    restoredRef.current = false;
    setSelectedChapter(chapter);
  }

  function goChapter(delta: number, page: PendingPage): boolean {
    const list = chaptersRef.current;
    const currentId = selectedRef.current?.id;
    const chapterIndex = list.findIndex((chapter) => chapter.id === currentId);
    if (chapterIndex < 0) {
      return false;
    }
    const next = list[chapterIndex + delta];
    if (!next) {
      return false;
    }
    selectChapter(next, page);
    return true;
  }

  function turnPage(delta: number) {
    const index = pageIndexRef.current;
    const length = pagesRef.current.length;
    const next = nextSpreadIndex(index, delta);
    if (next < 0) {
      goChapter(-1, "end");
      return;
    }
    if (length === 0 || next >= length) {
      goChapter(1, "start");
      return;
    }
    setPageIndex(next);
  }

  function nextSpreadIndex(index: number, delta: number): number {
    if (readModeRef.current !== "spread") {
      return index + delta;
    }
    const coverAlone = spreadCoverRef.current;
    if (delta > 0) {
      if (coverAlone && index === 0) {
        return 1;
      }
      return index + 2;
    }
    if (coverAlone && index <= 1) {
      return 0;
    }
    return index - 2;
  }

  function lastSpreadStart(length: number): number {
    if (length <= 1) {
      return 0;
    }
    if (readModeRef.current !== "spread") {
      return length - 1;
    }
    if (!spreadCoverRef.current) {
      return length % 2 === 0 ? Math.max(0, length - 2) : length - 1;
    }
    const last = length - 1;
    return last % 2 === 1 ? last : last - 1;
  }

  function alignSpreadIndex(index: number): number {
    if (readModeRef.current !== "spread") {
      return index;
    }
    if (spreadCoverRef.current) {
      if (index === 0) {
        return 0;
      }
      return index % 2 === 1 ? index : index - 1;
    }
    return index % 2 === 0 ? index : index - 1;
  }

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        if (!chromeRef.current) {
          revealChrome();
          return;
        }
        onBack?.();
        return;
      }
      if (event.key === "F11") {
        event.preventDefault();
        void toggleFullscreen();
        return;
      }
      if (event.key === "f" || event.key === "F") {
        if ((event.target as HTMLElement | null)?.tagName === "INPUT") {
          return;
        }
        event.preventDefault();
        if (comic) {
          onToggleFavorite?.(comic);
        }
        return;
      }
      if (event.key === "[" || event.key === "PageUp") {
        event.preventDefault();
        goChapter(-1, "end");
        return;
      }
      if (event.key === "]" || event.key === "PageDown") {
        event.preventDefault();
        goChapter(1, "start");
        return;
      }
      if (readModeRef.current === "scroll") {
        return;
      }
      const goingNext =
        event.key === " " ||
        event.key === "ArrowRight" ||
        event.key === "d" ||
        event.key === "D";
      const goingPrev =
        event.key === "ArrowLeft" || event.key === "a" || event.key === "A";
      if (!goingNext && !goingPrev) {
        return;
      }
      event.preventDefault();
      const next = directionRef.current === "rtl" ? goingPrev : goingNext;
      turnPage(next ? 1 : -1);
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onBack, comic, onToggleFavorite]);

  const currentPage = pages[pageIndex];
  const showSpreadPair = !(spreadCover && pageIndex === 0);
  const spreadSecondIndex = Math.min(pageIndex + 1, Math.max(pages.length - 1, 0));
  const currentPageLabel =
    pages.length === 0
      ? "0 / 0"
      : readMode === "spread" && pages.length > 1 && showSpreadPair && pages[pageIndex + 1]
        ? `${pageIndex + 1}-${spreadSecondIndex + 1} / ${pages.length}`
        : `${pageIndex + 1} / ${pages.length}`;

  function spacerPx(from: number, to: number): number {
    void heightTick;
    let total = 0;
    for (let index = from; index < to; index += 1) {
      total += heightsRef.current.get(index) ?? 900;
    }
    return total;
  }

  function handleStageClick(event: MouseEvent<HTMLButtonElement>) {
    revealChrome();
    const rect = event.currentTarget.getBoundingClientRect();
    const ratio = (event.clientX - rect.left) / Math.max(rect.width, 1);
    const clickedLeft = ratio < 0.35;
    const goNext = direction === "rtl" ? clickedLeft : !clickedLeft;
    turnPage(goNext ? 1 : -1);
  }

  function handleWheel(event: WheelEvent<HTMLDivElement>) {
    if (readMode === "scroll") {
      return;
    }
    if (Math.abs(event.deltaY) < 20) {
      return;
    }
    if (wheelLockRef.current) {
      return;
    }
    wheelLockRef.current = true;
    window.setTimeout(() => {
      wheelLockRef.current = false;
    }, 280);
    turnPage(event.deltaY > 0 ? 1 : -1);
  }

  function persistPrefs(next: Partial<LibraryComic>) {
    if (comic && onComicChange) {
      onComicChange({
        ...comic,
        readingDirection: next.readingDirection ?? direction,
        fitMode: next.fitMode ?? fitMode,
        readMode: next.readMode ?? readMode,
      });
    }
  }

  function changeDirection(next: ReadingDirection) {
    setDirection(next);
    persistPrefs({ readingDirection: next });
  }

  function changeFit(next: FitMode) {
    setFitMode(next);
    persistPrefs({ fitMode: next });
  }

  function changeReadMode(next: ReadMode) {
    setReadMode(next);
    persistPrefs({ readMode: next });
  }

  async function toggleFullscreen() {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const windowRef = getCurrentWindow();
      const current = await windowRef.isFullscreen();
      await windowRef.setFullscreen(!current);
      setIsFullscreen(!current);
    } catch {
      if (document.fullscreenElement) {
        await document.exitFullscreen();
        setIsFullscreen(false);
      } else {
        await document.documentElement.requestFullscreen();
        setIsFullscreen(true);
      }
    }
  }

  const prevPage = pages[pageIndex - 1];
  const nextPage = pages[pageIndex + 1];
  const extractLabel =
    extractProgress && extractProgress.total > 0
      ? `正在解压 ${extractProgress.current}/${extractProgress.total}`
      : readerMessage;

  return (
    <section
      className={chromeVisible ? "reader-view" : "reader-view chrome-hidden"}
      aria-labelledby="reader-title"
      onMouseMove={revealChrome}
    >
      <header
        className="reader-toolbar"
        onMouseEnter={() => {
          hoveringChromeRef.current = true;
          setChromeVisible(true);
        }}
        onMouseLeave={() => {
          hoveringChromeRef.current = false;
          scheduleHideChrome();
        }}
      >
        <div className="reader-toolbar-left">
          {onBack && (
            <button
              className="secondary-action compact-action"
              type="button"
              onClick={onBack}
              aria-label="返回书库"
            >
              <ChevronLeft size={16} aria-hidden="true" />
              返回
            </button>
          )}
          <h1 id="reader-title">{comic?.name ?? "阅读"}</h1>
        </div>
        <div className="reader-toolbar-right">
          <button
            className={chaptersOpen ? "tool-button active" : "tool-button"}
            type="button"
            onClick={() => setChaptersOpen((value) => !value)}
            aria-pressed={chaptersOpen}
          >
            目录
          </button>
          <label className="reader-fit-select">
            <span className="sr-only">页面适配</span>
            <select
              value={fitMode}
              aria-label="页面适配"
              onChange={(event) => changeFit(event.target.value as FitMode)}
            >
              <option value="contain">适应</option>
              <option value="width">宽度</option>
              <option value="height">高度</option>
              <option value="original">原图</option>
            </select>
          </label>
          <div className="reader-more">
            <button
              className={moreOpen ? "tool-button active" : "tool-button"}
              type="button"
              onClick={() => setMoreOpen((value) => !value)}
            >
              更多
            </button>
            {moreOpen && (
              <div className="reader-more-menu">
                <button type="button" className="tool-button" onClick={() => changeReadMode("page")}>翻页</button>
                <button type="button" className="tool-button" onClick={() => changeReadMode("scroll")}>滚动</button>
                <button type="button" className="tool-button" onClick={() => changeReadMode("spread")}>双页</button>
                {readMode === "spread" && (
                  <button
                    type="button"
                    className={spreadCover ? "tool-button active" : "tool-button"}
                    onClick={() => setSpreadCover((value) => !value)}
                  >
                    封面单独
                  </button>
                )}
                <button type="button" className="tool-button" onClick={() => changeDirection("ltr")}>左开</button>
                <button type="button" className="tool-button" onClick={() => changeDirection("rtl")}>右开</button>
                <button type="button" className="tool-button" onClick={() => void toggleFullscreen()}>
                  {isFullscreen ? "退出全屏" : "全屏 F11"}
                </button>
              </div>
            )}
          </div>
          <button
            className="tool-button"
            type="button"
            onClick={() => turnPage(direction === "rtl" ? 1 : -1)}
            disabled={pages.length === 0 && chapters.length === 0}
            aria-label="上一页"
          >
            <MoveLeft size={16} aria-hidden="true" />
          </button>
          <span className="reader-page-count">{currentPageLabel}</span>
          <button
            className="tool-button"
            type="button"
            onClick={() => turnPage(direction === "rtl" ? -1 : 1)}
            disabled={pages.length === 0 && chapters.length === 0}
            aria-label="下一页"
          >
            <MoveRight size={16} aria-hidden="true" />
          </button>
        </div>
      </header>

      <div className={chaptersOpen ? "reader-shell" : "reader-shell chapters-collapsed"}>
        {chaptersOpen && (
          <aside className="reader-side" aria-label="章节">
            <div className="chapter-list">
              {chapters.length === 0 ? (
                <span className="muted">{extractLabel || "无章节"}</span>
              ) : (
                chapters.map((chapter) => (
                  <button
                    className={
                      selectedChapter?.id === chapter.id
                        ? "chapter-button active"
                        : "chapter-button"
                    }
                    type="button"
                    aria-label={`${chapter.title} ${chapter.pageCount} 页`}
                    key={chapter.id}
                    onClick={() => selectChapter(chapter, "resume")}
                  >
                    <span>{chapter.title}</span>
                    <span className="muted">
                      {selectedChapter?.id === chapter.id && pages.length > 0
                        ? `${pageIndex + 1}/${pages.length}`
                        : chapter.readProgressPage > 0
                          ? `${chapter.readProgressPage + 1}/${chapter.pageCount}`
                          : `${chapter.pageCount} 页`}
                    </span>
                  </button>
                ))
              )}
            </div>
          </aside>
        )}

        <div
          className={readMode === "scroll" ? "reader-stage scroll-mode" : "reader-stage"}
          onWheel={handleWheel}
        >
          {readMode === "scroll" && pages.length > 0 ? (
            <div
              className="reader-scroll"
              onScroll={(event) => {
                const node = event.currentTarget;
                const images = Array.from(node.querySelectorAll("img"));
                const mid = node.scrollTop + node.clientHeight / 2;
                let closest = pageIndex;
                let best = Number.POSITIVE_INFINITY;
                images.forEach((image, offset) => {
                  const start = Number(image.dataset.index ?? "0");
                  const center = image.offsetTop + image.offsetHeight / 2;
                  const delta = Math.abs(center - mid);
                  if (delta < best) {
                    best = delta;
                    closest = start;
                  }
                  void offset;
                });
                if (closest !== pageIndex) {
                  setPageIndex(closest);
                }
              }}
            >
              <div
                style={{
                  height: spacerPx(0, Math.max(0, pageIndex - 8)),
                }}
              />
              {pages
                .slice(Math.max(0, pageIndex - 8), pageIndex + 9)
                .map((page, sliceIndex) => {
                  const index = Math.max(0, pageIndex - 8) + sliceIndex;
                  return (
                    <img
                      key={`${page}-${index}`}
                      data-index={index}
                      className="reader-page-image fit-width"
                      src={toImageSrc(page)}
                      alt={`${selectedChapter?.title ?? ""} 第 ${index + 1} 页`}
                      onLoad={(event) => {
                        const height = event.currentTarget.offsetHeight;
                        if (heightsRef.current.get(index) === height) {
                          return;
                        }
                        heightsRef.current.set(index, height);
                        setHeightTick((tick) => tick + 1);
                      }}
                    />
                  );
                })}
              <div
                style={{
                  height: spacerPx(
                    Math.min(pages.length, pageIndex + 9),
                    pages.length,
                  ),
                }}
              />
            </div>
          ) : readMode === "spread" && currentPage && selectedChapter && !imageFailed ? (
            <button
              type="button"
              className="reader-page-hit reader-spread"
              onClick={handleStageClick}
              aria-label="点击翻页"
            >
              {direction === "rtl" && showSpreadPair && pages[pageIndex + 1] ? (
                <img
                  className={`reader-page-image fit-${fitMode}`}
                  src={toImageSrc(pages[pageIndex + 1])}
                  alt={`${selectedChapter.title} 第 ${pageIndex + 2} 页`}
                  draggable={false}
                />
              ) : null}
              <img
                className={`reader-page-image fit-${fitMode}`}
                src={toImageSrc(currentPage)}
                alt={`${selectedChapter.title} 第 ${pageIndex + 1} 页`}
                draggable={false}
                onError={() => setImageFailed(true)}
              />
              {direction !== "rtl" && showSpreadPair && pages[pageIndex + 1] ? (
                <img
                  className={`reader-page-image fit-${fitMode}`}
                  src={toImageSrc(pages[pageIndex + 1])}
                  alt={`${selectedChapter.title} 第 ${pageIndex + 2} 页`}
                  draggable={false}
                />
              ) : null}
            </button>
          ) : currentPage && selectedChapter && !imageFailed ? (
            <button
              type="button"
              className="reader-page-hit"
              onClick={handleStageClick}
              aria-label="点击翻页"
            >
              <img
                key={`${currentPage}-${reloadKey}`}
                className={`reader-page-image fit-${fitMode}`}
                src={toImageSrc(currentPage)}
                alt={`${selectedChapter.title} 第 ${pageIndex + 1} 页`}
                draggable={false}
                onError={() => setImageFailed(true)}
              />
            </button>
          ) : imageFailed && selectedChapter ? (
            <div className="reader-error">
              <p>无法加载图片</p>
              <code>{currentPage}</code>
              <button
                type="button"
                className="secondary-action"
                onClick={() => {
                  setImageFailed(false);
                  setReloadKey((value) => value + 1);
                }}
              >
                重试
              </button>
            </div>
          ) : (
            <p className="muted empty-line">{extractLabel || "—"}</p>
          )}
        </div>
      </div>

      <div className="reader-preload" aria-hidden="true">
        {prevPage && <img src={toImageSrc(prevPage)} alt="" />}
        {nextPage && <img src={toImageSrc(nextPage)} alt="" />}
        {nextChapterPreview && (
          <img src={toImageSrc(nextChapterPreview)} alt="" />
        )}
      </div>

      {selectedChapter && (
        <footer className="reader-footer">
          <span>{selectedChapter.title}</span>
          {pages.length > 1 && (
            <input
              type="range"
              min={0}
              max={pages.length - 1}
              value={pageIndex}
              aria-label="阅读进度"
              onChange={(event) => setPageIndex(Number(event.target.value))}
            />
          )}
          <span className="muted">{currentPageLabel}</span>
          <span className="muted reader-hint">
            {readMode === "scroll"
              ? "滚动阅读 · [ ] 上/下话 · F 收藏 · F11 全屏 · Esc 返回"
              : direction === "rtl"
                ? "← 下一页 · → 上一页 · [ ] 上/下话 · F 收藏 · F11 全屏 · Esc 返回"
                : "← 上一页 · → 下一页 · [ ] 上/下话 · F 收藏 · F11 全屏 · Esc 返回"}
          </span>
        </footer>
      )}
    </section>
  );
}
