export interface CompanionPaths {
  mangaConExecutable: string;
  mangaConFavoritesJson: string;
  bookshelfRoot: string;
}

export interface MangaConFavorite {
  id: string;
  title: string;
  tags: string[];
  sourceUrl?: string;
  author?: string;
  favoritedAt?: string;
}

export interface BookshelfMatch {
  title: string;
  directory: string;
  chapterCount: number;
  imageCount: number;
}

export interface ImportFavoritesResult {
  imported: number;
  favorites: MangaConFavorite[];
  unmatchedTitles: string[];
}
