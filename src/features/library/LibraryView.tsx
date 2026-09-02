import { useEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  AlertTriangle,
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
import { Select } from "../../components/Select";
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
  const [failedOpen, setFailedOpen] = useState(false);
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
          {continueComic && onReadComic && (
            <button
              type="button"
              className="continue-pill"
              title={
                continueChapter
                  ? `${continueComic.name} · ${continueChapter}`
                  : continueComic.name
              }
              onClick={() => onReadComic(continueComic)}
            >
              <Play size={14} aria-hidden="true" />
              继续阅读
            </button>
          )}
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
                ? "只索引新增内容，不改动文件。"
                : "首次导入不标记更新。"
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

      {!baselineCompleted &&
        !bookshelfMissing &&
        !isScanning &&
        comics.length > 0 && (
        <p className="scan-progress-line muted">
          首次导入只建立索引，不会标记更新。
        </p>
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
        <div className="segmented" role="group" aria-label="筛选">
          {filters.map((filter) => (
            <button
              key={filter.id}
              type="button"
              className={
                activeFilter === filter.id
                  ? "segmented-item active"
                  : "segmented-item"
              }
              aria-pressed={activeFilter === filter.id}
              onClick={() => setActiveFilter(filter.id)}
            >
              {filter.label}
            </button>
          ))}
        </div>
        <Select
          label="排序"
          value={sortBy}
          options={sorts.map((sort) => ({ value: sort.id, label: sort.label }))}
          onChange={setSortBy}
        />
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
        {failedItems.length > 0 && (
          <div className="toolbar-alert">
            <button
              type="button"
              className="icon-action"
              aria-label={`扫描失败 ${failedItems.length} 本`}
              aria-expanded={failedOpen}
              onClick={() => setFailedOpen((open) => !open)}
            >
              <AlertTriangle size={15} />
            </button>
            {failedOpen && (
              <div className="toolbar-alert-pop" role="status">
                <strong>扫描失败 {failedItems.length} 本</strong>
                <ul>
                  {failedItems.map((item) => (
                    <li key={item.title}>
                      <strong>{item.title}</strong>
                      <span className="muted"> {item.error}</span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        )}
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
                <div className={comic.coverPath ? "cover-frame" : "cover-frame is-empty"}>
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
    return "换个关键词试试。";
  }
  if (total === 0) {
    return bookshelfMissing
      ? "选择漫画文件夹后再导入。"
      : baselineCompleted
        ? "点右上角「扫描书架」。"
        : "点右上角「导入现有书库」。不会改动或删除文件。";
  }
  switch (filter) {
    case "favorited":
      return "在封面上点 ★";
    case "recent":
      return "阅读后会出现在这里。";
    case "missing":
      return "全部书目都已匹配。";
    default:
      return "调整筛选后再试。";
  }
}

function BookCover({ comic }: { comic: LibraryComic }) {
  const [loaded, setLoaded] = useState(!comic.coverPath);
  if (comic.coverPath) {
    return (
      <img
        className={loaded ? "book-cover" : "book-cover is-loading"}
        src={convertFileSrc(comic.coverPath)}
        alt=""
        loading="lazy"
        onLoad={() => setLoaded(true)}
      />
    );
  }

  return (
    <div className="book-tile" aria-hidden="true">
      <BookMarked size={28} />
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
