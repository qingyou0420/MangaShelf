import type { CompanionPaths, MangaConFavorite } from "./types";

export const approvedDefaultPaths: CompanionPaths = {
  mangaConExecutable: "E:\\漫画控\\MangaCon.exe",
  mangaConFavoritesJson: "E:\\漫画控\\20260528184624.mc3db.json",
  mangaConDatabase:
    "C:\\Users\\Administrator\\AppData\\Local\\MangaCon3\\MangaCon.dat",
  bookshelfRoot: "E:\\书架",
  databasePath: "E:\\书架\\mangacon-companion.sqlite",
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
