import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { importFavorites, listChapterPages, scanLocalChapters } from "./lib/api";
import { approvedDefaultPaths } from "./lib/defaults";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
  invoke: vi.fn(),
}));

vi.mock("./lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./lib/api")>()),
  importFavorites: vi.fn(),
  listChapterPages: vi.fn(),
  scanLocalChapters: vi.fn(),
}));

const importFavoritesMock = vi.mocked(importFavorites);
const listChapterPagesMock = vi.mocked(listChapterPages);
const scanLocalChaptersMock = vi.mocked(scanLocalChapters);

describe("App", () => {
  beforeEach(() => {
    importFavoritesMock.mockReset();
    listChapterPagesMock.mockReset();
    scanLocalChaptersMock.mockReset();
  });

  afterEach(() => {
    cleanup();
  });

  it("导入漫画控收藏后刷新仪表盘和书库", async () => {
    const user = userEvent.setup();
    importFavoritesMock.mockResolvedValue({
      imported: 2,
      matched: 1,
      favorites: [
        {
          id: "cp:hzzsddhhshct",
          name: "婚纱之中待到花火散去",
          location: "婚纱之中待到花火散去",
          tags: ["测试作者"],
          sourceUri: "cp:hzzsddhhshct",
          sourceScheme: "cp",
          sourceDomain: "www.2025copy.com",
          localPath: "E:\\书架\\婚纱之中待到花火散去",
          chapterCount: 24,
          imageCount: 720,
          readProgressPage: 0,
          scanStatus: "matched",
        },
        {
          id: "mg:37753",
          name: "航海士样本",
          location: "航海士样本",
          tags: [],
          sourceUri: "mg:37753",
          sourceScheme: "mg",
          sourceDomain: "www.manhuagui.com",
          chapterCount: 0,
          imageCount: 0,
          readProgressPage: 0,
          scanStatus: "missing",
        },
      ],
    });

    render(<App />);

    await user.click(screen.getByRole("button", { name: "导入漫画控收藏" }));

    await waitFor(() => {
      expect(importFavoritesMock).toHaveBeenCalledWith({
        favoritesJsonPath: approvedDefaultPaths.mangaConFavoritesJson,
        bookshelfRoot: approvedDefaultPaths.bookshelfRoot,
        databasePath: approvedDefaultPaths.databasePath,
      });
    });
    expect(
      screen.getAllByText("已导入 2 条收藏，匹配 1 本本地漫画").length,
    ).toBeGreaterThan(0);

    const favoritesMetric = screen.getByLabelText("收藏统计");
    expect(within(favoritesMetric).getByText("2")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "书库" }));

    expect(screen.getAllByText("婚纱之中待到花火散去").length).toBeGreaterThan(0);
    expect(screen.getByText("本地 24 章 / 720 页")).toBeInTheDocument();
    expect(screen.getByText("缺少本地目录")).toBeInTheDocument();
  });

  it("可从书库打开已匹配漫画并进入本地阅读器", async () => {
    const user = userEvent.setup();
    importFavoritesMock.mockResolvedValue({
      imported: 1,
      matched: 1,
      favorites: [
        {
          id: "cp:hzzsddhhshct",
          name: "婚纱之中待到花火散去",
          location: "婚纱之中待到花火散去",
          tags: [],
          sourceUri: "cp:hzzsddhhshct",
          sourceScheme: "cp",
          sourceDomain: "www.2025copy.com",
          localPath: "E:\\书架\\婚纱之中待到花火散去",
          chapterCount: 1,
          imageCount: 2,
          readProgressPage: 0,
          scanStatus: "matched",
        },
      ],
    });
    scanLocalChaptersMock.mockResolvedValue([
      {
        id: "cp:hzzsddhhshct::第01话",
        comicId: "cp:hzzsddhhshct",
        title: "第01话",
        path: "E:\\书架\\婚纱之中待到花火散去\\第01话",
        ordinal: 1,
        pageCount: 2,
        readProgressPage: 0,
        specialKind: "regular",
      },
    ]);
    listChapterPagesMock.mockResolvedValue([
      "E:\\书架\\婚纱之中待到花火散去\\第01话\\001.jpg",
      "E:\\书架\\婚纱之中待到花火散去\\第01话\\002.jpg",
    ]);

    render(<App />);

    await user.click(screen.getByRole("button", { name: "导入漫画控收藏" }));
    await user.click(screen.getByRole("button", { name: "书库" }));
    await user.click(
      screen.getByRole("button", { name: "阅读 婚纱之中待到花火散去" }),
    );

    expect(await screen.findByRole("heading", { name: "婚纱之中待到花火散去" })).toBeInTheDocument();
    expect(await screen.findByAltText("第01话 第 1 页")).toBeInTheDocument();
  });
});
