import { describe, expect, it } from "vitest";
import { sampleComic } from "./defaults";
import {
  chapterTitleFromId,
  coverProgress,
  lastReadChapterLabel,
} from "./progress";

describe("reading progress labels", () => {
  it("strips the comic id prefix from a chapter id", () => {
    expect(chapterTitleFromId("local:a", "local:a::第01话")).toBe("第01话");
    expect(chapterTitleFromId("local:a", "other")).toBeUndefined();
  });

  it("prefers stored chapter title over the latest chapter on disk", () => {
    expect(
      lastReadChapterLabel({
        ...sampleComic,
        lastReadChapterId: `${sampleComic.id}::第01话`,
        lastReadChapterTitle: "第01话",
      }),
    ).toBe("第01话");
  });

  it("estimates cover progress from the last-read chapter number", () => {
    expect(
      coverProgress({
        ...sampleComic,
        chapterCount: 10,
        lastReadAt: "2026-08-17 12:00:00",
        lastReadChapterTitle: "第04话",
        lastReadChapterPages: 1,
      }),
    ).toBe(0.4);
    expect(coverProgress(sampleComic)).toBe(0);
  });
});
