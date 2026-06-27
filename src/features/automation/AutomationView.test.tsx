import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { approvedDefaultPaths } from "../../test/fixtures";
import { AutomationView } from "./AutomationView";

afterEach(() => {
  cleanup();
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
      }),
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

    await user.click(screen.getByRole("button", { name: "处理下一个收藏更新" }));

    expect(await screen.findByText("收藏点击 117,172")).toBeInTheDocument();
    expect(screen.getByText("收藏滚动定位 2 次")).toBeInTheDocument();
    expect(screen.getByText("更新下载点击 203,516")).toBeInTheDocument();
  });
});
