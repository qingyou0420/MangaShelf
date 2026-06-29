import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import {
  ensureMangaConRunning,
  importFavorites,
  listChapterPages,
  listenFavoriteUpdateRecoveryEvents,
  queueMangaConUpdates,
  scanLocalChapters,
  triggerAllFavoriteUpdatesWithRecovery,
} from "./lib/api";
import { approvedDefaultPaths } from "./lib/defaults";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
  invoke: vi.fn(),
}));

vi.mock("./lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./lib/api")>()),
  ensureMangaConRunning: vi.fn(),
  importFavorites: vi.fn(),
  listChapterPages: vi.fn(),
  listenFavoriteUpdateRecoveryEvents: vi.fn(),
  queueMangaConUpdates: vi.fn(),
  scanLocalChapters: vi.fn(),
  triggerAllFavoriteUpdatesWithRecovery: vi.fn(),
}));

const ensureMangaConRunningMock = vi.mocked(ensureMangaConRunning);
const importFavoritesMock = vi.mocked(importFavorites);
const listChapterPagesMock = vi.mocked(listChapterPages);
const listenFavoriteUpdateRecoveryEventsMock = vi.mocked(
  listenFavoriteUpdateRecoveryEvents,
);
const scanLocalChaptersMock = vi.mocked(scanLocalChapters);
const queueMangaConUpdatesMock = vi.mocked(queueMangaConUpdates);
const triggerAllFavoriteUpdatesWithRecoveryMock = vi.mocked(
  triggerAllFavoriteUpdatesWithRecovery,
);

describe("App", () => {
  beforeEach(() => {
    ensureMangaConRunningMock.mockReset();
    importFavoritesMock.mockReset();
    listChapterPagesMock.mockReset();
    listenFavoriteUpdateRecoveryEventsMock.mockReset();
    queueMangaConUpdatesMock.mockReset();
    scanLocalChaptersMock.mockReset();
    triggerAllFavoriteUpdatesWithRecoveryMock.mockReset();
    ensureMangaConRunningMock.mockResolvedValue({
      launched: false,
      launchPid: null,
      windows: [{ hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" }],
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("启动伴侣后自动确保漫画控本体运行", async () => {
    ensureMangaConRunningMock.mockResolvedValueOnce({
      launched: true,
      launchPid: 456,
      windows: [],
    });

    render(<App />);

    await waitFor(() => {
      expect(ensureMangaConRunningMock).toHaveBeenCalledWith({
        executablePath: approvedDefaultPaths.mangaConExecutable,
      });
    });
    expect(
      screen.getAllByText("漫画控已启动，正在检索收藏更新...").length,
    ).toBeGreaterThan(0);
  });

  it("导入漫画控收藏后刷新仪表盘和书库", async () => {
    const user = userEvent.setup();
    importFavoritesMock.mockResolvedValue({
      imported: 2,
      matched: 0,
      favorites: [
        {
          id: "cp:hzzsddhhshct",
          name: "婚纱之中待到花火散去",
          location: "婚纱之中待到花火散去",
          tags: ["测试作者"],
          sourceUri: "cp:hzzsddhhshct",
          sourceScheme: "cp",
          sourceDomain: "www.2025copy.com",
          chapterCount: 0,
          imageCount: 0,
          readProgressPage: 0,
          scanStatus: "imported",
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
          scanStatus: "imported",
        },
      ],
    });

    render(<App />);

    await user.click(screen.getByRole("button", { name: "导入漫画控收藏" }));

    await waitFor(() => {
      expect(importFavoritesMock).toHaveBeenCalledWith({
        favoritesJsonPath: approvedDefaultPaths.mangaConFavoritesJson,
        databasePath: approvedDefaultPaths.databasePath,
      });
    });
    expect(
      screen.getAllByText("已导入 2 条收藏，书架匹配稍后执行").length,
    ).toBeGreaterThan(0);

    const favoritesMetric = screen.getByLabelText("收藏统计");
    expect(within(favoritesMetric).getByText("2")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "书库" }));

    expect(screen.getAllByText("婚纱之中待到花火散去").length).toBeGreaterThan(0);
    expect(screen.getAllByText("已导入").length).toBeGreaterThan(0);
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

  it("从仪表盘一键更新收藏会先确保漫画控运行再写入数据库队列", async () => {
    const user = userEvent.setup();
    queueMangaConUpdatesMock.mockResolvedValue({
      backupPath:
        "C:\\Users\\Administrator\\AppData\\Local\\MangaCon3\\MangaCon.dat.companion-backup-1",
      totalUpdates: 34,
      queued: 33,
      skippedExisting: 1,
      clearedUpdateMarkers: 34,
      launched: true,
      confirm: { found: true, clicked: true, dialogTitle: "漫画控" },
      tasks: [],
    });

    render(<App />);

    await waitFor(() => {
      expect(ensureMangaConRunningMock).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByRole("button", { name: "一键更新收藏" }));

    await waitFor(() => {
      expect(ensureMangaConRunningMock).toHaveBeenCalledTimes(2);
      expect(queueMangaConUpdatesMock).toHaveBeenCalledWith({
        mangaConDatabasePath: approvedDefaultPaths.mangaConDatabase,
        executablePath: approvedDefaultPaths.mangaConExecutable,
        maxUpdates: 500,
      });
    });
    expect(
      ensureMangaConRunningMock.mock.invocationCallOrder[1],
    ).toBeLessThan(queueMangaConUpdatesMock.mock.invocationCallOrder[0]);
    expect(
      screen.getAllByText(
        "已加入漫画控下载队列 33 话，跳过已有任务 1 话，清理更新标记 34 处",
      ).length,
    ).toBeGreaterThan(0);
  });
});
