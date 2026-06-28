import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { approvedDefaultPaths } from "../../test/fixtures";
import type {
  FavoriteUpdateRecoveryEvent,
  RecoveringFavoriteUpdateResult,
} from "../../lib/types";
import type { AutomationService } from "./AutomationView";
import { AutomationView } from "./AutomationView";
import {
  buildRecoveryRunHistoryRecord,
  loadRecoveryRunHistory,
  saveRecoveryRunHistory,
} from "./recoveryHistory";

afterEach(() => {
  cleanup();
  localStorage.clear();
});

describe("AutomationView", () => {
  it("展示等待刷新状态、稳定样本和流程时间线", () => {
    render(
      <AutomationView
        status={{
          state: "waiting_refresh",
          message: "等待漫画控刷新收藏更新...",
          detectedBadges: 2,
          stableSamples: 3,
        }}
      />,
    );

    expect(screen.getByText("等待漫画控刷新收藏更新...")).toBeInTheDocument();
    expect(screen.getByText("稳定样本")).toBeInTheDocument();
    expect(screen.getByText("红点 2")).toBeInTheDocument();
    expect(screen.getByText("稳定样本 3")).toBeInTheDocument();
    expect(screen.getByText("流程时间线")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "查看任务队列" }),
    ).toBeInTheDocument();
  });

  it("能查找漫画控窗口、启动漫画控并刷新自动化状态", async () => {
    const user = userEvent.setup();
    const service = {
      findWindows: async () => [
        { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
      ],
      launch: async () => ({ pid: 2345 }),
      restart: async () => ({ pid: 7890 }),
      getStatus: async () => ({
        state: "waiting_refresh" as const,
        message: "等待漫画控刷新收藏更新...",
        detectedBadges: 5,
        stableSamples: 4,
      }),
      scanBadges: async () => ({
        window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
        width: 850,
        height: 600,
        badges: [
          { x: 164, y: 96 },
          { x: 360, y: 96 },
        ],
      }),
      scanFavoritesUpdates: async () => ({
        window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
        width: 850,
        height: 600,
        badges: [
          { x: 174, y: 96 },
          { x: 374, y: 296 },
          { x: 574, y: 96 },
        ],
        pages: [
          { scrollAttempts: 0, badges: [{ x: 174, y: 96 }] },
          {
            scrollAttempts: 2,
            badges: [
              { x: 374, y: 296 },
              { x: 574, y: 96 },
            ],
          },
        ],
        scrollAttempts: 2,
      }),
      openFavorites: async () => ({
        window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
        clicked: { x: 212, y: 330 },
        width: 850,
        height: 600,
        badges: [{ x: 174, y: 95 }],
      }),
      openFirstUpdatedComic: async () => ({
        window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
        badge: { x: 174, y: 95 },
        clicked: { x: 117, y: 171 },
        width: 850,
        height: 600,
        remainingBadges: [],
      }),
      scanDetailUpdates: async () => ({
        window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
        width: 850,
        height: 600,
        badges: [{ x: 142, y: 516 }],
        scrollAttempts: 3,
      }),
      triggerFirstDetailUpdateDownload: async () => ({
        window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
        badge: { x: 151, y: 516 },
        clicked: { x: 203, y: 516 },
        width: 850,
        height: 600,
        remainingBadges: [],
        scrollAttempts: 0,
      }),
      triggerDetailUpdateDownloadBatch: async () => ({
        requestedLimit: 20,
        processed: 2,
        stoppedReason: "no_update_badge" as const,
        downloads: [
          {
            window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
            badge: { x: 151, y: 516 },
            clicked: { x: 203, y: 516 },
            width: 850,
            height: 600,
            remainingBadges: [],
            scrollAttempts: 0,
          },
          {
            window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
            badge: { x: 273, y: 516 },
            clicked: { x: 325, y: 516 },
            width: 850,
            height: 600,
            remainingBadges: [],
            scrollAttempts: 0,
          },
        ],
      }),
      triggerNextFavoriteUpdateDownload: async () => ({
        comic: {
          window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
          badge: { x: 174, y: 96 },
          clicked: { x: 117, y: 172 },
          width: 850,
          height: 600,
          remainingBadges: [],
          scrollAttempts: 2,
        },
        download: {
          window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
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
          stoppedReason: "no_update_badge" as const,
          downloads: [
            {
              window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
              badge: { x: 151, y: 516 },
              clicked: { x: 203, y: 516 },
              width: 850,
              height: 600,
              remainingBadges: [],
              scrollAttempts: 4,
            },
            {
              window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
              badge: { x: 273, y: 516 },
              clicked: { x: 325, y: 516 },
              width: 850,
              height: 600,
              remainingBadges: [],
              scrollAttempts: 0,
            },
          ],
        },
      }),
      triggerFavoriteUpdateBatch: async () => ({
        requestedLimit: 3,
        processed: 2,
        downloadedChapters: 3,
        stoppedReason: "no_update_badge" as const,
        skipped: [
          {
            comic: {
              window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
              badge: { x: 374, y: 296 },
              clicked: { x: 317, y: 372 },
              width: 850,
              height: 600,
              remainingBadges: [],
              scrollAttempts: 1,
            },
            reason: "detail_no_update_badge" as const,
          },
        ],
        items: [
          {
            comic: {
              window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
              badge: { x: 174, y: 96 },
              clicked: { x: 117, y: 172 },
              width: 850,
              height: 600,
              remainingBadges: [],
              scrollAttempts: 0,
            },
            download: {
              window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
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
              stoppedReason: "no_update_badge" as const,
              downloads: [
                {
                  window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
                  badge: { x: 151, y: 516 },
                  clicked: { x: 203, y: 516 },
                  width: 850,
                  height: 600,
                  remainingBadges: [],
                  scrollAttempts: 1,
                },
                {
                  window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
                  badge: { x: 273, y: 516 },
                  clicked: { x: 325, y: 516 },
                  width: 850,
                  height: 600,
                  remainingBadges: [],
                  scrollAttempts: 0,
                },
                {
                  window: { hwnd: 1001, title: "漫画控 v3.0.15.58 Beta4" },
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
      }),
      triggerAllFavoriteUpdates: async () => ({
        requestedLimit: 500,
        processed: 447,
        downloadedChapters: 520,
        stoppedReason: "no_update_badge" as const,
        skipped: [],
        items: [],
      }),
      triggerAllFavoriteUpdatesWithRecovery: async () => ({
        requestedLimit: 500,
        maxRestarts: 2,
        restarts: 1,
        processed: 447,
        downloadedChapters: 520,
        skippedCount: 3,
        stoppedReason: "completed" as const,
        lastError: "漫画控窗口无响应",
        events: [
          {
            kind: "started" as const,
            message: "开始自动恢复长跑，目标 500 本",
            processed: 0,
            downloadedChapters: 0,
            skippedCount: 0,
            restarts: 0,
          },
          {
            kind: "error" as const,
            message: "漫画控窗口无响应",
            processed: 120,
            downloadedChapters: 160,
            skippedCount: 1,
            restarts: 0,
          },
          {
            kind: "restarted" as const,
            message: "漫画控已重启，等待红点刷新（1/2）",
            processed: 120,
            downloadedChapters: 160,
            skippedCount: 1,
            restarts: 1,
          },
          {
            kind: "completed" as const,
            message: "自动恢复长跑完成",
            processed: 447,
            downloadedChapters: 520,
            skippedCount: 3,
            restarts: 1,
          },
        ],
        runs: [],
      }),
      listenFavoriteUpdateRecoveryEvents: async () => () => {},
    };

    render(
      <AutomationView
        paths={approvedDefaultPaths}
        service={service}
        status={{
          state: "waiting_refresh",
          message: "等待漫画控刷新收藏更新...",
          detectedBadges: 0,
          stableSamples: 1,
        }}
      />,
    );

    await user.click(screen.getByRole("button", { name: "查找漫画控窗口" }));

    expect(await screen.findByText("已发现 1 个窗口")).toBeInTheDocument();
    expect(screen.getByText("漫画控 v3.0.15.58 Beta4")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "启动漫画控" }));

    expect(await screen.findByText("已启动 PID 2345")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "重启漫画控" }));

    expect(await screen.findByText("已启动 PID 7890")).toBeInTheDocument();
    expect(screen.getByText("漫画控已重启，等待刷新红点")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "刷新状态" }));

    expect(await screen.findByText("红点 5")).toBeInTheDocument();
    expect(screen.getByText("稳定样本 4")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "识别红点" }));

    expect(await screen.findByText("截图 850x600")).toBeInTheDocument();
    expect(screen.getByText("识别红点 2")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "滚动扫描收藏更新" }));

    expect(await screen.findByText("收藏红点 3")).toBeInTheDocument();
    expect(screen.getByText("收藏页数 2")).toBeInTheDocument();
    expect(screen.getByText("收藏滚动 2 次")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "打开收藏夹" }));

    expect(await screen.findByText("点击 212,330")).toBeInTheDocument();
    expect(screen.getByText("识别红点 1")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "打开首个更新" }));

    expect(await screen.findByText("更新红点 174,95")).toBeInTheDocument();
    expect(screen.getByText("打开详情 117,171")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "扫描详情更新" }));

    expect(await screen.findByText("详情红点 1")).toBeInTheDocument();
    expect(screen.getByText("滚动扫描 3 次")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "下载首个章节更新" }));

    expect(await screen.findByText("章节红点 151,516")).toBeInTheDocument();
    expect(screen.getByText("章节点击 203,516")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "下载详情全部更新" }));

    expect(await screen.findByText("详情批量章节 2/20")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "处理下一个收藏更新" }));

    expect(await screen.findByText("收藏点击 117,172")).toBeInTheDocument();
    expect(screen.getByText("收藏滚动定位 2 次")).toBeInTheDocument();
    expect(screen.getByText("更新下载点击 203,516")).toBeInTheDocument();
    expect(screen.getByText("收藏章节下载 2")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "连续处理 3 个更新" }));

    expect(await screen.findByText("批量处理 2/3")).toBeInTheDocument();
    expect(screen.getByText("章节下载 3")).toBeInTheDocument();
    expect(screen.getByText("跳过收藏 1")).toBeInTheDocument();
    expect(screen.getByText("停止原因 no_update_badge")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "更新全部收藏" }));

    expect(await screen.findByText("全部收藏处理 447/500")).toBeInTheDocument();
    expect(screen.getByText("全部章节下载 520")).toBeInTheDocument();
    expect(screen.getByText("全部跳过收藏 0")).toBeInTheDocument();
    expect(screen.getByText("全部停止原因 no_update_badge")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "自动恢复更新全部" }));

    expect(await screen.findByText("恢复处理 447/500")).toBeInTheDocument();
    expect(screen.getByText("恢复章节下载 520")).toBeInTheDocument();
    expect(screen.getByText("恢复重启 1/2")).toBeInTheDocument();
    expect(screen.getByText("恢复跳过收藏 3")).toBeInTheDocument();
    expect(screen.getByText("恢复停止原因 completed")).toBeInTheDocument();
    expect(screen.getByText("自动更新长跑")).toBeInTheDocument();
    expect(screen.getByText("跳过漫画").closest("div")).toHaveTextContent("3");
    expect(screen.getByText("自动恢复长跑完成")).toBeInTheDocument();
    expect(
      screen.getByText("已处理 447 本，下载 520 话，跳过 3 本，重启 1 次"),
    ).toBeInTheDocument();
    expect(screen.getByText("上次异常")).toBeInTheDocument();
    expect(screen.getAllByText("漫画控窗口无响应").length).toBeGreaterThan(0);
  });

  it("自动恢复长跑运行中实时追加事件日志", async () => {
    const user = userEvent.setup();
    let liveHandler: ((event: FavoriteUpdateRecoveryEvent) => void) | undefined;
    let resolveRun!: (value: RecoveringFavoriteUpdateResult) => void;
    const runPromise = new Promise<RecoveringFavoriteUpdateResult>((resolve) => {
      resolveRun = resolve;
    });
    const unlisten = vi.fn();
    const service: AutomationService = {
      findWindows: async () => [],
      launch: async () => {
        throw new Error("not used");
      },
      restart: async () => {
        throw new Error("not used");
      },
      getStatus: async () => {
        throw new Error("not used");
      },
      scanBadges: async () => {
        throw new Error("not used");
      },
      scanFavoritesUpdates: async () => {
        throw new Error("not used");
      },
      openFavorites: async () => {
        throw new Error("not used");
      },
      openFirstUpdatedComic: async () => {
        throw new Error("not used");
      },
      scanDetailUpdates: async () => {
        throw new Error("not used");
      },
      triggerFirstDetailUpdateDownload: async () => {
        throw new Error("not used");
      },
      triggerDetailUpdateDownloadBatch: async () => {
        throw new Error("not used");
      },
      triggerNextFavoriteUpdateDownload: async () => {
        throw new Error("not used");
      },
      triggerFavoriteUpdateBatch: async () => {
        throw new Error("not used");
      },
      triggerAllFavoriteUpdates: async () => {
        throw new Error("not used");
      },
      triggerAllFavoriteUpdatesWithRecovery: async () => runPromise,
      listenFavoriteUpdateRecoveryEvents: async (handler) => {
        liveHandler = handler;
        return unlisten;
      },
    };

    render(<AutomationView paths={approvedDefaultPaths} service={service} />);

    await user.click(screen.getByRole("button", { name: "自动恢复更新全部" }));
    await waitFor(() => expect(liveHandler).toBeDefined());

    liveHandler?.({
      kind: "started",
      message: "开始自动恢复长跑，目标 500 本",
      processed: 0,
      downloadedChapters: 0,
      skippedCount: 0,
      restarts: 0,
    });

    expect(
      await screen.findByText("开始自动恢复长跑，目标 500 本"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("已处理 0 本，下载 0 话，跳过 0 本，重启 0 次"),
    ).toBeInTheDocument();
    expect(screen.getByText("运行中")).toBeInTheDocument();

    resolveRun!({
      requestedLimit: 500,
      maxRestarts: 2,
      restarts: 0,
      processed: 0,
      downloadedChapters: 0,
      skippedCount: 0,
      stoppedReason: "completed",
      lastError: null,
      events: [
        {
          kind: "started",
          message: "开始自动恢复长跑，目标 500 本",
          processed: 0,
          downloadedChapters: 0,
          skippedCount: 0,
          restarts: 0,
        },
        {
          kind: "completed",
          message: "自动恢复长跑完成",
          processed: 0,
          downloadedChapters: 0,
          skippedCount: 0,
          restarts: 0,
        },
      ],
      runs: [],
    });

    await waitFor(() => expect(unlisten).toHaveBeenCalledTimes(1));
  });

  it("实时日志较多时折叠较早记录", async () => {
    const realtimeEvents = Array.from({ length: 126 }, (_, index) => ({
      kind: "comic_downloaded" as const,
      message: `第 ${index + 1} 本已交给漫画控，下载 1 话`,
      processed: index + 1,
      downloadedChapters: index + 1,
      skippedCount: 0,
      restarts: 0,
    }));
    const service: AutomationService = {
      findWindows: async () => [],
      launch: async () => {
        throw new Error("not used");
      },
      restart: async () => {
        throw new Error("not used");
      },
      getStatus: async () => {
        throw new Error("not used");
      },
      scanBadges: async () => {
        throw new Error("not used");
      },
      scanFavoritesUpdates: async () => {
        throw new Error("not used");
      },
      openFavorites: async () => {
        throw new Error("not used");
      },
      openFirstUpdatedComic: async () => {
        throw new Error("not used");
      },
      scanDetailUpdates: async () => {
        throw new Error("not used");
      },
      triggerFirstDetailUpdateDownload: async () => {
        throw new Error("not used");
      },
      triggerDetailUpdateDownloadBatch: async () => {
        throw new Error("not used");
      },
      triggerNextFavoriteUpdateDownload: async () => {
        throw new Error("not used");
      },
      triggerFavoriteUpdateBatch: async () => {
        throw new Error("not used");
      },
      triggerAllFavoriteUpdates: async () => {
        throw new Error("not used");
      },
      listenFavoriteUpdateRecoveryEvents: async () => () => {},
      triggerAllFavoriteUpdatesWithRecovery: async () => ({
        requestedLimit: 500,
        maxRestarts: 2,
        restarts: 0,
        processed: 126,
        downloadedChapters: 126,
        skippedCount: 0,
        stoppedReason: "completed",
        lastError: null,
        events: realtimeEvents,
        runs: [],
      }),
    };

    render(<AutomationView paths={approvedDefaultPaths} service={service} />);

    await userEvent.click(screen.getByRole("button", { name: "自动恢复更新全部" }));

    expect(await screen.findByText("已折叠较早日志 6 条")).toBeInTheDocument();
    expect(screen.queryByText("第 1 本已交给漫画控，下载 1 话")).not.toBeInTheDocument();
    expect(screen.getByText("第 126 本已交给漫画控，下载 1 话")).toBeInTheDocument();
  });

  it("启动时恢复上次保存的自动恢复长跑记录", () => {
    saveRecoveryRunHistory(
      buildRecoveryRunHistoryRecord({
        result: {
          requestedLimit: 500,
          maxRestarts: 2,
          restarts: 1,
          processed: 12,
          downloadedChapters: 30,
          skippedCount: 2,
          stoppedReason: "completed",
          lastError: "漫画控窗口无响应",
          events: [
            {
              kind: "completed",
              message: "自动恢复长跑完成",
              processed: 12,
              downloadedChapters: 30,
              skippedCount: 2,
              restarts: 1,
            },
          ],
          runs: [],
        },
        now: new Date("2026-06-28T08:00:00.000Z"),
      }),
    );

    render(<AutomationView paths={approvedDefaultPaths} />);

    expect(screen.getByText("已恢复上次长跑记录")).toBeInTheDocument();
    expect(screen.getByText("恢复处理 12/500")).toBeInTheDocument();
    expect(screen.getByText("处理漫画").closest("div")).toHaveTextContent("12");
    expect(screen.getByText("下载章节").closest("div")).toHaveTextContent("30");
    expect(screen.getByText("自动恢复长跑完成")).toBeInTheDocument();
    expect(screen.getAllByText("漫画控窗口无响应").length).toBeGreaterThan(0);
  });

  it("自动恢复长跑完成后保存最新记录", async () => {
    const service: AutomationService = {
      findWindows: async () => [],
      launch: async () => {
        throw new Error("not used");
      },
      restart: async () => {
        throw new Error("not used");
      },
      getStatus: async () => {
        throw new Error("not used");
      },
      scanBadges: async () => {
        throw new Error("not used");
      },
      scanFavoritesUpdates: async () => {
        throw new Error("not used");
      },
      openFavorites: async () => {
        throw new Error("not used");
      },
      openFirstUpdatedComic: async () => {
        throw new Error("not used");
      },
      scanDetailUpdates: async () => {
        throw new Error("not used");
      },
      triggerFirstDetailUpdateDownload: async () => {
        throw new Error("not used");
      },
      triggerDetailUpdateDownloadBatch: async () => {
        throw new Error("not used");
      },
      triggerNextFavoriteUpdateDownload: async () => {
        throw new Error("not used");
      },
      triggerFavoriteUpdateBatch: async () => {
        throw new Error("not used");
      },
      triggerAllFavoriteUpdates: async () => {
        throw new Error("not used");
      },
      listenFavoriteUpdateRecoveryEvents: async () => () => {},
      triggerAllFavoriteUpdatesWithRecovery: async () => ({
        requestedLimit: 500,
        maxRestarts: 2,
        restarts: 0,
        processed: 18,
        downloadedChapters: 41,
        skippedCount: 1,
        stoppedReason: "completed",
        lastError: null,
        events: [
          {
            kind: "completed",
            message: "自动恢复长跑完成",
            processed: 18,
            downloadedChapters: 41,
            skippedCount: 1,
            restarts: 0,
          },
        ],
        runs: [],
      }),
    };

    render(<AutomationView paths={approvedDefaultPaths} service={service} />);

    await userEvent.click(screen.getByRole("button", { name: "自动恢复更新全部" }));

    await waitFor(() => {
      expect(loadRecoveryRunHistory()?.result?.processed).toBe(18);
    });
    expect(loadRecoveryRunHistory()?.events.at(-1)?.message).toBe(
      "自动恢复长跑完成",
    );
  });
});
