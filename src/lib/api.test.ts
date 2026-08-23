import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  allowAssetRoot,
  cancelLibraryScan,
  checkLocalInstallerUpdates,
  getAppVersion,
  listChapterPages,
  loadLibrary,
  openLocalInstaller,
  openPath,
  pickDirectory,
  saveReadProgress,
  scanLibrary,
  scanLocalChapters,
  setComicFavorite,
  setReaderPrefs,
  updateComicMetadata,
} from "./api";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

describe("local library API wrappers", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("loadLibrary / scanLibrary", async () => {
    invokeMock.mockResolvedValue({
      scanned: 1,
      added: 1,
      updated: 0,
      missing: 0,
      databasePath: "E:\\书架\\manga-library.sqlite",
      bookshelfRoot: "E:\\书架",
      comics: [],
    });
    await loadLibrary({
      bookshelfRoot: "E:\\书架",
      databasePath: "E:\\书架\\manga-library.sqlite",
    });
    await scanLibrary({
      bookshelfRoot: "E:\\书架",
      databasePath: "E:\\书架\\manga-library.sqlite",
    });
    expect(invokeMock).toHaveBeenCalledWith("load_library", {
      bookshelfRoot: "E:\\书架",
      databasePath: "E:\\书架\\manga-library.sqlite",
    });
    expect(invokeMock).toHaveBeenCalledWith("scan_library", {
      bookshelfRoot: "E:\\书架",
      databasePath: "E:\\书架\\manga-library.sqlite",
      extraRoots: [],
    });
  });

  it("scanLocalChapters / listChapterPages / saveReadProgress", async () => {
    invokeMock.mockResolvedValue(null);
    await scanLocalChapters({
      comicId: "local:a",
      comicDirectory: "E:\\书架\\a",
      databasePath: "lib.sqlite",
    });
    await listChapterPages({ chapterPath: "E:\\书架\\a\\第01话" });
    await saveReadProgress({
      databasePath: "lib.sqlite",
      comicId: "local:a",
      chapterId: "local:a::第01话",
      page: 3,
    });
    expect(invokeMock).toHaveBeenCalledWith("scan_local_chapters", {
      comicId: "local:a",
      comicDirectory: "E:\\书架\\a",
      databasePath: "lib.sqlite",
      force: false,
    });
    expect(invokeMock).toHaveBeenCalledWith("list_chapter_pages", {
      chapterPath: "E:\\书架\\a\\第01话",
    });
    expect(invokeMock).toHaveBeenCalledWith("save_read_progress", {
      databasePath: "lib.sqlite",
      comicId: "local:a",
      chapterId: "local:a::第01话",
      page: 3,
    });
  });

  it("metadata / favorite / reader prefs", async () => {
    invokeMock.mockResolvedValue(null);
    await updateComicMetadata({
      databasePath: "lib.sqlite",
      comicId: "local:a",
      name: "标题",
      author: "作者",
      tags: ["标签"],
    });
    await setComicFavorite({
      databasePath: "lib.sqlite",
      comicId: "local:a",
      favorited: true,
    });
    await setReaderPrefs({
      databasePath: "lib.sqlite",
      comicId: "local:a",
      readingDirection: "rtl",
      fitMode: "width",
    });
    expect(invokeMock).toHaveBeenCalledWith("update_comic_metadata", {
      databasePath: "lib.sqlite",
      comicId: "local:a",
      name: "标题",
      author: "作者",
      tags: ["标签"],
    });
    expect(invokeMock).toHaveBeenCalledWith("set_comic_favorite", {
      databasePath: "lib.sqlite",
      comicId: "local:a",
      favorited: true,
    });
    expect(invokeMock).toHaveBeenCalledWith("set_reader_prefs", {
      databasePath: "lib.sqlite",
      comicId: "local:a",
      readingDirection: "rtl",
      fitMode: "width",
      readMode: "page",
    });
  });

  it("getAppVersion / checkLocalInstallerUpdates / openLocalInstaller", async () => {
    invokeMock
      .mockResolvedValueOnce("2.0.0")
      .mockResolvedValueOnce({
        currentVersion: "2.0.0",
        hasUpdate: false,
        latest: null,
        packages: [],
        searchedDirs: [],
      })
      .mockResolvedValueOnce(undefined);

    await expect(getAppVersion()).resolves.toBe("2.0.0");
    await checkLocalInstallerUpdates();
    await openLocalInstaller(
      "https://github.com/qingyou0420/MangaShelf/releases/download/v2.5.0/MangaShelf_2.5.0_x64-setup.exe",
    );

    expect(invokeMock).toHaveBeenCalledWith("get_app_version");
    expect(invokeMock).toHaveBeenCalledWith("check_local_installer_updates");
    expect(invokeMock).toHaveBeenCalledWith("open_local_installer", {
      path: "https://github.com/qingyou0420/MangaShelf/releases/download/v2.5.0/MangaShelf_2.5.0_x64-setup.exe",
    });
  });

  it("pickDirectory / openPath / cancel scan / allow assets", async () => {
    invokeMock.mockResolvedValue(null);
    await pickDirectory();
    await openPath("E:\\书架");
    await cancelLibraryScan();
    await allowAssetRoot("E:\\书架");
    expect(invokeMock).toHaveBeenCalledWith("pick_directory");
    expect(invokeMock).toHaveBeenCalledWith("open_path", { path: "E:\\书架" });
    expect(invokeMock).toHaveBeenCalledWith("cancel_library_scan");
    expect(invokeMock).toHaveBeenCalledWith("allow_asset_root", {
      path: "E:\\书架",
    });
  });
});
