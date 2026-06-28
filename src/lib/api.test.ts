import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  FAVORITE_UPDATE_RECOVERY_EVENT,
  findMangaConWindows,
  getAutomationStatus,
  importFavorites,
  listenFavoriteUpdateRecoveryEvents,
  launchMangaCon,
  openFirstUpdatedComic,
  openMangaConFavorites,
  restartMangaCon,
  scanDetailUpdates,
  scanFavoritesUpdates,
  scanMangaConBadges,
  triggerAllFavoriteUpdates,
  triggerAllFavoriteUpdatesWithRecovery,
  triggerDetailUpdateDownloadBatch,
  triggerFirstDetailUpdateDownload,
  triggerFavoriteUpdateBatch,
  triggerNextFavoriteUpdateDownload,
} from "./api";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

const { listenMock } = vi.hoisted(() => ({
  listenMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

describe("importFavorites", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
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

  it("封装打开首个更新漫画详情 command", async () => {
    invokeMock.mockResolvedValue({
      window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
      badge: { x: 174, y: 95 },
      clicked: { x: 117, y: 171 },
      width: 850,
      height: 600,
      remainingBadges: [],
    });

    const result = await openFirstUpdatedComic();

    expect(invokeMock).toHaveBeenCalledWith("open_first_updated_comic");
    expect(result).toMatchObject({
      badge: { x: 174, y: 95 },
      clicked: { x: 117, y: 171 },
      remainingBadges: [],
    });
  });

  it("封装重启漫画控 command", async () => {
    invokeMock.mockResolvedValue({ pid: 789 });

    const result = await restartMangaCon({
      executablePath: "E:\\漫画控\\MangaCon.exe",
    });

    expect(invokeMock).toHaveBeenCalledWith("restart_mangacon", {
      executablePath: "E:\\漫画控\\MangaCon.exe",
    });
    expect(result).toEqual({ pid: 789 });
  });

  it("封装扫描详情页章节更新 command", async () => {
    invokeMock.mockResolvedValue({
      window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
      width: 850,
      height: 600,
      badges: [{ x: 142, y: 516 }],
      scrollAttempts: 3,
    });

    const result = await scanDetailUpdates();

    expect(invokeMock).toHaveBeenCalledWith("scan_detail_updates");
    expect(result).toMatchObject({
      badges: [{ x: 142, y: 516 }],
      scrollAttempts: 3,
    });
  });

  it("封装滚动扫描收藏夹更新 command", async () => {
    invokeMock.mockResolvedValue({
      window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
      width: 850,
      height: 600,
      badges: [
        { x: 174, y: 96 },
        { x: 374, y: 296 },
      ],
      pages: [
        { scrollAttempts: 0, badges: [{ x: 174, y: 96 }] },
        { scrollAttempts: 2, badges: [{ x: 374, y: 296 }] },
      ],
      scrollAttempts: 2,
    });

    const result = await scanFavoritesUpdates();

    expect(invokeMock).toHaveBeenCalledWith("scan_favorites_updates");
    expect(result).toMatchObject({
      badges: [
        { x: 174, y: 96 },
        { x: 374, y: 296 },
      ],
      pages: [
        { scrollAttempts: 0, badges: [{ x: 174, y: 96 }] },
        { scrollAttempts: 2, badges: [{ x: 374, y: 296 }] },
      ],
      scrollAttempts: 2,
    });
  });

  it("封装触发首个详情页章节更新下载 command", async () => {
    invokeMock.mockResolvedValue({
      window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
      badge: { x: 151, y: 516 },
      clicked: { x: 203, y: 516 },
      width: 850,
      height: 600,
      remainingBadges: [],
      scrollAttempts: 0,
    });

    const result = await triggerFirstDetailUpdateDownload();

    expect(invokeMock).toHaveBeenCalledWith("trigger_first_detail_update_download");
    expect(result).toMatchObject({
      badge: { x: 151, y: 516 },
      clicked: { x: 203, y: 516 },
      scrollAttempts: 0,
    });
  });

  it("封装批量触发详情页章节更新下载 command", async () => {
    invokeMock.mockResolvedValue({
      requestedLimit: 20,
      processed: 2,
      stoppedReason: "no_update_badge",
      downloads: [
        {
          window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
          badge: { x: 151, y: 516 },
          clicked: { x: 203, y: 516 },
          width: 850,
          height: 600,
          remainingBadges: [{ x: 273, y: 516 }],
          scrollAttempts: 0,
        },
        {
          window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
          badge: { x: 273, y: 516 },
          clicked: { x: 325, y: 516 },
          width: 850,
          height: 600,
          remainingBadges: [],
          scrollAttempts: 0,
        },
      ],
    });

    const result = await triggerDetailUpdateDownloadBatch({ maxChapters: 20 });

    expect(invokeMock).toHaveBeenCalledWith(
      "trigger_detail_update_download_batch",
      {
        maxChapters: 20,
      },
    );
    expect(result).toMatchObject({
      requestedLimit: 20,
      processed: 2,
      stoppedReason: "no_update_badge",
      downloads: [
        { clicked: { x: 203, y: 516 } },
        { clicked: { x: 325, y: 516 } },
      ],
    });
  });

  it("封装处理下一个收藏更新 command", async () => {
    invokeMock.mockResolvedValue({
      comic: {
        window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
        badge: { x: 174, y: 96 },
        clicked: { x: 117, y: 172 },
        width: 850,
        height: 600,
        remainingBadges: [],
        scrollAttempts: 2,
      },
      download: {
        window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
        badge: { x: 151, y: 516 },
        clicked: { x: 203, y: 516 },
        width: 850,
        height: 600,
        remainingBadges: [],
        scrollAttempts: 4,
      },
      downloadBatch: {
        requestedLimit: 20,
        processed: 2,
        stoppedReason: "no_update_badge",
        downloads: [
          {
            window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
            badge: { x: 151, y: 516 },
            clicked: { x: 203, y: 516 },
            width: 850,
            height: 600,
            remainingBadges: [],
            scrollAttempts: 4,
          },
          {
            window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
            badge: { x: 273, y: 516 },
            clicked: { x: 325, y: 516 },
            width: 850,
            height: 600,
            remainingBadges: [],
            scrollAttempts: 0,
          },
        ],
      },
    });

    const result = await triggerNextFavoriteUpdateDownload();

    expect(invokeMock).toHaveBeenCalledWith("trigger_next_favorite_update_download");
    expect(result).toMatchObject({
      comic: {
        badge: { x: 174, y: 96 },
        clicked: { x: 117, y: 172 },
        scrollAttempts: 2,
      },
      download: {
        badge: { x: 151, y: 516 },
        clicked: { x: 203, y: 516 },
        scrollAttempts: 4,
      },
      downloadBatch: {
        processed: 2,
      },
    });
  });

  it("封装连续处理收藏更新 command", async () => {
    invokeMock.mockResolvedValue({
      requestedLimit: 3,
      processed: 2,
      downloadedChapters: 3,
      stoppedReason: "no_update_badge",
      skipped: [
        {
          comic: {
            window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
            badge: { x: 374, y: 296 },
            clicked: { x: 317, y: 372 },
            width: 850,
            height: 600,
            remainingBadges: [],
            scrollAttempts: 1,
          },
          reason: "detail_no_update_badge",
        },
      ],
      items: [
        {
          comic: {
            window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
            badge: { x: 174, y: 96 },
            clicked: { x: 117, y: 172 },
            width: 850,
            height: 600,
            remainingBadges: [],
            scrollAttempts: 0,
          },
          download: {
            window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
            badge: { x: 151, y: 516 },
            clicked: { x: 203, y: 516 },
            width: 850,
            height: 600,
            remainingBadges: [],
            scrollAttempts: 1,
          },
          downloadBatch: {
            requestedLimit: 20,
            processed: 3,
            stoppedReason: "no_update_badge",
            downloads: [
              {
                window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
                badge: { x: 151, y: 516 },
                clicked: { x: 203, y: 516 },
                width: 850,
                height: 600,
                remainingBadges: [],
                scrollAttempts: 1,
              },
              {
                window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
                badge: { x: 273, y: 516 },
                clicked: { x: 325, y: 516 },
                width: 850,
                height: 600,
                remainingBadges: [],
                scrollAttempts: 0,
              },
              {
                window: { hwnd: 123, title: "漫画控 v3.0.15.58 Beta4" },
                badge: { x: 395, y: 516 },
                clicked: { x: 447, y: 516 },
                width: 850,
                height: 600,
                remainingBadges: [],
                scrollAttempts: 0,
              },
            ],
          },
        },
      ],
    });

    const result = await triggerFavoriteUpdateBatch({ maxUpdates: 3 });

    expect(invokeMock).toHaveBeenCalledWith("trigger_favorite_update_batch", {
      maxUpdates: 3,
    });
    expect(result).toMatchObject({
      requestedLimit: 3,
      processed: 2,
      downloadedChapters: 3,
      stoppedReason: "no_update_badge",
      skipped: [
        {
          comic: {
            badge: { x: 374, y: 296 },
            clicked: { x: 317, y: 372 },
            scrollAttempts: 1,
          },
          reason: "detail_no_update_badge",
        },
      ],
    });
  });

  it("封装全量处理收藏更新 command", async () => {
    invokeMock.mockResolvedValue({
      requestedLimit: 500,
      processed: 447,
      downloadedChapters: 520,
      stoppedReason: "no_update_badge",
      skipped: [],
      items: [],
    });

    const result = await triggerAllFavoriteUpdates({ maxComics: 500 });

    expect(invokeMock).toHaveBeenCalledWith("trigger_all_favorite_updates", {
      maxComics: 500,
    });
    expect(result).toMatchObject({
      requestedLimit: 500,
      processed: 447,
      downloadedChapters: 520,
      stoppedReason: "no_update_badge",
    });
  });

  it("封装自动恢复全量处理收藏更新 command", async () => {
    invokeMock.mockResolvedValue({
      requestedLimit: 500,
      maxRestarts: 2,
      restarts: 1,
      processed: 447,
      downloadedChapters: 520,
      skippedCount: 3,
      stoppedReason: "completed",
      lastError: null,
      events: [],
      runs: [],
    });

    const result = await triggerAllFavoriteUpdatesWithRecovery({
      executablePath: "E:\\漫画控\\MangaCon.exe",
      maxComics: 500,
      maxRestarts: 2,
    });

    expect(invokeMock).toHaveBeenCalledWith(
      "trigger_all_favorite_updates_with_recovery",
      {
        executablePath: "E:\\漫画控\\MangaCon.exe",
        maxComics: 500,
        maxRestarts: 2,
      },
    );
    expect(result).toMatchObject({
      requestedLimit: 500,
      maxRestarts: 2,
      restarts: 1,
      processed: 447,
      downloadedChapters: 520,
      stoppedReason: "completed",
    });
  });

  it("监听自动恢复长跑实时事件", async () => {
    const unlisten = vi.fn();
    const handler = vi.fn();
    listenMock.mockResolvedValue(unlisten);

    const result = await listenFavoriteUpdateRecoveryEvents(handler);

    expect(listenMock).toHaveBeenCalledWith(
      FAVORITE_UPDATE_RECOVERY_EVENT,
      expect.any(Function),
    );

    const tauriHandler = listenMock.mock.calls[0][1];
    tauriHandler({
      payload: {
        kind: "started",
        message: "开始自动恢复长跑，目标 500 本",
        processed: 0,
        downloadedChapters: 0,
        skippedCount: 0,
        restarts: 0,
      },
    });

    expect(handler).toHaveBeenCalledWith({
      kind: "started",
      message: "开始自动恢复长跑，目标 500 本",
      processed: 0,
      downloadedChapters: 0,
      skippedCount: 0,
      restarts: 0,
    });
    expect(result).toBe(unlisten);
  });
});
