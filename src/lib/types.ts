export interface CompanionPaths {
  mangaConExecutable: string;
  mangaConFavoritesJson: string;
  bookshelfRoot: string;
}

export interface MangaConFavorite {
  id: string;
  name: string;
  location: string;
  tags: string[];
  sourceUri: string;
  sourceScheme?: string;
  sourceDomain?: string;
  localPath?: string;
  chapterCount: number;
  imageCount: number;
  readProgressPage: number;
  scanStatus: "pending" | "missing" | "matched" | "imported" | "error";
}

export interface BookshelfMatch {
  title: string;
  directory: string;
  chapterCount: number;
  imageCount: number;
}

export interface ImportFavoritesResult {
  imported: number;
  matched: number;
  favorites: MangaConFavorite[];
}
