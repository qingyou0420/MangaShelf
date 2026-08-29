import { useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  ArrowDownUp,
  BookMarked,
  FolderOpen,
  FolderSearch,
  Pencil,
  Play,
  Search,
  Star,
  Trash2,
} from "lucide-react";
import {
  loadLibraryViewPrefs,
  saveLibraryViewPrefs,
} from "../../lib/defaults";
import { coverProgress, lastReadChapterLabel } from "../../lib/progress";
import type { LibraryComic, ScanProgress } from "../../lib/types";
import { MetadataDialog } from "./MetadataDialog";

type LibraryFilter = "all" | "favorited" | "recent" | "missing";
type LibrarySort = "name" | "recent" | "chapters" | "updated";

interface LibraryViewProps {
  comics: LibraryComic[];
  onOpenSeries?: (comic: LibraryComic) => void;
  onReadComic?: (comic: LibraryComic) => void;
  onScanLibrary?: () => void;
  onCancelScan?: () => void;
  onToggleFavorite?: (comic: LibraryComic) => void;
  onSaveMetadata?: (
    comic: LibraryComic,
    draft: { name: string; author: string; tags: string },
  ) => void;
  onPickBookshelf?: () => void;
  onDeleteComic?: (comic: LibraryComic) => void;
  isScanning?: boolean;
  scanProgress?: ScanProgress;
  failedItems?: Array<{ title: string; error: string }>;
  bookshelfMissing?: boolean;
  baselineCompleted?: boolean;
}

const filters: Array<{ id: LibraryFilter; label: string }> = [
  { id: "all", label: "全部" },
  { id: "favorited", label: "收藏" },
  { id: "recent", label: "最近阅读" },
  { id: "missing", label: "未匹配" },
];

const sorts: Array<{ id: LibrarySort; label: string }> = [
  { id: "updated", label: "最近更新" },
  { id: "name", label: "名称" },
  { id: "recent", label: "最近阅读" },
  { id: "chapters", label: "章节" },
];

