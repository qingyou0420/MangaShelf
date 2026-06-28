import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { approvedDefaultPaths, sampleFavorite } from "../../test/fixtures";
import { Dashboard } from "./Dashboard";

describe("Dashboard", () => {
  afterEach(() => {
    cleanup();
  });

  it("展示工具标题、漫画控状态和数据库队列更新按钮", () => {
    render(
      <Dashboard
        paths={approvedDefaultPaths}
        favorites={[sampleFavorite]}
        pendingTasks={3}
      />,
    );

    expect(screen.getByText("漫画控伴侣")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "一键更新收藏" })).toBeInTheDocument();
    expect(screen.getByText("数据库队列")).toBeInTheDocument();
    expect(screen.getByText("漫画控状态")).toBeInTheDocument();
    expect(screen.getByText("E:\\漫画控\\MangaCon.exe")).toBeInTheDocument();
    expect(
      screen.getByText(
        "C:\\Users\\Administrator\\AppData\\Local\\MangaCon3\\MangaCon.dat",
      ),
    ).toBeInTheDocument();
  });

  it("点击一键更新收藏时触发更新回调", async () => {
    const user = userEvent.setup();
    const onUpdateFavorites = vi.fn();

    render(
      <Dashboard
        paths={approvedDefaultPaths}
        favorites={[sampleFavorite]}
        pendingTasks={3}
        onUpdateFavorites={onUpdateFavorites}
      />,
    );

    await user.click(screen.getByRole("button", { name: "一键更新收藏" }));

    expect(onUpdateFavorites).toHaveBeenCalledTimes(1);
  });
});
