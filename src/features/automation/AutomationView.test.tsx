import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AutomationView } from "./AutomationView";

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
});
