import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ReaderView } from "./ReaderView";

describe("ReaderView", () => {
  it("展示阅读器空状态和关键工具按钮", () => {
    render(<ReaderView />);

    expect(screen.getByText("阅读器")).toBeInTheDocument();
    expect(screen.getByText("选择一本本地漫画后开始阅读")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "书签" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "双页" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "全屏" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "方向" })).toBeInTheDocument();
  });
});