export function LibraryView({
  comics,
  onOpenSeries,
  onReadComic,
  onScanLibrary,
  onCancelScan,
  onToggleFavorite,
  onSaveMetadata,
  onPickBookshelf,
  onDeleteComic,
  isScanning = false,
  scanProgress,
  failedItems = [],
  bookshelfMissing = false,
  baselineCompleted = false,
}: LibraryViewProps) {
  const stored = loadLibraryViewPrefs();
  const [activeFilter, setActiveFilter] = useState<LibraryFilter>(stored.filter);
  const [sortBy, setSortBy] = useState<LibrarySort>(stored.sort);
  const [sortDesc, setSortDesc] = useState(stored.sortDesc);
  const [query, setQuery] = useState(stored.query);
  const [editing, setEditing] = useState<LibraryComic>();
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const searchRef = useRef<HTMLInputElement>(null);

  const continueComic = useMemo(() => {
    return comics
      .filter((comic) => comic.lastReadAt && comic.localPath)
      .slice()
      .sort((a, b) => (b.lastReadAt ?? "").localeCompare(a.lastReadAt ?? ""))[0];
  }, [comics]);

  const visibleComics = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    const filtered = comics.filter((comic) => {
      if (activeFilter === "favorited" && !comic.favorited) {
        return false;
      }
      if (activeFilter === "recent" && !comic.lastReadAt) {
        return false;
      }
      if (activeFilter === "missing" && comic.localPath) {
        return false;
      }
      if (!normalized) {
        return true;
      }
      return [comic.name, comic.location, comic.author ?? "", ...(comic.tags ?? [])]
        .join(" ")
        .toLowerCase()
        .includes(normalized);
    });

    const sorted = filtered.sort((a, b) => {
      if (sortBy === "recent") {
        return (
          (b.lastReadAt ?? "").localeCompare(a.lastReadAt ?? "") ||
          a.name.localeCompare(b.name, "zh")
        );
      }
      if (sortBy === "updated") {
        return (
          (b.shelfUpdatedAt ?? "").localeCompare(a.shelfUpdatedAt ?? "") ||
          a.name.localeCompare(b.name, "zh")
        );
      }
      if (sortBy === "chapters") {
        return (
          (b.chapterCount ?? 0) - (a.chapterCount ?? 0) ||
          a.name.localeCompare(b.name, "zh")
        );
      }
      return a.name.localeCompare(b.name, "zh");
    });
    return sortDesc ? sorted.reverse() : sorted;
  }, [activeFilter, comics, query, sortBy, sortDesc]);

  useEffect(() => {
    saveLibraryViewPrefs({
      filter: activeFilter,
      sort: sortBy,
      sortDesc,
      query,
    });
  }, [activeFilter, sortBy, sortDesc, query]);

  useEffect(() => {
    setSelectedIndex(-1);
  }, [activeFilter, query, sortBy, sortDesc, comics.length]);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null;
      const typing =
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.tagName === "SELECT");
      if (event.key === "/" && !typing) {
        event.preventDefault();
        searchRef.current?.focus();
        return;
      }
      if (typing) {
        return;
      }
      if (event.key === "ArrowRight" || event.key === "ArrowDown") {
        event.preventDefault();
        setSelectedIndex((index) =>
          Math.min(
            visibleComics.length - 1,
            index < 0 ? 0 : index + 1,
          ),
        );
      } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
        event.preventDefault();
        setSelectedIndex((index) =>
          Math.max(0, index < 0 ? 0 : index - 1),
        );
      } else if (event.key === "Enter") {
        const comic = visibleComics[selectedIndex];
        if (comic?.localPath) {
          onOpenSeries?.(comic);
        }
      } else if (event.key === "f" || event.key === "F") {
        const comic = visibleComics[selectedIndex];
        if (comic) {
          onToggleFavorite?.(comic);
        }
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [visibleComics, selectedIndex, onOpenSeries, onToggleFavorite]);

  function openEditor(comic: LibraryComic) {
    setEditing(comic);
  }

  const continueChapter = continueComic
    ? lastReadChapterLabel(continueComic)
    : undefined;

  return (
    <section className="view" aria-labelledby="library-title">
      <div className="view-header">
        <h1 id="library-title">书库</h1>
        <div className="view-actions">
          {isScanning && onCancelScan && (
            <button
              className="secondary-action"
              type="button"
              onClick={onCancelScan}
            >
              停止扫描
            </button>
          )}
          <button
            className="secondary-action"
            type="button"
            onClick={onScanLibrary}
            disabled={isScanning}
            title={
              baselineCompleted
                ? "扫描新出现的书文件夹和话文件夹。改文件、刷新封面不会标成更新。"
                : "一次性导入现有书库。已有的书和话只建立索引，不会在封面上标更新。"
            }
          >
            <FolderOpen size={16} aria-hidden="true" />
            {isScanning
              ? scanProgress && scanProgress.total > 0
                ? `扫描中 ${scanProgress.scanned}/${scanProgress.total}`
                : "扫描中…"
              : baselineCompleted
                ? "扫描书架"
                : "导入现有书库"}
          </button>
        </div>
      </div>

      {isScanning && scanProgress?.currentTitle && (
        <p className="scan-progress-line muted">
          正在处理 {scanProgress.currentTitle}
        </p>
      )}

      {!baselineCompleted && !bookshelfMissing && !isScanning && (
        <p className="scan-progress-line muted">
          首次导入会索引全部已有漫画，不会把它们标成更新。之后新书和新话会排在前面，并在封面标出更新了几话。
        </p>
      )}

      {failedItems.length > 0 && (
        <details className="scan-failed-list">
          <summary>扫描失败 {failedItems.length} 本</summary>
          <ul>
            {failedItems.map((item) => (
              <li key={item.title}>
                <strong>{item.title}</strong>
                <span className="muted"> {item.error}</span>
              </li>
            ))}
          </ul>
        </details>
      )}

      {continueComic && onReadComic && (
        <button
          type="button"
          className="continue-card"
          onClick={() => onReadComic(continueComic)}
        >
          <BookCover comic={continueComic} compact />
          <div className="continue-card-body">
            <span className="muted">继续阅读</span>
            <strong>{continueComic.name}</strong>
            <span>
              {continueChapter ? continueChapter : "本地漫画"}
              {continueComic.readProgressPage > 0
                ? ` · 第 ${continueComic.readProgressPage + 1} 页`
                : ""}
            </span>
          </div>
          <span className="continue-card-action">
            <Play size={16} aria-hidden="true" />
            继续
          </span>
        </button>
      )}

      <div className="library-toolbar">
        <label className="search-field">
          <Search size={15} aria-hidden="true" />
          <input
            ref={searchRef}
            type="search"
            value={query}
            placeholder="搜索标题、作者、标签"
            onChange={(event) => setQuery(event.target.value)}
            aria-label="搜索书库"
          />
        </label>
        <label className="toolbar-select">
          <span className="toolbar-group-label">筛选</span>
          <select
            aria-label="筛选"
            value={activeFilter}
            onChange={(event) =>
              setActiveFilter(event.target.value as LibraryFilter)
            }
          >
            {filters.map((filter) => (
              <option key={filter.id} value={filter.id}>
                {filter.label}
              </option>
            ))}
          </select>
        </label>
        <label className="toolbar-select">
          <span className="toolbar-group-label">排序</span>
          <select
            aria-label="排序"
            value={sortBy}
            onChange={(event) => setSortBy(event.target.value as LibrarySort)}
          >
            {sorts.map((sort) => (
              <option key={sort.id} value={sort.id}>
                {sort.label}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className={sortDesc ? "icon-action active" : "icon-action"}
          aria-pressed={sortDesc}
          aria-label={sortDesc ? "切换为正序" : "倒序排列"}
          title={sortDesc ? "当前倒序，点击恢复正序" : "倒序排列"}
          onClick={() => setSortDesc((value) => !value)}
        >
          <ArrowDownUp size={16} aria-hidden="true" />
        </button>
        <span className="library-count muted">
          {visibleComics.length}
          {comics.length > 0 ? ` / ${comics.length}` : ""}
        </span>
      </div>

      <div className="library-grid">
        {visibleComics.map((comic, index) => {
          const canOpen = Boolean(comic.localPath && onOpenSeries);
          const progress = coverProgress(comic);
          const selected = index === selectedIndex;
          const updateBadge = coverUpdateBadge(comic);
          return (
            <article
              className={[
                canOpen ? "library-tile interactive" : "library-tile",
                selected ? "selected" : "",
              ]
                .filter(Boolean)
                .join(" ")}
              key={comic.id}
              ref={(node) => {
                if (selected && node && typeof node.scrollIntoView === "function") {
                  node.scrollIntoView({ block: "nearest" });
                }
              }}
            >
              <button
                type="button"
                className="library-tile-button"
                disabled={!canOpen}
                aria-label={canOpen ? `查看 ${comic.name}` : comic.name}
                onClick={() => {
                  if (canOpen && onOpenSeries) {
                    onOpenSeries(comic);
                  }
                }}
              >
                <div className="cover-frame">
                  <BookCover comic={comic} />
                  {updateBadge && (
                    <span className="cover-update-badge">{updateBadge}</span>
                  )}
                  {progress > 0 && (
                    <span
                      className="cover-progress"
                      style={{ width: `${Math.round(progress * 100)}%` }}
                    />
                  )}
                </div>
                <div className="library-tile-body">
                  <h2 title={comic.name}>{comic.name}</h2>
                  <p className="tile-meta muted">
                    {comic.author ? (
                      <>
                        <span
                          className="linkish-button"
                          role="link"
                          onClick={(event) => {
                            event.preventDefault();
                            event.stopPropagation();
                            setQuery(comic.author ?? "");
                          }}
                        >
                          {comic.author}
                        </span>
                        {" · "}
                      </>
                    ) : null}
                    {comic.localPath
                      ? `${comic.chapterCount} 话`
                      : statusLabel(comic)}
                    {comic.tags.slice(0, 2).map((tag) => (
                      <span key={tag}>
                        {" · "}
                        <span
                          className="linkish-button"
                          role="link"
                          onClick={(event) => {
                            event.preventDefault();
                            event.stopPropagation();
                            setQuery(tag);
                          }}
                        >
                          {tag}
                        </span>
                      </span>
                    ))}
                  </p>
                </div>
              </button>
              <div className="library-tile-actions">
                <button
                  type="button"
                  className={comic.favorited ? "icon-action active" : "icon-action"}
                  aria-label={
                    comic.favorited ? `取消收藏 ${comic.name}` : `收藏 ${comic.name}`
                  }
                  onClick={() => onToggleFavorite?.(comic)}
                >
                  <Star size={15} fill={comic.favorited ? "currentColor" : "none"} />
                </button>
                <button
                  type="button"
                  className="icon-action"
                  aria-label={`编辑 ${comic.name}`}
                  onClick={() => openEditor(comic)}
                >
                  <Pencil size={15} />
                </button>
                {onDeleteComic && (
                  <button
                    type="button"
                    className="icon-action"
                    aria-label={`从索引删除 ${comic.name}`}
                    onClick={() => onDeleteComic(comic)}
                  >
                    <Trash2 size={15} />
                  </button>
                )}
              </div>
            </article>
          );
        })}
      </div>

      {visibleComics.length === 0 && (
        <div className="empty-hint">
          <FolderSearch size={18} aria-hidden="true" />
          <div className="empty-hint-text">
            <strong>
              {emptyTitle(activeFilter, query, comics.length, bookshelfMissing)}
            </strong>
            <p>
              {emptyHint(
                activeFilter,
                query,
                comics.length,
                bookshelfMissing,
                baselineCompleted,
              )}
            </p>
          </div>
          {comics.length === 0 && onPickBookshelf && (
            <button
              type="button"
              className="primary-action"
              onClick={onPickBookshelf}
            >
              选择书架文件夹
            </button>
          )}
        </div>
      )}

      {editing && (
        <MetadataDialog
          comic={editing}
          onClose={() => setEditing(undefined)}
          onSave={(draft) => {
            onSaveMetadata?.(editing, draft);
            setEditing(undefined);
          }}
        />
      )}
    </section>
  );
}

function emptyTitle(
  filter: LibraryFilter,
  query: string,
  total: number,
  bookshelfMissing = false,
): string {
  if (query.trim()) {
    return "没有匹配的搜索结果";
  }
  if (total === 0) {
    return bookshelfMissing ? "找不到书架文件夹" : "书库还是空的";
  }
  switch (filter) {
    case "favorited":
      return "还没有收藏";
    case "recent":
      return "还没有阅读记录";
    case "missing":
      return "没有未匹配的漫画";
    default:
      return "暂无内容";
  }
}

function emptyHint(
  filter: LibraryFilter,
  query: string,
  total: number,
  bookshelfMissing = false,
  baselineCompleted = false,
): string {
  if (query.trim()) {
    return "试试清空搜索，或换个关键词。";
  }
  if (total === 0) {
    return bookshelfMissing
      ? "默认路径不存在。选择你的漫画文件夹后再导入。不会改动或删除文件。"
      : baselineCompleted
        ? "选择本地书架文件夹，然后点右上角「扫描书架」。不会改动或删除你的漫画文件。"
        : "选择本地书架文件夹，然后点右上角「导入现有书库」。首次导入只建立索引，不会把已有漫画标成更新。";
  }
  switch (filter) {
    case "favorited":
      return "把鼠标移到封面上，点星标即可收藏。";
    case "recent":
      return "打开任意漫画阅读后，这里会出现最近记录。";
    case "missing":
      return "全部书目都已匹配到本地文件夹。";
    default:
      return "调整筛选或搜索后再试。";
  }
}

function BookCover({
  comic,
  compact = false,
}: {
  comic: LibraryComic;
  compact?: boolean;
}) {
  if (comic.coverPath) {
    return (
      <img
        className={compact ? "book-cover compact" : "book-cover"}
        src={convertFileSrc(comic.coverPath)}
        alt=""
        loading="lazy"
      />
    );
  }

  return (
    <div className={compact ? "book-tile compact" : "book-tile"} aria-hidden="true">
      <BookMarked size={compact ? 20 : 28} />
    </div>
  );
}

function coverUpdateBadge(comic: LibraryComic): string | undefined {
  if (!comic.shelfUpdatedAt || !comic.localPath) {
    return undefined;
  }
  const note = comic.shelfUpdateNote?.trim();
  if (!note) {
    return "有更新";
  }
  if (note === "新书") {
    return "新书";
  }
  const count = note.match(/^(?:新增|更新了)\s*(\d+)\s*话$/);
  if (count) {
    return `更新了${count[1]}话`;
  }
  return note;
}

function statusLabel(comic: LibraryComic) {
  if (comic.scanStatus === "missing") {
    return "未匹配";
  }
  return "待扫描";
}
