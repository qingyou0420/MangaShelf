import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { approvedDefaultPaths, sampleFavorite } from "../../test/fixtures";
import { Dashboard } from "./Dashboard";

describe("Dashboard", () => {
  it("展示工具标题、漫画控状态和一键更新按钮", () => {
    render(
      <Dashboard
        paths={approvedDefaultPaths}
        favorites={[sampleFavorite]}
        pendingTasks={3}
      />,
    );

    expect(screen.getByText("漫画控伴侣")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "一键更新收藏" })).toBeInTheDocument();
    expect(screen.getByText("漫画控状态")).toBeInTheDocument();
    expect(screen.getByText("E:\\漫画控\\MangaCon.exe")).toBeInTheDocument();
  });
});
