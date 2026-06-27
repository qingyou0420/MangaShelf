import type { CompanionPaths, MangaConFavorite } from "../lib/types";

export const approvedDefaultPaths: CompanionPaths = {
  mangaConExecutable: "E:\\漫画控\\MangaCon.exe",
  mangaConFavoritesJson: "E:\\漫画控\\20260528184624.mc3db.json",
  bookshelfRoot: "E:\\书架",
};

export const sampleFavorite: MangaConFavorite = {
  id: "cp:ruoshijiechuyuheiye",
  name: "若世界處於黑夜",
  location: "若世界處於黑夜",
  tags: ["むちまろ"],
  sourceUri: "cp:ruoshijiechuyuheiye",
  sourceScheme: "cp",
  chapterCount: 0,
  imageCount: 0,
  readProgressPage: 0,
  scanStatus: "pending",
};
