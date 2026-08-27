import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import {
  allowAssetRoot,
  cancelLibraryScan,
  checkLocalInstallerUpdates,
  getAppVersion,
  listChapterPages,
  listenScanProgress,
  loadLibrary,
  openLocalInstaller,
  openPath,
  pathIsDirectory,
  pickDirectory,
  saveReadProgress,
  scanLibrary,
  scanLocalChapters,
  setComicFavorite,
} from "./lib/api";
import { defaultLibraryPaths } from "./lib/defaults";
import type { LibraryComic } from "./lib/types";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
  invoke: vi.fn(),
}));

vi.mock("./lib/api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./lib/api")>()),
  allowAssetRoot: vi.fn(),
  cancelLibraryScan: vi.fn(),
  checkLocalInstallerUpdates: vi.fn(),
  getAppVersion: vi.fn(),
  listChapterPages: vi.fn(),
  listenScanProgress: vi.fn().mockResolvedValue(() => undefined),
  listenExtractProgress: vi.fn().mockResolvedValue(() => undefined),
  loadLibrary: vi.fn(),
  deleteLibraryComic: vi.fn(),
  listCoverCandidates: vi.fn(),
  setComicCover: vi.fn(),
  openLocalInstaller: vi.fn(),
  openPath: vi.fn(),
  pickDirectory: vi.fn(),
  pathIsDirectory: vi.fn().mockResolvedValue(true),
  clearReadProgress: vi.fn(),
  saveReadProgress: vi.fn(),
  scanLibrary: vi.fn(),
  scanLocalChapters: vi.fn(),
  setComicFavorite: vi.fn(),
  setReaderPrefs: vi.fn(),
  updateComicMetadata: vi.fn(),
}));

const checkLocalInstallerUpdatesMock = vi.mocked(checkLocalInstallerUpdates);
const getAppVersionMock = vi.mocked(getAppVersion);
const openLocalInstallerMock = vi.mocked(openLocalInstaller);
const listChapterPagesMock = vi.mocked(listChapterPages);
const listenScanProgressMock = vi.mocked(listenScanProgress);
const loadLibraryMock = vi.mocked(loadLibrary);
const saveReadProgressMock = vi.mocked(saveReadProgress);
const scanLibraryMock = vi.mocked(scanLibrary);
const scanLocalChaptersMock = vi.mocked(scanLocalChapters);
const setComicFavoriteMock = vi.mocked(setComicFavorite);
const allowAssetRootMock = vi.mocked(allowAssetRoot);
const cancelLibraryScanMock = vi.mocked(cancelLibraryScan);
const pickDirectoryMock = vi.mocked(pickDirectory);
const openPathMock = vi.mocked(openPath);
const pathIsDirectoryMock = vi.mocked(pathIsDirectory);

const loadedComics: LibraryComic[] = [
  {
    id: "local:a",
    name: "婚纱之中待到花火散去",
    location: "婚纱之中待到花火散去",
    tags: ["测试作者"],
    localPath: "E:\\书架\\婚纱之中待到花火散去",
    coverPath: "E:\\书架\\婚纱之中待到花火散去\\第01话\\001.jpg",
    chapterCount: 1,
    imageCount: 2,
    latestChapterTitle: "第01话",
    readProgressPage: 0,
    scanStatus: "matched",
    favorited: false,
    readingDirection: "ltr",
    fitMode: "contain",
  },
];

