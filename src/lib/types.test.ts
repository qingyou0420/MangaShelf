import { describe, expect, it } from "vitest";
import { approvedDefaultPaths, sampleFavorite } from "../test/fixtures";

describe("shared MangaCon companion types", () => {
  it("keeps approved default Windows paths available to tests", () => {
    expect(approvedDefaultPaths).toEqual({
      mangaConExecutable: "E:\\漫画控\\MangaCon.exe",
      mangaConFavoritesJson: "E:\\漫画控\\20260528184624.mc3db.json",
      bookshelfRoot: "E:\\书架",
    });
  });

  it("models imported favorites with stable ids, titles, and tags", () => {
    expect(sampleFavorite).toMatchObject({
      id: "fav-001",
      title: "孤独摇滚",
      tags: ["音乐", "日常"],
    });
  });
});
