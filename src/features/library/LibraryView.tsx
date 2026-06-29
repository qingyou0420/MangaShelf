import { useMemo, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { BookMarked, BookOpen, FolderSearch } from "lucide-react";
import type { MangaConFavorite } from "../../lib/types";

type LibraryFilter = "all" | "matched" | "missing" | "updated";

interface LibraryViewProps {
  favorites: MangaConFavorite[];
  onReadFavorite?: (favorite: MangaConFavorite) => void;
}

const filters: Array<{ id: LibraryFilter; label: string }> = [
  { id: "all", label: "全部" },
  { id: "matched", label: "已下载" },
  { id: "missing", label: "未匹配" },
  { id: "updated", label: "有更新" },
];

export function LibraryView({ favorites, onReadFavorite }: LibraryViewProps) {
  const [activeFilter, setActiveFilter] = useState<LibraryFilter>("all");
  const filteredFavorites = useMemo(
    () =>
      favorites.filter((favorite) => {
        if (activeFilter === "matched") {
          return Boolean(favorite.localPath);
        }
        if (activeFilter === "missing") {
          return !favorite.localPath;
        }
        if (activeFilter === "updated") {
          return Boolean(favorite.hasUpdate);
        }
        return true;
      }),
    [activeFilter, favorites],
  );

  return (
    <section className="view" aria-labelledby="library-title">
      <div className="view-header compact">
        <div>
          <p className="section-kicker">收藏书库</p>
          <h1 id="library-title">导入收藏</h1>
          <p className="view-subtitle">查看漫画控收藏、本地书架匹配、封面和阅读入口。</p>
        </div>
        <div className="filter-tabs" aria-label="书库筛选">
          {filters.map((filter) => (
            <button
              className={activeFilter === filter.id ? "filter-tab active" : "filter-tab"}
              key={filter.id}
              type="button"
              onClick={() => setActiveFilter(filter.id)}
            >
              {filter.label}
            </button>
          ))}
        </div>
      </div>

      <div className="library-list">
        {filteredFavorites.map((favorite) => (
          <article className="library-card" key={favorite.id}>
            <BookCover favorite={favorite} />
            <div className="library-card-main">
              <div>
                <h2>{favorite.name}</h2>
                <p>{favorite.location}</p>
              </div>
              <div className="tag-row">
                {favorite.hasUpdate && <span className="tag update-tag">有更新</span>}
                {favorite.latestChapterTitle && (
                  <span className="tag">最新 {favorite.latestChapterTitle}</span>
                )}
                {favorite.tags.map((tag) => (
                  <span className="tag" key={tag}>
                    {tag}
                  </span>
                ))}
              </div>
            </div>
            <div className="library-meta">
              <span>{favoriteStatusLabel(favorite)}</span>
              <small>{favorite.sourceUri}</small>
              {favorite.localPath && (
                <small>{`本地 ${favorite.chapterCount} 章 / ${favorite.imageCount} 页`}</small>
              )}
              {favorite.localPath && onReadFavorite && (
                <button
                  className="secondary-action compact-action"
                  type="button"
                  aria-label={`阅读 ${favorite.name}`}
                  onClick={() => onReadFavorite(favorite)}
                >
                  <BookOpen size={16} aria-hidden="true" />
                  阅读
                </button>
              )}
            </div>
          </article>
        ))}
      </div>

      {filteredFavorites.length === 0 && (
        <div className="empty-hint">
          <FolderSearch size={18} aria-hidden="true" />
          当前筛选没有漫画。导入收藏并扫描本地书架后会显示匹配、章节数和封面。
        </div>
      )}
    </section>
  );
}

function BookCover({ favorite }: { favorite: MangaConFavorite }) {
  if (favorite.coverPath) {
    return (
      <img
        className="book-cover"
        src={convertFileSrc(favorite.coverPath)}
        alt={`${favorite.name} 封面`}
        loading="lazy"
      />
    );
  }

  return (
    <div className="book-tile" aria-hidden="true">
      <BookMarked size={24} />
    </div>
  );
}

function favoriteStatusLabel(favorite: MangaConFavorite) {
  if (favorite.localPath) {
    return "已匹配本地";
  }

  if (favorite.scanStatus === "missing") {
    return "缺少本地目录";
  }

  if (favorite.scanStatus === "imported") {
    return "已导入";
  }

  return favorite.scanStatus === "pending" ? "待匹配" : "已处理";
}