describe("App", () => {
  beforeEach(() => {
    checkLocalInstallerUpdatesMock.mockReset();
    getAppVersionMock.mockReset();
    openLocalInstallerMock.mockReset();
    listChapterPagesMock.mockReset();
    listenScanProgressMock.mockReset();
    loadLibraryMock.mockReset();
    saveReadProgressMock.mockReset();
    scanLibraryMock.mockReset();
    scanLocalChaptersMock.mockReset();
    setComicFavoriteMock.mockReset();
    allowAssetRootMock.mockReset();
    cancelLibraryScanMock.mockReset();
    pickDirectoryMock.mockReset();
    openPathMock.mockReset();
    pathIsDirectoryMock.mockReset();
    window.localStorage.clear();

    listenScanProgressMock.mockResolvedValue(() => undefined);
    allowAssetRootMock.mockResolvedValue(undefined);
    cancelLibraryScanMock.mockResolvedValue(undefined);
    pickDirectoryMock.mockResolvedValue(null);
    openPathMock.mockResolvedValue(undefined);
    pathIsDirectoryMock.mockResolvedValue(true);
    getAppVersionMock.mockResolvedValue("2.0.0");
    checkLocalInstallerUpdatesMock.mockResolvedValue({
      currentVersion: "2.0.0",
      hasUpdate: false,
      latest: null,
      packages: [],
      searchedDirs: [],
    });
    openLocalInstallerMock.mockResolvedValue(undefined);
    loadLibraryMock.mockResolvedValue({
      bookshelfRoot: defaultLibraryPaths.bookshelfRoot,
      databasePath: defaultLibraryPaths.databasePath,
      comics: [],
    });
    scanLibraryMock.mockResolvedValue({
      scanned: 0,
      added: 0,
      updated: 0,
      missing: 0,
      bookshelfRoot: defaultLibraryPaths.bookshelfRoot,
      databasePath: defaultLibraryPaths.databasePath,
      comics: [],
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("启动时加载本地索引且不启动外部引擎", async () => {
    loadLibraryMock.mockResolvedValue({
      bookshelfRoot: defaultLibraryPaths.bookshelfRoot,
      databasePath: defaultLibraryPaths.databasePath,
      comics: loadedComics,
      baselineCompleted: true,
    });
    scanLibraryMock.mockResolvedValue({
      scanned: 1,
      added: 0,
      updated: 0,
      unchanged: 1,
      missing: 0,
      bookshelfRoot: defaultLibraryPaths.bookshelfRoot,
      databasePath: defaultLibraryPaths.databasePath,
      comics: loadedComics,
      baselineCompleted: true,
    });

    render(<App />);

    await waitFor(() => {
      expect(loadLibraryMock).toHaveBeenCalledWith({
        ...defaultLibraryPaths,
        extraRoots: [],
      });
    });
    await waitFor(() => {
      expect(scanLibraryMock).toHaveBeenCalled();
    });
    expect(screen.getByRole("heading", { name: "书库" })).toBeInTheDocument();
    expect(screen.getAllByText("婚纱之中待到花火散去").length).toBeGreaterThan(0);
    expect(screen.queryByRole("button", { name: "下载" })).not.toBeInTheDocument();
    expect(screen.queryByText(/漫画控/)).not.toBeInTheDocument();
    expect(screen.queryByText(/下载引擎/)).not.toBeInTheDocument();
  });

  it("发现本地更高版本安装包时在版本号旁显示更新按钮并可启动安装", async () => {
    const user = userEvent.setup();
    checkLocalInstallerUpdatesMock.mockResolvedValue({
      currentVersion: "2.0.0",
      hasUpdate: true,
      latest: {
        path: "D:\\Grisia Studio\\Manga Library\\release\\Manga Library_2.1.0_x64-setup.exe",
        fileName: "Manga Library_2.1.0_x64-setup.exe",
        version: "2.1.0",
        isNewer: true,
      },
      packages: [],
      searchedDirs: ["D:\\Grisia Studio\\Manga Library\\release"],
    });

    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId("sidebar-app-update")).toHaveTextContent(
        "更新 v2.1.0",
      );
    });

    await user.click(screen.getByTestId("sidebar-app-update"));
    await waitFor(() => {
      expect(openLocalInstallerMock).toHaveBeenCalledWith(
        "D:\\Grisia Studio\\Manga Library\\release\\Manga Library_2.1.0_x64-setup.exe",
      );
    });
  });

  it("扫描书架后可打开阅读并恢复进度", async () => {
    const user = userEvent.setup();
    scanLibraryMock.mockResolvedValue({
      scanned: 1,
      added: 1,
      updated: 0,
      missing: 0,
      bookshelfRoot: defaultLibraryPaths.bookshelfRoot,
      databasePath: defaultLibraryPaths.databasePath,
      comics: loadedComics,
      baselineCompleted: true,
      establishedBaseline: true,
    });
    scanLocalChaptersMock.mockResolvedValue([
      {
        id: "local:a::第01话",
        comicId: "local:a",
        title: "第01话",
        path: "E:\\书架\\婚纱之中待到花火散去\\第01话",
        ordinal: 1,
        pageCount: 2,
        readProgressPage: 0,
        specialKind: "regular",
      },
    ]);
    listChapterPagesMock.mockResolvedValue([
      "E:\\书架\\婚纱之中待到花火散去\\第01话\\001.jpg",
      "E:\\书架\\婚纱之中待到花火散去\\第01话\\002.jpg",
    ]);
    saveReadProgressMock.mockResolvedValue({
      ...loadedComics[0],
      readProgressPage: 1,
      lastReadChapterId: "local:a::第01话",
      lastReadAt: "2026-08-17 12:00:00",
    });

    render(<App />);

    await user.click(
      await screen.findByRole("button", { name: /导入现有书库|扫描书架/ }),
    );
    await waitFor(() => {
      expect(scanLibraryMock).toHaveBeenCalledWith({
        ...defaultLibraryPaths,
        extraRoots: [],
      });
    });
    expect(
      screen.getAllByText(/已导入现有书库 1 部，已作为基准/).length,
    ).toBeGreaterThan(0);

    await user.click(
      screen.getByRole("button", { name: "查看 婚纱之中待到花火散去" }),
    );
    expect(await screen.findByRole("button", { name: "开始阅读" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "开始阅读" }));
    expect(
      await screen.findByRole("heading", { name: "婚纱之中待到花火散去" }),
    ).toBeInTheDocument();
    expect(await screen.findByAltText("第01话 第 1 页")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "返回书库" })).toBeInTheDocument();
  });

  it("continues from the library card and returns to the library", async () => {
    const user = userEvent.setup();
    const historyComic = {
      ...loadedComics[0],
      lastReadAt: "2026-08-17 12:00:00",
      lastReadChapterId: "local:a::第01话",
      lastReadChapterTitle: "第01话",
      readProgressPage: 1,
    };
    loadLibraryMock.mockResolvedValue({
      bookshelfRoot: defaultLibraryPaths.bookshelfRoot,
      databasePath: defaultLibraryPaths.databasePath,
      comics: [historyComic],
      baselineCompleted: true,
    });
    scanLibraryMock.mockResolvedValue({
      scanned: 1,
      added: 0,
      updated: 0,
      unchanged: 1,
      missing: 0,
      bookshelfRoot: defaultLibraryPaths.bookshelfRoot,
      databasePath: defaultLibraryPaths.databasePath,
      comics: [historyComic],
      baselineCompleted: true,
    });
    scanLocalChaptersMock.mockResolvedValue([
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
    ]);
    listChapterPagesMock.mockResolvedValue([
      "E:\\书架\\婚纱之中待到花火散去\\第01话\\001.jpg",
      "E:\\书架\\婚纱之中待到花火散去\\第01话\\002.jpg",
    ]);

    render(<App />);
    await screen.findAllByText("婚纱之中待到花火散去");
    expect(screen.queryByRole("button", { name: "历史" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /继续$/ }));
    expect(await screen.findByAltText("第01话 第 2 页")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "返回书库" }));
    expect(screen.getByRole("heading", { name: "书库" })).toBeInTheDocument();
  });
});
