import {
  Bookmark,
  Columns2,
  Expand,
  Maximize2,
  MoveLeft,
  MoveRight,
  PanelTop,
  RotateCw,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { listChapterPages, scanLocalChapters } from "../../lib/api";
import type { LocalChapter, MangaConFavorite } from "../../lib/types";

const readerTools = [
  { label: "书签", icon: Bookmark },
  { label: "双页", icon: Columns2 },
  { label: "缩小", icon: ZoomOut },
  { label: "放大", icon: ZoomIn },
  { label: "全屏", icon: Maximize2 },
  { label: "方向", icon: RotateCw },
];

export interface ReaderService {
  scanChapters: (comicId: string, comicDirectory: string) => Promise<LocalChapter[]>;
  listPages: (chapterPath: string) => Promise<string[]>;
}

interface ReaderViewProps {
  comic?: MangaConFavorite;
  service?: ReaderService;
  toImageSrc?: (path: string) => string;
}

const defaultReaderService: ReaderService = {
  scanChapters: (comicId, comicDirectory) =>
    scanLocalChapters({
      comicId,
      comicDirectory,
    }),
  listPages: (chapterPath) =>
    listChapterPages({
      chapterPath,
    }),
};

export function ReaderView({
  comic,
  service = defaultReaderService,
  toImageSrc = convertFileSrc,
}: ReaderViewProps) {
  const [chapters, setChapters] = useState<LocalChapter[]>([]);
  const [selectedChapter, setSelectedChapter] = useState<LocalChapter>();
  const [pages, setPages] = useState<string[]>([]);
  const [pageIndex, setPageIndex] = useState(0);
  const [readerMessage, setReaderMessage] = useState("选择一本本地漫画后开始阅读");

  useEffect(() => {
    let cancelled = false;
    setChapters([]);
    setSelectedChapter(undefined);
    setPages([]);
    setPageIndex(0);

    if (!comic?.localPath) {
      setReaderMessage("选择一本本地漫画后开始阅读");
      return;
    }

    setReaderMessage("正在扫描章节...");
    void service
      .scanChapters(comic.id, comic.localPath)
      .then((nextChapters) => {
        if (cancelled) {
          return;
        }
        setChapters(nextChapters);
        setSelectedChapter(nextChapters[0]);
        setReaderMessage(
          nextChapters.length > 0 ? "章节已载入" : "未发现可阅读章节",
        );
      })
      .catch((cause) => {
        if (!cancelled) {
          setReaderMessage(cause instanceof Error ? cause.message : String(cause));
        }
      });

    return () => {
      cancelled = true;
    };
  }, [comic, service]);

  useEffect(() => {
    let cancelled = false;
    setPages([]);
    setPageIndex(0);

    if (!selectedChapter) {
      return;
    }

    setReaderMessage("正在加载图片页...");
    void service
      .listPages(selectedChapter.path)
      .then((nextPages) => {
        if (cancelled) {
          return;
        }
        setPages(nextPages);
        setReaderMessage(nextPages.length > 0 ? "图片页已载入" : "章节没有图片页");
      })
      .catch((cause) => {
        if (!cancelled) {
          setReaderMessage(cause instanceof Error ? cause.message : String(cause));
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedChapter, service]);

  const currentPage = pages[pageIndex];
  const currentPageLabel = pages.length === 0 ? "0 / 0" : `${pageIndex + 1} / ${pages.length}`;

  function handleSelectChapter(chapter: LocalChapter) {
    setSelectedChapter(chapter);
  }

  function handlePreviousPage() {
    setPageIndex((index) => Math.max(0, index - 1));
  }

  function handleNextPage() {
    setPageIndex((index) => Math.min(pages.length - 1, index + 1));
  }

  return (
    <section className="view reader-view" aria-labelledby="reader-title">
      <div className="reader-toolbar" aria-label="阅读器工具栏">
        <div>
          <p className="section-kicker">阅读</p>
          <h1 id="reader-title">{comic?.name ?? "阅读器"}</h1>
        </div>
        <div className="tool-button-row">
          <button
            className="tool-button"
            type="button"
            onClick={handlePreviousPage}
            disabled={pageIndex === 0 || pages.length === 0}
          >
            <MoveLeft size={17} aria-hidden="true" />
            <span>上一页</span>
          </button>
          <button
            className="tool-button"
            type="button"
            onClick={handleNextPage}
            disabled={pageIndex >= pages.length - 1}
          >
            <MoveRight size={17} aria-hidden="true" />
            <span>下一页</span>
          </button>
          {readerTools.map(({ label, icon: Icon }) => (
            <button className="tool-button" type="button" key={label} aria-label={label}>
              <Icon size={17} aria-hidden="true" />
              <span>{label}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="reader-shell">
        <aside className="reader-side">
          <div className="reader-side-title">
            <PanelTop size={18} aria-hidden="true" />
            <span>章节</span>
          </div>
          <div className="chapter-list" aria-label="章节列表">
            {chapters.length === 0 ? (
              <span>{readerMessage}</span>
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
                  onClick={() => handleSelectChapter(chapter)}
                >
                  <span>{chapter.title}</span>
                  <small>{chapter.pageCount} 页</small>
                </button>
              ))
            )}
          </div>
        </aside>
        <div className="reader-stage">
          {currentPage && selectedChapter ? (
            <>
              <div className="reader-page-meta">
                <span>{selectedChapter.title}</span>
                <strong>{currentPageLabel}</strong>
              </div>
              <img
                className="reader-page-image"
                src={toImageSrc(currentPage)}
                alt={`${selectedChapter.title} 第 ${pageIndex + 1} 页`}
              />
            </>
          ) : (
            <>
              <Expand size={32} aria-hidden="true" />
              <h2>{readerMessage}</h2>
              <p>导入并选择一本已匹配本地目录的漫画后，这里会显示图片页。</p>
            </>
          )}
        </div>
      </div>
    </section>
  );
}
