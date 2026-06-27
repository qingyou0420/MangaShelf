import type { CompanionPaths, MangaConFavorite } from "../lib/types";

export const approvedDefaultPaths: CompanionPaths = {
  mangaConExecutable: "E:\\漫画控\\MangaCon.exe",
  mangaConFavoritesJson: "E:\\漫画控\\20260528184624.mc3db.json",
  bookshelfRoot: "E:\\书架",
};

export const sampleFavorite: MangaConFavorite = {
  id: "fav-001",
  title: "孤独摇滚",
  tags: ["音乐", "日常"],
  author: "はまじあき",
  favoritedAt: "2026-05-28T18:46:24Z",
};
