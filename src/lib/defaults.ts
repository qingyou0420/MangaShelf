import type {
  LibraryComic,
  LibraryPaths,
  LibraryViewPrefs,
  ReaderDefaults,
} from "./types";

export const PATHS_STORAGE_KEY = "manga-library.paths";

export const defaultLibraryPaths: LibraryPaths = {
  bookshelfRoot: "E:\\书架",
  databasePath: "E:\\书架\\manga-library.sqlite",
  extraRoots: [],
};

export const sampleComic: LibraryComic = {
  id: "local:e:/书架/若世界處於黑夜",
  name: "若世界處於黑夜",
  location: "若世界處於黑夜",
  author: "むちまろ",
  tags: ["むちまろ"],
  localPath: "E:\\书架\\若世界處於黑夜",
  chapterCount: 3,
  imageCount: 90,
  latestChapterTitle: "第03话",
  readProgressPage: 0,
  scanStatus: "matched",
  favorited: false,
  readingDirection: "ltr",
  fitMode: "contain",
  readMode: "page",
};

export function loadStoredPaths(): LibraryPaths {
  try {
    const raw = window.localStorage.getItem(PATHS_STORAGE_KEY);
    if (!raw) {
      return { ...defaultLibraryPaths, extraRoots: [] };
    }
    const parsed = JSON.parse(raw) as Partial<LibraryPaths>;
    return {
      bookshelfRoot:
        parsed.bookshelfRoot?.trim() || defaultLibraryPaths.bookshelfRoot,
      databasePath:
        parsed.databasePath?.trim() || defaultLibraryPaths.databasePath,
      extraRoots: Array.isArray(parsed.extraRoots)
        ? parsed.extraRoots.filter((root) => root.trim())
        : [],
    };
  } catch {
    return { ...defaultLibraryPaths, extraRoots: [] };
  }
}

export function cacheRootForPath(
  filePath: string,
  paths: LibraryPaths,
): string {
  const normalized = filePath.replace(/\//g, "\\").toLowerCase();
  for (const extra of paths.extraRoots ?? []) {
    const root = extra.replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();
    if (normalized === root || normalized.startsWith(`${root}\\`)) {
      return extra;
    }
  }
  return paths.bookshelfRoot;
}

export function databasePathFor(bookshelfRoot: string): string {
  const trimmed = bookshelfRoot.trim().replace(/[\\/]+$/, "");
  if (!trimmed) {
    return defaultLibraryPaths.databasePath;
  }
  return `${trimmed}\\manga-library.sqlite`;
}

export function saveStoredPaths(paths: LibraryPaths): LibraryPaths {
  const bookshelfRoot =
    paths.bookshelfRoot.trim() || defaultLibraryPaths.bookshelfRoot;
  const next = {
    bookshelfRoot,
    databasePath: paths.databasePath.trim() || databasePathFor(bookshelfRoot),
    extraRoots: (paths.extraRoots ?? []).map((root) => root.trim()).filter(Boolean),
  };
  window.localStorage.setItem(PATHS_STORAGE_KEY, JSON.stringify(next));
  return next;
}

export const READER_DEFAULTS_KEY = "manga-library.reader-defaults";
export const LIBRARY_PREFS_KEY = "manga-library.library-prefs.v2";

export const defaultReaderDefaults: ReaderDefaults = {
  readingDirection: "ltr",
  fitMode: "contain",
  readMode: "page",
};

export function loadReaderDefaults(): ReaderDefaults {
  try {
    const raw = window.localStorage.getItem(READER_DEFAULTS_KEY);
    if (!raw) {
      return { ...defaultReaderDefaults };
    }
    const parsed = JSON.parse(raw) as Partial<ReaderDefaults>;
    return {
      readingDirection:
        parsed.readingDirection === "rtl" ? "rtl" : "ltr",
      fitMode:
        parsed.fitMode === "width" ||
        parsed.fitMode === "height" ||
        parsed.fitMode === "original"
          ? parsed.fitMode
          : "contain",
      readMode:
        parsed.readMode === "scroll" || parsed.readMode === "spread"
          ? parsed.readMode
          : "page",
    };
  } catch {
    return { ...defaultReaderDefaults };
  }
}

export function saveReaderDefaults(defaults: ReaderDefaults): ReaderDefaults {
  window.localStorage.setItem(READER_DEFAULTS_KEY, JSON.stringify(defaults));
  return defaults;
}

export const defaultLibraryViewPrefs: LibraryViewPrefs = {
  filter: "all",
  sort: "updated",
  sortDesc: false,
  query: "",
};

export function loadLibraryViewPrefs(): LibraryViewPrefs {
  try {
    const raw = window.localStorage.getItem(LIBRARY_PREFS_KEY);
    if (!raw) {
      return { ...defaultLibraryViewPrefs };
    }
    const parsed = JSON.parse(raw) as Partial<LibraryViewPrefs>;
    return {
      filter:
        parsed.filter === "favorited" ||
        parsed.filter === "recent" ||
        parsed.filter === "missing"
          ? parsed.filter
          : "all",
      sort:
        parsed.sort === "name" ||
        parsed.sort === "recent" ||
        parsed.sort === "chapters"
          ? parsed.sort
          : "updated",
      sortDesc: parsed.sortDesc === true,
      query: parsed.query ?? "",
    };
  } catch {
    return { ...defaultLibraryViewPrefs };
  }
}

export function saveLibraryViewPrefs(prefs: LibraryViewPrefs): void {
  window.localStorage.setItem(LIBRARY_PREFS_KEY, JSON.stringify(prefs));
}

export const THEME_KEY = "manga-library.theme";

export function loadTheme(): "light" | "dark" {
  try {
    return window.localStorage.getItem(THEME_KEY) === "dark" ? "dark" : "light";
  } catch {
    return "light";
  }
}

export function saveTheme(theme: "light" | "dark"): "light" | "dark" {
  window.localStorage.setItem(THEME_KEY, theme);
  return theme;
}
