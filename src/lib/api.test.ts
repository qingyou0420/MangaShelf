import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  findMangaConWindows,
  getAutomationStatus,
  importFavorites,
  launchMangaCon,
  openMangaConFavorites,
  scanMangaConBadges,
} from "./api";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

describe("importFavorites", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("调用 Rust import_favorites command 并返回导入摘要", async () => {
    invokeMock.mockResolvedValue({
      imported: 1,
      matched: 0,
      favorites: [
        {
          id: "cp:ruoshijiechuyuheiye",
          name: "若世界處於黑夜",
          location: "若世界處於黑夜",
          sourceUri: "cp:ruoshijiechuyuheiye",
          sourceScheme: "cp",
          tags: ["むちまろ"],
          chapterCount: 0,
          imageCount: 0,
          readProgressPage: 0,
          scanStatus: "pending",
        },
      ],
    });

    const summary = await importFavorites({
      favoritesJsonPath: "E:\\漫画控\\20260528184624.mc3db.json",
      bookshelfRoot: "E:\\书架",
      databasePath: "companion.sqlite",
    });

    expect(invokeMock).toHaveBeenCalledWith("import_favorites", {
      favoritesJsonPath: "E:\\漫画控\\20260528184624.mc3db.json",
      bookshelfRoot: "E:\\书架",
      databasePath: "companion.sqlite",
    });
    expect(summary.imported).toBe(1);
    expect(summary.favorites[0].sourceUri).toBe("cp:ruoshijiechuyuheiye");
  });

  it("封装漫画控窗口、启动和自动化状态 commands", async () => {
    invokeMock
      .mockResolvedValueOnce([{ hwnd: 123, title: "漫画控 - 收藏" }])
      .mockResolvedValueOnce({ pid: 456 })
      .mockResolvedValueOnce({
        state: "waiting_refresh",
        message: "等待漫画控刷新收藏更新...",
        detectedBadges: 2,
        stableSamples: 3,
      });

    await expect(findMangaConWindows()).resolves.toEqual([
      { hwnd: 123, title: "漫画控 - 收藏" },
    ]);
    await expect(
      launchMangaCon({ executablePath: "E:\\漫画控\\MangaCon.exe" }),
    ).resolves.toEqual({ pid: 456 });
    await expect(getAutomationStatus()).resolves.toMatchObject({
      state: "waiting_refresh",
      detectedBadges: 2,
      stableSamples: 3,
    });

    expect(invokeMock).toHaveBeenCalledWith("find_mangacon_windows");
    expect(invokeMock).toHaveBeenCalledWith("launch_mangacon", {
      executablePath: "E:\\漫画控\\MangaCon.exe",
    });
    expect(invokeMock).toHaveBeenCalledWith("get_automation_status");
  });

  it("封装漫画控截图红点识别 command", async () => {
    invokeMock.mockResolvedValue({
      window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
      width: 850,
      height: 600,
      badges: [{ x: 164, y: 96 }],
    });

    const result = await scanMangaConBadges();

    expect(invokeMock).toHaveBeenCalledWith("scan_mangacon_badges");
    expect(result).toMatchObject({
      width: 850,
      height: 600,
      badges: [{ x: 164, y: 96 }],
    });
  });

  it("封装打开漫画控收藏夹 command", async () => {
    invokeMock.mockResolvedValue({
      window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
      clicked: { x: 212, y: 330 },
      width: 850,
      height: 600,
      badges: [{ x: 174, y: 95 }],
    });

    const result = await openMangaConFavorites();

    expect(invokeMock).toHaveBeenCalledWith("open_mangacon_favorites");
    expect(result).toMatchObject({
      clicked: { x: 212, y: 330 },
      badges: [{ x: 174, y: 95 }],
    });
  });
});
