import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ReaderView } from "./ReaderView";
import type { MangaConFavorite } from "../../lib/types";

afterEach(() => {
  cleanup();
});

describe("ReaderView", () => {
  it("展示阅读器空状态和关键工具按钮", () => {
    render(<ReaderView />);

    expect(screen.getByText("阅读器")).toBeInTheDocument();
    expect(screen.getAllByText("选择一本本地漫画后开始阅读").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "书签" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "双页" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "全屏" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "方向" })).toBeInTheDocument();
  });

  it("加载本地漫画章节并支持翻页阅读", async () => {
    const user = userEvent.setup();
    const comic: MangaConFavorite = {
      id: "cp:hzzsddhhshct",
      name: "婚纱之中待到花火散去",
      location: "婚纱之中待到花火散去",
      tags: [],
      sourceUri: "cp:hzzsddhhshct",
      sourceScheme: "cp",
      localPath: "E:\\书架\\婚纱之中待到花火散去",
      chapterCount: 1,
      imageCount: 2,
      readProgressPage: 0,
      scanStatus: "matched",
    };

    render(
      <ReaderView
        comic={comic}
        service={{
          scanChapters: async () => [
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
          ],
          listPages: async () => [
            "E:\\书架\\婚纱之中待到花火散去\\第01话\\001.jpg",
            "E:\\书架\\婚纱之中待到花火散去\\第01话\\002.jpg",
          ],
        }}
        toImageSrc={(path) => `asset://${path}`}
      />,
    );

    expect(await screen.findByRole("button", { name: "第01话 2 页" })).toBeInTheDocument();
    const firstPage = await screen.findByAltText("第01话 第 1 页");
    expect(firstPage).toHaveAttribute(
      "src",
      "asset://E:\\书架\\婚纱之中待到花火散去\\第01话\\001.jpg",
    );
    expect(screen.getByText("1 / 2")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "下一页" }));

    expect(screen.getByAltText("第01话 第 2 页")).toHaveAttribute(
      "src",
      "asset://E:\\书架\\婚纱之中待到花火散去\\第01话\\002.jpg",
    );
    expect(screen.getByText("2 / 2")).toBeInTheDocument();
  });
});
