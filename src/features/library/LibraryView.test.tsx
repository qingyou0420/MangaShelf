import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LibraryComic } from "../../lib/types";
import { LibraryView } from "./LibraryView";

const comics: LibraryComic[] = [
  {
    id: "a",
    name: "已匹配书",
    location: "已匹配书",
    tags: [],
    chapterCount: 3,
    imageCount: 10,
    readProgressPage: 0,
    scanStatus: "matched",
    localPath: "E:\\书架\\已匹配书",
    favorited: false,
    readingDirection: "ltr",
    fitMode: "contain",
  },
  {
    id: "b",
    name: "收藏书",
    location: "收藏书",
    tags: ["标签"],
    author: "作者",
    chapterCount: 1,
    imageCount: 2,
    readProgressPage: 0,
    scanStatus: "matched",
    localPath: "E:\\书架\\收藏书",
    favorited: true,
    lastReadAt: "2026-08-17 10:00:00",
    readingDirection: "ltr",
    fitMode: "contain",
  },
];

afterEach(() => {
  cleanup();
  window.localStorage.clear();
});

describe("LibraryView toolbar", () => {
  it("scans local bookshelf and filters favorites", async () => {
    const user = userEvent.setup();
    const onScanLibrary = vi.fn();
    render(
      <LibraryView
        comics={comics}
        onScanLibrary={onScanLibrary}
        baselineCompleted
      />,
    );

    expect(
      document.querySelector(".library-tile.selected"),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "扫描书架" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "导入现有书库" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "有更新" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "扫描书架" }));
    expect(onScanLibrary).toHaveBeenCalled();

    expect(screen.getByRole("button", { name: "倒序排列" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "倒序排列" }));
    expect(screen.getByRole("button", { name: "切换为正序" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "收藏" }));
    expect(screen.getAllByText("收藏书").length).toBeGreaterThan(0);
    expect(screen.queryByText("已匹配书")).not.toBeInTheDocument();
  });

  it("opens a series from the cover and continues the last-read title", async () => {
    const user = userEvent.setup();
    const onOpenSeries = vi.fn();
    const onReadComic = vi.fn();
    render(
      <LibraryView
        comics={comics}
        onOpenSeries={onOpenSeries}
        onReadComic={onReadComic}
      />,
    );

    expect(screen.getByText("继续阅读")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "查看 已匹配书" }));
    expect(onOpenSeries).toHaveBeenCalledWith(comics[0]);
    await user.click(screen.getByRole("button", { name: "继续阅读" }));
    expect(onReadComic).toHaveBeenCalledWith(comics[1]);
  });

  it("marks updated titles on the cover and defaults to recent-update sort", () => {
    render(
      <LibraryView
        comics={[
          comics[1],
          {
            ...comics[0],
            shelfUpdatedAt: "2026-08-21 12:00:00",
            shelfUpdateNote: "更新了2话",
          },
        ]}
        onOpenSeries={() => undefined}
      />,
    );
    expect(screen.queryByRole("region", { name: "最近更新" })).not.toBeInTheDocument();
    expect(screen.getByLabelText("排序")).toHaveTextContent("最近更新");
    expect(screen.getByText("更新了2话")).toBeInTheDocument();
    const titles = Array.from(document.querySelectorAll(".library-tile h2")).map(
      (node) => node.textContent,
    );
    expect(titles[0]).toBe("已匹配书");
    expect(titles[1]).toBe("收藏书");
  });

  it("offers a one-time import before the library baseline exists", () => {
    render(
      <LibraryView comics={[]} onScanLibrary={() => undefined} />,
    );
    expect(screen.getByRole("button", { name: "导入现有书库" })).toBeInTheDocument();
    expect(screen.getByText(/不会改动或删除文件/)).toBeInTheDocument();
    expect(screen.queryByText(/首次导入会索引全部已有漫画/)).not.toBeInTheDocument();
  });

  it("shows a one-line baseline hint when titles exist but the baseline does not", () => {
    render(
      <LibraryView comics={comics} onScanLibrary={() => undefined} />,
    );
    expect(screen.getByText("首次导入只建立索引，不会标记更新。")).toBeInTheDocument();
  });

  it("filters by tag when a tag label is clicked", async () => {
    const user = userEvent.setup();
    render(<LibraryView comics={comics} onOpenSeries={() => undefined} />);

    await user.click(screen.getByRole("link", { name: "标签" }));
    expect(screen.getAllByText("收藏书").length).toBeGreaterThan(0);
    expect(screen.queryByText("已匹配书")).not.toBeInTheDocument();
  });

  it("filters by author when the author label is clicked", async () => {
    const user = userEvent.setup();
    render(<LibraryView comics={comics} onOpenSeries={() => undefined} />);

    await user.click(screen.getByRole("link", { name: "作者" }));
    expect(screen.getAllByText("收藏书").length).toBeGreaterThan(0);
    expect(screen.queryByText("已匹配书")).not.toBeInTheDocument();
  });

  it("shows a missing-folder empty state", () => {
    render(
      <LibraryView
        comics={[]}
        bookshelfMissing
        onPickBookshelf={() => undefined}
      />,
    );
    expect(screen.getByText("找不到书架文件夹")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "选择书架文件夹" })).toBeInTheDocument();
  });

  it("explains empty library without mentioning external platforms", () => {
    render(<LibraryView comics={[]} />);
    expect(screen.getByText("书库还是空的")).toBeInTheDocument();
    expect(screen.getByText(/不会改动或删除文件/)).toBeInTheDocument();
    expect(screen.queryByText(/漫画控/)).not.toBeInTheDocument();
  });
});
