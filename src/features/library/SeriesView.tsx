import { useMemo, useState } from "react";
import { BookMarked, ChevronLeft, Star } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { OverflowMenu } from "../../components/OverflowMenu";
import { Select } from "../../components/Select";
import { lastReadChapterLabel } from "../../lib/progress";
import type { LibraryComic, LocalChapter } from "../../lib/types";
import { MetadataDialog } from "./MetadataDialog";

interface SeriesViewProps {
  comic: LibraryComic;
  chapters?: LocalChapter[];
  chaptersMessage?: string;
  onBack?: () => void;
  onRead?: (comic: LibraryComic, chapter?: LocalChapter) => void;
  onToggleFavorite?: (comic: LibraryComic) => void;
  onSaveMetadata?: (
    comic: LibraryComic,
    draft: { name: string; author: string; tags: string },
  ) => void;
  onOpenFolder?: (path: string) => void;
  onRescan?: (comic: LibraryComic) => void;
  onListCovers?: (comic: LibraryComic) => Promise<string[]>;
  onSetCover?: (comic: LibraryComic, sourcePath: string) => void;
  onClearProgress?: (comic: LibraryComic) => void;
}

export function SeriesView({
  comic,
  chapters = [],
  chaptersMessage,
  onBack,
  onRead,
  onToggleFavorite,
  onSaveMetadata,
  onOpenFolder,
  onRescan,
  onListCovers,
  onSetCover,
  onClearProgress,
}: SeriesViewProps) {
  const [editing, setEditing] = useState(false);
  const [coverCandidates, setCoverCandidates] = useState<string[]>();
  const [kindFilter, setKindFilter] = useState<
    "all" | "regular" | "volume" | "machine_translation" | "other"
  >("all");

  const continueLabel = lastReadChapterLabel(comic);
  const canRead = Boolean(comic.localPath);
  const visibleChapters = useMemo(() => {
    if (kindFilter === "all") {
      return chapters;
    }
    return chapters.filter((chapter) => chapter.specialKind === kindFilter);
  }, [chapters, kindFilter]);

  return (
    <section className="view series-view" aria-labelledby="series-title">
      <div className="view-header">
        {onBack && (
          <button
            type="button"
            className="secondary-action compact-action"
            onClick={onBack}
          >
            <ChevronLeft size={16} aria-hidden="true" />
            返回
          </button>
        )}
        <h1 id="series-title">{comic.name}</h1>
      </div>

      <div className="series-layout">
        <div className="series-hero">
          {comic.coverPath && (
            <div
              className="series-hero-glow"
              style={{ backgroundImage: `url("${convertFileSrc(comic.coverPath)}")` }}
            />
          )}
          {comic.coverPath ? (
            <img
              className="series-cover"
              src={convertFileSrc(comic.coverPath)}
              alt=""
            />
          ) : (
            <div className="series-cover placeholder" aria-hidden="true">
              <BookMarked size={36} />
            </div>
          )}
          <div className="series-meta">
            {comic.author && <p className="muted">{comic.author}</p>}
            <p className="muted">
              {comic.localPath
                ? `${comic.chapterCount} 话`
                : comic.scanStatus === "missing"
                  ? "文件夹已不在"
                  : "待扫描"}
            </p>
            {comic.tags.length > 0 && (
              <div className="tag-row">
                {comic.tags.map((tag) => (
                  <span className="tag" key={tag}>
                    {tag}
                  </span>
                ))}
              </div>
            )}
            <div className="series-actions">
              <button
                type="button"
                className="primary-action"
                disabled={!canRead}
                onClick={() => onRead?.(comic)}
              >
                {continueLabel ? "继续阅读" : "开始阅读"}
              </button>
              <button
                type="button"
                className={comic.favorited ? "icon-action active" : "icon-action"}
                aria-label={comic.favorited ? "取消收藏" : "收藏"}
                onClick={() => onToggleFavorite?.(comic)}
              >
                <Star size={16} fill={comic.favorited ? "currentColor" : "none"} />
              </button>
              <OverflowMenu
                items={[
                  {
                    id: "edit",
                    label: "编辑元数据",
                    onSelect: () => setEditing(true),
                  },
                  {
                    id: "folder",
                    label: "打开文件夹",
                    hidden: !comic.localPath,
                    onSelect: () => onOpenFolder?.(comic.localPath ?? ""),
                  },
                  {
                    id: "rescan",
                    label: "重新扫描",
                    hidden: !comic.localPath,
                    onSelect: () => onRescan?.(comic),
                  },
                  {
                    id: "cover",
                    label: "选择封面",
                    hidden: !comic.localPath || !onListCovers,
                    onSelect: () => {
                      void onListCovers?.(comic).then(setCoverCandidates);
                    },
                  },
                  {
                    id: "clear",
                    label: "清除进度",
                    hidden: !onClearProgress || !comic.lastReadAt,
                    onSelect: () => onClearProgress?.(comic),
                  },
                ]}
              />
            </div>
            {continueLabel && (
              <p className="muted">
                上次读到 {continueLabel}
                {comic.readProgressPage > 0
                  ? ` · 第 ${comic.readProgressPage + 1} 页`
                  : ""}
              </p>
            )}
          </div>
        </div>

        <div className="series-chapters">
          <div className="series-chapters-head">
            <h2>目录</h2>
            {chapters.length > 0 && (
              <Select
                label="章节类型"
                value={kindFilter}
                options={[
                  { value: "all", label: "全部" },
                  { value: "regular", label: "话" },
                  { value: "volume", label: "卷" },
                  { value: "machine_translation", label: "机翻" },
                  { value: "other", label: "其他" },
                ]}
                onChange={setKindFilter}
              />
            )}
          </div>
          {visibleChapters.length === 0 ? (
            <p className="muted">{chaptersMessage || "无章节"}</p>
          ) : (
            <ul className="series-chapter-list">
              {visibleChapters.map((chapter) => {
                const isCurrent = comic.lastReadChapterId === chapter.id;
                const isRead = chapter.readProgressPage > 0 && !isCurrent;
                return (
                  <li key={chapter.id}>
                    <button
                      type="button"
                      className={[
                        "series-chapter-item",
                        isCurrent ? "current" : "",
                        isRead ? "read" : "",
                      ]
                        .filter(Boolean)
                        .join(" ")}
                      disabled={!canRead}
                      onClick={() => onRead?.(comic, chapter)}
                    >
                      <span>{chapter.title}</span>
                      <span className="muted">
                        {chapter.readProgressPage > 0
                          ? `${chapter.readProgressPage + 1}/${chapter.pageCount} 页`
                          : `${chapter.pageCount} 页`}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </div>

      {coverCandidates && (
        <div className="modal-backdrop" role="presentation" onClick={() => setCoverCandidates(undefined)}>
          <div
            className="modal-dialog"
            role="dialog"
            aria-label="选择封面"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="modal-header">
              <h2>选择封面</h2>
            </div>
            <div className="cover-picker-grid">
              {coverCandidates.map((path) => (
                <button
                  type="button"
                  key={path}
                  className="cover-picker-item"
                  onClick={() => {
                    onSetCover?.(comic, path);
                    setCoverCandidates(undefined);
                  }}
                >
                  <img src={convertFileSrc(path)} alt="" />
                </button>
              ))}
            </div>
            <div className="modal-actions">
              <button
                type="button"
                className="secondary-action"
                onClick={() => setCoverCandidates(undefined)}
              >
                取消
              </button>
            </div>
          </div>
        </div>
      )}

      {editing && (
        <MetadataDialog
          comic={comic}
          onClose={() => setEditing(false)}
          onSave={(draft) => {
            onSaveMetadata?.(comic, draft);
            setEditing(false);
          }}
        />
      )}
    </section>
  );
}
