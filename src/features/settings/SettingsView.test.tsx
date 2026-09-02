import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  checkLocalInstallerUpdates,
  openLocalInstaller,
  openPath,
  pickDirectory,
} from "../../lib/api";
import { defaultLibraryPaths } from "../../lib/defaults";
import { SettingsView } from "./SettingsView";

vi.mock("../../lib/api", () => ({
  getAppVersion: vi.fn(),
  checkLocalInstallerUpdates: vi.fn(),
  openLocalInstaller: vi.fn(),
  pickDirectory: vi.fn(),
  openPath: vi.fn(),
  libraryCacheStats: vi.fn(),
  clearLibraryCache: vi.fn(),
}));

const checkLocalInstallerUpdatesMock = vi.mocked(checkLocalInstallerUpdates);
const openLocalInstallerMock = vi.mocked(openLocalInstaller);
const pickDirectoryMock = vi.mocked(pickDirectory);
const openPathMock = vi.mocked(openPath);

describe("SettingsView version & local update", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    checkLocalInstallerUpdatesMock.mockReset();
    openLocalInstallerMock.mockReset();
    pickDirectoryMock.mockReset();
    openPathMock.mockReset();
    pickDirectoryMock.mockResolvedValue(null);
    openPathMock.mockResolvedValue(undefined);
  });

  it("shows version and reports when a newer installer is found", async () => {
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
      packages: [
        {
          path: "D:\\Grisia Studio\\Manga Library\\release\\Manga Library_2.1.0_x64-setup.exe",
          fileName: "Manga Library_2.1.0_x64-setup.exe",
          version: "2.1.0",
          isNewer: true,
        },
      ],
      searchedDirs: ["D:\\Grisia Studio\\Manga Library\\release"],
    });
    openLocalInstallerMock.mockResolvedValue(undefined);

    render(
      <SettingsView paths={defaultLibraryPaths} appVersion="2.0.0" />,
    );

    expect(screen.getByTestId("app-version")).toHaveTextContent("v2.0.0");
    expect(screen.getByLabelText("书架路径")).toHaveValue(
      defaultLibraryPaths.bookshelfRoot,
    );
    expect(screen.getByRole("button", { name: "选择文件夹" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "保存路径" })).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "书架" })).toBeInTheDocument();
    expect(screen.queryByText(/引擎/)).not.toBeInTheDocument();
    expect(screen.queryByText(/漫画控/)).not.toBeInTheDocument();
    expect(screen.getByText("仅读取本地文件夹，不会修改或删除文件。")).toBeInTheDocument();
    expect(screen.queryByText(/qingyou0420\/MangaShelf/)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByTestId("update-latest-version")).toHaveTextContent(
        "v2.1.0",
      );
    });

    await user.click(screen.getByRole("button", { name: "下载并安装" }));
    await waitFor(() => {
      expect(openLocalInstallerMock).toHaveBeenCalledWith(
        "D:\\Grisia Studio\\Manga Library\\release\\Manga Library_2.1.0_x64-setup.exe",
      );
    });
  });

  it("shows empty state when no installer packages exist", async () => {
    const user = userEvent.setup();
    checkLocalInstallerUpdatesMock.mockResolvedValue({
      currentVersion: "2.0.0",
      hasUpdate: false,
      latest: null,
      packages: [],
      searchedDirs: ["D:\\Grisia Studio\\Manga Library\\release"],
    });

    render(
      <SettingsView paths={defaultLibraryPaths} appVersion="2.0.0" />,
    );

    await user.click(screen.getByRole("button", { name: "检查更新" }));

    await waitFor(() => {
      expect(screen.getByTestId("update-status")).toHaveTextContent(
        "未找到更新包",
      );
    });
  });

  it("picks a bookshelf folder and saves the derived index path", async () => {
    const user = userEvent.setup();
    const onSavePaths = vi.fn();
    pickDirectoryMock.mockResolvedValue("D:\\Comics");

    render(
      <SettingsView
        paths={defaultLibraryPaths}
        appVersion="2.0.0"
        onSavePaths={onSavePaths}
      />,
    );

    await user.click(screen.getByRole("button", { name: "选择文件夹" }));
    await waitFor(() => {
      expect(onSavePaths).toHaveBeenCalledWith({
        bookshelfRoot: "D:\\Comics",
        databasePath: "D:\\Comics\\manga-library.sqlite",
        extraRoots: [],
      });
    });
  });
});
