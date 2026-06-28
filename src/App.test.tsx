import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { importFavorites } from "./lib/api";
import { approvedDefaultPaths } from "./lib/defaults";

vi.mock("./lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./lib/api")>()),
  importFavorites: vi.fn(),
}));

const importFavoritesMock = vi.mocked(importFavorites);

describe("App", () => {
  beforeEach(() => {
    importFavoritesMock.mockReset();
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
});
