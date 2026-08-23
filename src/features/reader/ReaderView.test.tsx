import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ReaderView } from "./ReaderView";
import type { LibraryComic } from "../../lib/types";

vi.mock("../../lib/api", () => ({
  listenExtractProgress: vi.fn().mockResolvedValue(() => undefined),
  scanLocalChapters: vi.fn(),
  listChapterPages: vi.fn(),
}));

afterEach(() => {
  cleanup();
});

const comic: LibraryComic = {
  id: "local:a",
  name: "婚纱之中待到花火散去",
  location: "婚纱之中待到花火散去",
  tags: [],
  localPath: "E:\\书架\\婚纱之中待到花火散去",
  chapterCount: 1,
  imageCount: 2,
  readProgressPage: 0,
  lastReadChapterId: "local:a::第01话",
  scanStatus: "matched",
  favorited: false,
  readingDirection: "ltr",
  fitMode: "contain",
};

describe("ReaderView", () => {
  it("展示精简空状态与返回", () => {
    render(<ReaderView onBack={() => undefined} />);

    expect(screen.getByText("阅读")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "返回书库" })).toBeInTheDocument();
    expect(screen.getAllByText("从书库打开漫画").length).toBeGreaterThan(0);
  });

  it("加载本地漫画章节、恢复进度并保存翻页", async () => {
    const user = userEvent.setup();
    const saveProgress = vi.fn().mockResolvedValue(undefined);

    render(
      <ReaderView
        comic={{ ...comic, readProgressPage: 1 }}
        onBack={() => undefined}
        service={{
          scanChapters: async () => [
            {
              id: "local:a::第01话",
              comicId: "local:a",
              title: "第01话",
              path: "E:\\书架\\婚纱之中待到花火散去\\第01话",
              ordinal: 1,
              pageCount: 2,
              readProgressPage: 1,
              specialKind: "regular",
            },
          ],
          listPages: async () => [
            "E:\\书架\\婚纱之中待到花火散去\\第01话\\001.jpg",
            "E:\\书架\\婚纱之中待到花火散去\\第01话\\002.jpg",
          ],
          saveProgress,
        }}
        toImageSrc={(path) => `asset://${path}`}
      />,
    );

    expect(await screen.findByAltText("第01话 第 2 页")).toHaveAttribute(
      "src",
      "asset://E:\\书架\\婚纱之中待到花火散去\\第01话\\002.jpg",
    );
    expect(screen.getByRole("combobox", { name: "页面适配" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "目录" }));
    expect(screen.getByRole("button", { name: "第01话 2 页" })).toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "上一页" })[0]);
    expect(screen.getByAltText("第01话 第 1 页")).toBeInTheDocument();
  });

  it("advances to the next chapter after the last page", async () => {
    const user = userEvent.setup();
    render(
      <ReaderView
        comic={comic}
        onBack={() => undefined}
        service={{
          scanChapters: async () => [
            {
              id: "local:a::第01话",
              comicId: "local:a",
              title: "第01话",
              path: "E:\\书架\\婚纱之中待到花火散去\\第01话",
              ordinal: 1,
              pageCount: 1,
              readProgressPage: 0,
              specialKind: "regular",
            },
            {
              id: "local:a::第02话",
              comicId: "local:a",
              title: "第02话",
              path: "E:\\书架\\婚纱之中待到花火散去\\第02话",
              ordinal: 2,
              pageCount: 1,
              readProgressPage: 0,
              specialKind: "regular",
            },
          ],
          listPages: async (chapterPath) =>
            chapterPath.includes("第02话")
              ? ["E:\\书架\\婚纱之中待到花火散去\\第02话\\001.jpg"]
              : ["E:\\书架\\婚纱之中待到花火散去\\第01话\\001.jpg"],
        }}
        toImageSrc={(path) => `asset://${path}`}
      />,
    );

    expect(await screen.findByAltText("第01话 第 1 页")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "下一页" }));
    expect(await screen.findByAltText("第02话 第 1 页")).toBeInTheDocument();
  });

  it("进度回写不会重新扫描或清空当前页", async () => {
    const scanChapters = vi.fn(async () => [
      {
        id: "local:a::第01话",
        comicId: "local:a",
        title: "第01话",
        path: "E:\\书架\\婚纱之中待到花火散去\\第01话",
        ordinal: 1,
        pageCount: 2,
        readProgressPage: 0,
        specialKind: "regular" as const,
      },
    ]);
    const listPages = vi.fn(async () => [
      "E:\\书架\\婚纱之中待到花火散去\\第01话\\001.jpg",
      "E:\\书架\\婚纱之中待到花火散去\\第01话\\002.jpg",
    ]);

    const { rerender } = render(
      <ReaderView
        comic={comic}
        onBack={() => undefined}
        service={{
          scanChapters,
          listPages,
          saveProgress: async () => undefined,
        }}
        toImageSrc={(path) => `asset://${path}`}
      />,
    );

    expect(await screen.findByAltText("第01话 第 1 页")).toBeInTheDocument();
    expect(scanChapters).toHaveBeenCalledTimes(1);
    expect(listPages).toHaveBeenCalledTimes(1);

    rerender(
      <ReaderView
        comic={{
          ...comic,
          readProgressPage: 0,
          lastReadChapterId: "local:a::第01话",
          lastReadAt: "2026-08-17 12:00:01",
        }}
        onBack={() => undefined}
        service={{
          scanChapters,
          listPages,
          saveProgress: async () => undefined,
        }}
        toImageSrc={(path) => `asset://${path}`}
      />,
    );

    expect(screen.getByAltText("第01话 第 1 页")).toBeInTheDocument();
    expect(scanChapters).toHaveBeenCalledTimes(1);
    expect(listPages).toHaveBeenCalledTimes(1);
  });

  it("双页模式一次翻两页", async () => {
    const user = userEvent.setup();
    render(
      <ReaderView
        comic={{ ...comic, readMode: "spread", lastReadAt: "2026-08-17 12:00:00" }}
        onBack={() => undefined}
        service={{
          scanChapters: async () => [
            {
              id: "local:a::第01话",
              comicId: "local:a",
              title: "第01话",
              path: "E:\\书架\\婚纱之中待到花火散去\\第01话",
              ordinal: 1,
              pageCount: 4,
              readProgressPage: 0,
              specialKind: "regular",
            },
          ],
          listPages: async () => [
            "E:\\书架\\婚纱之中待到花火散去\\第01话\\001.jpg",
            "E:\\书架\\婚纱之中待到花火散去\\第01话\\002.jpg",
            "E:\\书架\\婚纱之中待到花火散去\\第01话\\003.jpg",
            "E:\\书架\\婚纱之中待到花火散去\\第01话\\004.jpg",
          ],
        }}
        toImageSrc={(path) => `asset://${path}`}
      />,
    );

    expect(await screen.findByAltText("第01话 第 1 页")).toBeInTheDocument();
    expect(screen.queryByAltText("第01话 第 2 页")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "更多" }));
    await user.click(screen.getByRole("button", { name: "封面单独" }));
    expect(screen.getByAltText("第01话 第 2 页")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "下一页" }));
    expect(screen.getByAltText("第01话 第 3 页")).toBeInTheDocument();
    expect(screen.getByAltText("第01话 第 4 页")).toBeInTheDocument();
  });
});
