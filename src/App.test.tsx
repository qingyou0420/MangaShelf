import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import {
  ensureMangaConRunning,
  getMangaConTaskStatus,
  importFavorites,
  listChapterPages,
  loadImportedComics,
  listenFavoriteUpdateRecoveryEvents,
  queueMangaConUpdates,
  repairMangaConFailedTasks,
  scanLocalChapters,
  syncBookshelfMatches,
  triggerAllFavoriteUpdatesWithRecovery,
} from "./lib/api";
import { approvedDefaultPaths } from "./lib/defaults";
import type { MangaConFavorite } from "./lib/types";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
  invoke: vi.fn(),
}));

vi.mock("./lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./lib/api")>()),
  ensureMangaConRunning: vi.fn(),
  getMangaConTaskStatus: vi.fn(),
  importFavorites: vi.fn(),
  listChapterPages: vi.fn(),
  loadImportedComics: vi.fn(),
  listenFavoriteUpdateRecoveryEvents: vi.fn(),
  queueMangaConUpdates: vi.fn(),
  repairMangaConFailedTasks: vi.fn(),
  scanLocalChapters: vi.fn(),
  syncBookshelfMatches: vi.fn(),
  triggerAllFavoriteUpdatesWithRecovery: vi.fn(),
}));

const ensureMangaConRunningMock = vi.mocked(ensureMangaConRunning);
const getMangaConTaskStatusMock = vi.mocked(getMangaConTaskStatus);
const importFavoritesMock = vi.mocked(importFavorites);
const listChapterPagesMock = vi.mocked(listChapterPages);
const loadImportedComicsMock = vi.mocked(loadImportedComics);
const listenFavoriteUpdateRecoveryEventsMock = vi.mocked(
  listenFavoriteUpdateRecoveryEvents,
);
const queueMangaConUpdatesMock = vi.mocked(queueMangaConUpdates);
const repairMangaConFailedTasksMock = vi.mocked(repairMangaConFailedTasks);
const scanLocalChaptersMock = vi.mocked(scanLocalChapters);
const syncBookshelfMatchesMock = vi.mocked(syncBookshelfMatches);
const triggerAllFavoriteUpdatesWithRecoveryMock = vi.mocked(
  triggerAllFavoriteUpdatesWithRecovery,
);

const importedFavorites: MangaConFavorite[] = [
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
];

const syncedFavorites: MangaConFavorite[] = [
  {
    ...importedFavorites[0],
    localPath: "E:\\书架\\婚纱之中待到花火散去",
    coverPath: "E:\\书架\\.mangacon-companion\\covers\\31.png",
    chapterCount: 1,
    imageCount: 2,
    latestChapterTitle: "第01话",
    scanStatus: "matched",
    hasUpdate: true,
  },
  {
    ...importedFavorites[1],
    scanStatus: "missing",
    hasUpdate: false,
  },
];

