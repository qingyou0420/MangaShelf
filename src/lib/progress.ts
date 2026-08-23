import type { LibraryComic } from "./types";

export function chapterTitleFromId(
  comicId: string,
  chapterId?: string | null,
): string | undefined {
  if (!chapterId) {
    return undefined;
  }
  const prefix = `${comicId}::`;
  if (!chapterId.startsWith(prefix)) {
    return undefined;
  }
  const title = chapterId.slice(prefix.length).trim();
  return title || undefined;
}

export function lastReadChapterLabel(
  comic: Pick<
    LibraryComic,
    "id" | "lastReadChapterId" | "lastReadChapterTitle"
  >,
): string | undefined {
  const stored = comic.lastReadChapterTitle?.trim();
  if (stored) {
    return stored;
  }
  return chapterTitleFromId(comic.id, comic.lastReadChapterId);
}

export function coverProgress(comic: LibraryComic): number {
  if (!comic.lastReadAt || comic.chapterCount <= 0) {
    return 0;
  }
  const chapterCount = comic.chapterCount;
  const pages = Math.max(comic.lastReadChapterPages ?? 0, 1);
  const pageFrac = Math.min(1, (comic.readProgressPage + 1) / pages);
  if (comic.lastReadChapterOrdinal && comic.lastReadChapterOrdinal > 0) {
    const index = Math.min(comic.lastReadChapterOrdinal, chapterCount) - 1;
    return Math.min(1, (Math.max(index, 0) + pageFrac) / chapterCount);
  }
  const title = lastReadChapterLabel(comic) ?? "";
  const digits = title.match(/\d+/);
  if (!digits) {
    return Math.max(0.08, pageFrac / chapterCount);
  }
  const n = Number(digits[0]);
  if (!Number.isFinite(n) || n <= 0) {
    return 0.08;
  }
  return Math.min(1, (Math.min(n, chapterCount) - 1 + pageFrac) / chapterCount);
}
