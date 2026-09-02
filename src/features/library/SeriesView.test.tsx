import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LibraryComic, LocalChapter } from "../../lib/types";
import { SeriesView } from "./SeriesView";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
}));

const comic: LibraryComic = {
  id: "local:a",
  name: "已匹配书",
  location: "已匹配书",
  tags: ["标签"],
  author: "作者",
  chapterCount: 2,
  imageCount: 20,
  readProgressPage: 3,
  lastReadChapterId: "local:a::第01话",
  lastReadChapterTitle: "第01话",
  lastReadAt: "2026-08-17 12:00:00",
  scanStatus: "matched",
  localPath: "E:\\书架\\已匹配书",
  favorited: false,
  readingDirection: "ltr",
  fitMode: "contain",
};

const chapters: LocalChapter[] = [
  {
    id: "local:a::第01话",
    comicId: "local:a",
    title: "第01话",
    path: "E:\\书架\\已匹配书\\第01话",
    ordinal: 1,
    pageCount: 10,
    readProgressPage: 3,
    specialKind: "regular",
  },
  {
    id: "local:a::第02话",
    comicId: "local:a",
    title: "第02话",
    path: "E:\\书架\\已匹配书\\第02话",
    ordinal: 2,
    pageCount: 10,
    readProgressPage: 0,
    specialKind: "regular",
  },
];

afterEach(() => {
  cleanup();
});

describe("SeriesView", () => {
  it("shows metadata, continues from last chapter, and opens a specific chapter", async () => {
    const user = userEvent.setup();
    const onRead = vi.fn();
    const onBack = vi.fn();
    render(
      <SeriesView
        comic={comic}
        chapters={chapters}
        onBack={onBack}
        onRead={onRead}
      />,
    );

    expect(screen.getByRole("heading", { name: "已匹配书" })).toBeInTheDocument();
    expect(screen.getByText("作者")).toBeInTheDocument();
    expect(screen.getByText(/上次读到 第01话/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "继续阅读" }));
    expect(onRead).toHaveBeenCalledWith(comic);

    await user.click(screen.getByRole("button", { name: /第02话/ }));
    expect(onRead).toHaveBeenLastCalledWith(comic, chapters[1]);

    await user.click(screen.getByRole("button", { name: "返回" }));
    expect(onBack).toHaveBeenCalled();
  });

  it("filters chapters by kind", async () => {
    const user = userEvent.setup();
    render(
      <SeriesView
        comic={comic}
        chapters={[
          ...chapters,
          {
            id: "local:a::第01卷",
            comicId: "local:a",
            title: "第01卷",
            path: "E:\\书架\\已匹配书\\第01卷",
            ordinal: 1,
            pageCount: 80,
            readProgressPage: 0,
            specialKind: "volume",
          },
        ]}
      />,
    );

    expect(screen.getByRole("button", { name: /第01卷/ })).toBeInTheDocument();
    await user.click(screen.getByLabelText("章节类型"));
    await user.click(screen.getByRole("option", { name: "卷" }));
    expect(screen.getByRole("button", { name: /第01卷/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /第01话/ })).not.toBeInTheDocument();
  });
});
