import { BookMarked, FolderSearch, ListFilter } from "lucide-react";
import type { MangaConFavorite } from "../../lib/types";

interface LibraryViewProps {
  favorites: MangaConFavorite[];
}

export function LibraryView({ favorites }: LibraryViewProps) {
  return (
    <section className="view" aria-labelledby="library-title">
      <div className="view-header compact">
        <div>
          <p className="section-kicker">收藏书库</p>
          <h1 id="library-title">导入收藏</h1>
          <p className="view-subtitle">使用 fixture 数据预览收藏卡片和匹配状态。</p>
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
              <span>{favorite.scanStatus === "pending" ? "待扫描" : "已处理"}</span>
              <small>{favorite.sourceUri}</small>
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
