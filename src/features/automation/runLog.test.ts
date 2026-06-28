import { describe, expect, it } from "vitest";
import type { RecoveringFavoriteUpdateResult } from "../../lib/types";
import {
  buildRecoveryRunLog,
  buildRecoveryRunSnapshot,
  recoveryNeedsAttention,
} from "./runLog";

const completedRecovery: RecoveringFavoriteUpdateResult = {
  requestedLimit: 500,
  maxRestarts: 2,
  restarts: 1,
  processed: 5,
  downloadedChapters: 8,
  skippedCount: 1,
  stoppedReason: "completed",
  lastError: "漫画控窗口无响应",
  events: [
    {
      kind: "started",
      message: "开始自动恢复长跑",
      processed: 0,
      downloadedChapters: 0,
      skippedCount: 0,
      restarts: 0,
    },
    {
      kind: "error",
      message: "漫画控窗口无响应",
      processed: 2,
      downloadedChapters: 3,
      skippedCount: 1,
      restarts: 0,
    },
    {
      kind: "restarted",
      message: "漫画控已重启，等待红点刷新",
      processed: 2,
      downloadedChapters: 3,
      skippedCount: 1,
      restarts: 1,
    },
    {
      kind: "completed",
      message: "自动恢复长跑完成",
      processed: 5,
      downloadedChapters: 8,
      skippedCount: 1,
      restarts: 1,
    },
  ],
  runs: [],
};

describe("automation run log", () => {
  it("builds readable recovery log entries from backend events", () => {
    const entries = buildRecoveryRunLog(completedRecovery);

    expect(entries).toEqual([
      {
        tone: "info",
        title: "开始自动恢复长跑",
        detail: "已处理 0 本，下载 0 话，跳过 0 本，重启 0 次",
      },
      {
        tone: "danger",
        title: "漫画控窗口无响应",
        detail: "已处理 2 本，下载 3 话，跳过 1 本，重启 0 次",
      },
      {
        tone: "warning",
        title: "漫画控已重启，等待红点刷新",
        detail: "已处理 2 本，下载 3 话，跳过 1 本，重启 1 次",
      },
      {
        tone: "success",
        title: "自动恢复长跑完成",
        detail: "已处理 5 本，下载 8 话，跳过 1 本，重启 1 次",
      },
    ]);
  });

  it("marks restart-limit runs as needing attention", () => {
    expect(
      recoveryNeedsAttention({
        ...completedRecovery,
        stoppedReason: "restart_limit_reached",
      }),
    ).toBe(true);
    expect(recoveryNeedsAttention(completedRecovery)).toBe(false);
  });

  it("builds readable entries for per-comic live events", () => {
    const entries = buildRecoveryRunLog(undefined, [
      {
        kind: "comic_downloaded",
        message: "第 2 本已交给漫画控，下载 4 话",
        processed: 2,
        downloadedChapters: 7,
        skippedCount: 1,
        restarts: 0,
      },
      {
        kind: "comic_skipped",
        message: "跳过 1 本：详情页没有更新红点",
        processed: 2,
        downloadedChapters: 7,
        skippedCount: 2,
        restarts: 0,
      },
    ]);

    expect(entries).toEqual([
      {
        tone: "success",
        title: "第 2 本已交给漫画控，下载 4 话",
        detail: "已处理 2 本，下载 7 话，跳过 1 本，重启 0 次",
      },
      {
        tone: "warning",
        title: "跳过 1 本：详情页没有更新红点",
        detail: "已处理 2 本，下载 7 话，跳过 2 本，重启 0 次",
      },
    ]);
  });

  it("builds capped run snapshot from the latest realtime event", () => {
    const snapshot = buildRecoveryRunSnapshot(
      undefined,
      [
        {
          kind: "started",
          message: "开始自动恢复长跑，目标 500 本",
          processed: 0,
          downloadedChapters: 0,
          skippedCount: 0,
          restarts: 0,
        },
        {
          kind: "comic_downloaded",
          message: "第 1 本已交给漫画控，下载 2 话",
          processed: 1,
          downloadedChapters: 2,
          skippedCount: 0,
          restarts: 0,
        },
        {
          kind: "comic_skipped",
          message: "跳过 1 本：详情页没有更新红点",
          processed: 1,
          downloadedChapters: 2,
          skippedCount: 1,
          restarts: 0,
        },
        {
          kind: "comic_downloaded",
          message: "第 2 本已交给漫画控，下载 4 话",
          processed: 2,
          downloadedChapters: 6,
          skippedCount: 1,
          restarts: 0,
        },
      ],
      { maxEntries: 2, fallbackMaxRestarts: 2 },
    );

    expect(snapshot.processed).toBe(2);
    expect(snapshot.downloadedChapters).toBe(6);
    expect(snapshot.skippedCount).toBe(1);
    expect(snapshot.restarts).toBe(0);
    expect(snapshot.maxRestarts).toBe(2);
    expect(snapshot.hiddenEntries).toBe(2);
    expect(snapshot.entries).toHaveLength(2);
    expect(snapshot.entries[0].title).toBe("跳过 1 本：详情页没有更新红点");
    expect(snapshot.entries[1].title).toBe("第 2 本已交给漫画控，下载 4 话");
  });
});
