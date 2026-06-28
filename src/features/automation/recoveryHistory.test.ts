import { describe, expect, it } from "vitest";
import type {
  FavoriteUpdateRecoveryEvent,
  RecoveringFavoriteUpdateResult,
} from "../../lib/types";
import {
  RECOVERY_RUN_HISTORY_STORAGE_KEY,
  buildRecoveryRunHistoryRecord,
  clearRecoveryRunHistory,
  loadRecoveryRunHistory,
  saveRecoveryRunHistory,
} from "./recoveryHistory";

const completedResult: RecoveringFavoriteUpdateResult = {
  requestedLimit: 500,
  maxRestarts: 2,
  restarts: 1,
  processed: 12,
  downloadedChapters: 30,
  skippedCount: 2,
  stoppedReason: "completed",
  lastError: "漫画控窗口无响应",
  events: [
    {
      kind: "started",
      message: "开始自动恢复长跑，目标 500 本",
      processed: 0,
      downloadedChapters: 0,
      skippedCount: 0,
      restarts: 0,
    },
    {
      kind: "completed",
      message: "自动恢复长跑完成",
      processed: 12,
      downloadedChapters: 30,
      skippedCount: 2,
      restarts: 1,
    },
  ],
  runs: [],
};

function createStorage(): Storage {
  const items = new Map<string, string>();
  return {
    get length() {
      return items.size;
    },
    clear: () => items.clear(),
    getItem: (key) => items.get(key) ?? null,
    key: (index) => Array.from(items.keys())[index] ?? null,
    removeItem: (key) => items.delete(key),
    setItem: (key, value) => items.set(key, value),
  };
}

describe("recovery history", () => {
  it("saves and loads the latest completed recovery run", () => {
    const storage = createStorage();
    const record = buildRecoveryRunHistoryRecord({
      result: completedResult,
      now: new Date("2026-06-28T08:00:00.000Z"),
    });

    saveRecoveryRunHistory(record, storage);

    expect(loadRecoveryRunHistory(storage)).toEqual({
      savedAt: "2026-06-28T08:00:00.000Z",
      status: "completed",
      result: completedResult,
      events: completedResult.events,
    });
  });

  it("drops corrupt storage instead of returning unsafe data", () => {
    const storage = createStorage();
    storage.setItem(RECOVERY_RUN_HISTORY_STORAGE_KEY, "{broken");

    expect(loadRecoveryRunHistory(storage)).toBeUndefined();
    expect(storage.getItem(RECOVERY_RUN_HISTORY_STORAGE_KEY)).toBeNull();
  });

  it("keeps only recent live events for a recoverable running snapshot", () => {
    const events: FavoriteUpdateRecoveryEvent[] = Array.from(
      { length: 126 },
      (_, index) => ({
        kind: "comic_downloaded",
        message: `第 ${index + 1} 本已交给漫画控，下载 1 话`,
        processed: index + 1,
        downloadedChapters: index + 1,
        skippedCount: 0,
        restarts: 0,
      }),
    );

    const record = buildRecoveryRunHistoryRecord({
      events,
      now: new Date("2026-06-28T08:30:00.000Z"),
    });

    expect(record.status).toBe("running");
    expect(record.result).toBeUndefined();
    expect(record.events).toHaveLength(120);
    expect(record.events[0].message).toBe("第 7 本已交给漫画控，下载 1 话");
    expect(record.events.at(-1)?.message).toBe("第 126 本已交给漫画控，下载 1 话");
  });

  it("clears the stored recovery run", () => {
    const storage = createStorage();
    const record = buildRecoveryRunHistoryRecord({
      result: completedResult,
      now: new Date("2026-06-28T08:00:00.000Z"),
    });

    saveRecoveryRunHistory(record, storage);
    clearRecoveryRunHistory(storage);

    expect(loadRecoveryRunHistory(storage)).toBeUndefined();
  });
});
