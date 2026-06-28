import { BookMarked, BookOpen, FolderSearch, ListFilter } from "lucide-react";
import type { MangaConFavorite } from "../../lib/types";

interface LibraryViewProps {
  favorites: MangaConFavorite[];
  onReadFavorite?: (favorite: MangaConFavorite) => void;
}

export function LibraryView({ favorites, onReadFavorite }: LibraryViewProps) {
  return (
    <section className="view" aria-labelledby="library-title">
      <div className="view-header compact">
        <div>
          <p className="section-kicker">收藏书库</p>
          <h1 id="library-title">导入收藏</h1>
          <p className="view-subtitle">查看漫画控收藏和本地书架匹配状态。</p>
        </div>
        <button className="secondary-action" type="button">
          <ListFilter size={18} aria-hidden="true" />
          筛选
        </button>
      </div>

      <div className="library-list">
        {favorites.map((favorite) => (
          <article className="library-card" key={favorite.id}>
            <div className="book-tile">
              <BookMarked size={24} aria-hidden="true" />
            </div>
            <div className="library-card-main">
              <div>
                <h2>{favorite.name}</h2>
                <p>{favorite.location}</p>
              </div>
              <div className="tag-row">
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

      <div className="empty-hint">
        <FolderSearch size={18} aria-hidden="true" />
        导入完成后会在这里显示本地匹配、章节数和阅读进度。
      </div>
    </section>
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
