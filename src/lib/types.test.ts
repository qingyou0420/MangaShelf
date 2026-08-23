import { describe, expect, it } from "vitest";
import {
  cacheRootForPath,
  databasePathFor,
  defaultLibraryPaths,
  sampleComic,
} from "./defaults";

describe("local library types", () => {
  it("keeps default local bookshelf paths", () => {
    expect(defaultLibraryPaths).toEqual({
      bookshelfRoot: "E:\\书架",
      databasePath: "E:\\书架\\manga-library.sqlite",
      extraRoots: [],
    });
  });

  it("models a local comic without platform source fields", () => {
    expect(sampleComic).toMatchObject({
      name: "若世界處於黑夜",
      scanStatus: "matched",
      favorited: false,
      readingDirection: "ltr",
    });
    expect(sampleComic).not.toHaveProperty("sourceUri");
    expect(sampleComic).not.toHaveProperty("hasUpdate");
  });

  it("derives the index path from a bookshelf folder", () => {
    expect(databasePathFor("D:\\Comics\\")).toBe("D:\\Comics\\manga-library.sqlite");
  });

  it("picks extra bookshelf root for cache paths", () => {
    expect(
      cacheRootForPath("D:\\Extra\\书\\第01话\\001.jpg", {
        bookshelfRoot: "E:\\书架",
        databasePath: "E:\\书架\\manga-library.sqlite",
        extraRoots: ["D:\\Extra"],
      }),
    ).toBe("D:\\Extra");
  });
});