describe("App", () => {
  beforeEach(() => {
    ensureMangaConRunningMock.mockReset();
    getMangaConTaskStatusMock.mockReset();
    importFavoritesMock.mockReset();
    listChapterPagesMock.mockReset();
    loadImportedComicsMock.mockReset();
    listenFavoriteUpdateRecoveryEventsMock.mockReset();
    queueMangaConUpdatesMock.mockReset();
    repairMangaConFailedTasksMock.mockReset();
    scanLocalChaptersMock.mockReset();
    syncBookshelfMatchesMock.mockReset();
    triggerAllFavoriteUpdatesWithRecoveryMock.mockReset();

    ensureMangaConRunningMock.mockResolvedValue({
      launched: false,
      launchPid: null,
      windows: [{ hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" }],
    });
    getMangaConTaskStatusMock.mockResolvedValue({
      totalTasks: 0,
      activeTasks: 0,
      failedTasks: 0,
      finishedTasks: 0,
      totalErrors: 0,
    });
    loadImportedComicsMock.mockResolvedValue([]);
    repairMangaConFailedTasksMock.mockResolvedValue({
      backupPath:
        "C:\\Users\\Administrator\\AppData\\Local\\MangaCon3\\MangaCon.dat.companion-backup-2",
      totalFailed: 0,
      requeued: 0,
      launched: false,
      launchPid: null,
      confirm: { found: false, clicked: false, dialogTitle: null },
      tasks: [],
    });
    syncBookshelfMatchesMock.mockResolvedValue({
      imported: 0,
      scanned: 0,
      matched: 0,
      missing: 0,
      orphaned: 0,
      favorites: [],
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
    expect(screen.getAllByText("漫画控已启动，正在检索收藏更新...").length).toBeGreaterThan(
      0,
    );
  });

  it("启动时加载已有伴侣索引且不扫描书架", async () => {
    loadImportedComicsMock.mockResolvedValue(syncedFavorites);

    render(<App />);

    await waitFor(() => {
      expect(loadImportedComicsMock).toHaveBeenCalledWith({
        databasePath: approvedDefaultPaths.databasePath,
      });
    });
    expect(syncBookshelfMatchesMock).not.toHaveBeenCalled();
    const favoritesMetric = screen.getByLabelText("收藏统计");
    expect(within(favoritesMetric).getByText("2")).toBeInTheDocument();
  });

  it("导入收藏不自动扫描书架，手动扫描后才同步本地索引和封面", async () => {
    const user = userEvent.setup();
    importFavoritesMock.mockResolvedValue({
      imported: 2,
      matched: 0,
      favorites: importedFavorites,
    });
    syncBookshelfMatchesMock.mockResolvedValue({
      imported: 2,
      scanned: 2,
      matched: 1,
      missing: 1,
      orphaned: 0,
      favorites: syncedFavorites,
    });

    render(<App />);

    await user.click(screen.getByRole("button", { name: "导入漫画控收藏" }));

    await waitFor(() => {
      expect(importFavoritesMock).toHaveBeenCalledWith({
        favoritesJsonPath: approvedDefaultPaths.mangaConFavoritesJson,
        databasePath: approvedDefaultPaths.databasePath,
      });
    });
    expect(syncBookshelfMatchesMock).not.toHaveBeenCalled();

    const favoritesMetric = screen.getByLabelText("收藏统计");
    expect(within(favoritesMetric).getByText("2")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "扫描本地书架" }));

    await waitFor(() => {
      expect(syncBookshelfMatchesMock).toHaveBeenCalledWith({
        bookshelfRoot: approvedDefaultPaths.bookshelfRoot,
        databasePath: approvedDefaultPaths.databasePath,
        mangaConDatabasePath: approvedDefaultPaths.mangaConDatabase,
      });
    });
    expect(
      screen.getAllByText(
        "书架扫描完成：收藏 2 条，匹配 1 条，缺失 1 条，暂未匹配历史文件夹 0 个",
      ).length,
    ).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "书库" }));

    expect(screen.getAllByText("婚纱之中待到花火散去").length).toBeGreaterThan(0);
    expect(screen.getAllByText("已匹配本地").length).toBeGreaterThan(0);
    expect(screen.getByAltText("婚纱之中待到花火散去 封面")).toHaveAttribute(
      "src",
      expect.stringContaining("31.png"),
    );
  });

  it("可从书库打开已匹配漫画并进入本地阅读器", async () => {
    const user = userEvent.setup();
    importFavoritesMock.mockResolvedValue({
      imported: 1,
      matched: 0,
      favorites: [importedFavorites[0]],
    });
    syncBookshelfMatchesMock.mockResolvedValue({
      imported: 1,
      scanned: 1,
      matched: 1,
      missing: 0,
      orphaned: 0,
      favorites: [syncedFavorites[0]],
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
    await user.click(screen.getByRole("button", { name: "扫描本地书架" }));
    await user.click(screen.getByRole("button", { name: "书库" }));
    await user.click(
      screen.getByRole("button", { name: "阅读 婚纱之中待到花火散去" }),
    );

    expect(
      await screen.findByRole("heading", { name: "婚纱之中待到花火散去" }),
    ).toBeInTheDocument();
    expect(await screen.findByAltText("第01话 第 1 页")).toBeInTheDocument();
  });

  it("一键更新收藏不会触发本地书架扫描", async () => {
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
        companionDatabasePath: approvedDefaultPaths.databasePath,
        maxUpdates: 500,
      });
    });
    expect(
      ensureMangaConRunningMock.mock.invocationCallOrder[1],
    ).toBeLessThan(queueMangaConUpdatesMock.mock.invocationCallOrder[0]);
    expect(syncBookshelfMatchesMock).not.toHaveBeenCalled();
    expect(
      screen.getAllByText(
        "已加入漫画控下载队列 33 话，跳过已有任务 1 话，清理更新标记 34 处",
      ).length,
    ).toBeGreaterThan(0);
  });

  it("从仪表盘手动修复失败图片会重新加入漫画控修复队列", async () => {
    const user = userEvent.setup();
    repairMangaConFailedTasksMock.mockResolvedValue({
      backupPath:
        "C:\\Users\\Administrator\\AppData\\Local\\MangaCon3\\MangaCon.dat.companion-backup-2",
      totalFailed: 2,
      requeued: 2,
      launched: true,
      launchPid: 789,
      confirm: { found: true, clicked: true, dialogTitle: "漫画控" },
      tasks: [],
    });

    render(<App />);

    await user.click(screen.getByRole("button", { name: "修复失败图片" }));

    await waitFor(() => {
      expect(repairMangaConFailedTasksMock).toHaveBeenCalledWith({
        mangaConDatabasePath: approvedDefaultPaths.mangaConDatabase,
        executablePath: approvedDefaultPaths.mangaConExecutable,
        maxTasks: 200,
      });
    });
    expect(
      screen.getAllByText("已将 2 个失败任务重新加入漫画控修复队列").length,
    ).toBeGreaterThan(0);
  });
});
