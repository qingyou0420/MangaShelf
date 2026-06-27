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

    await user.click(screen.getByRole("button", { name: "刷新状态" }));

    expect(await screen.findByText("红点 5")).toBeInTheDocument();
    expect(screen.getByText("稳定样本 4")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "识别红点" }));

    expect(await screen.findByText("截图 850x600")).toBeInTheDocument();
    expect(screen.getByText("识别红点 2")).toBeInTheDocument();
  });
});
