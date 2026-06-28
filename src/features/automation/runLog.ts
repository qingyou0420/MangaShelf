import type {
  FavoriteUpdateRecoveryEvent,
  FavoriteUpdateRecoveryEventKind,
  RecoveringFavoriteUpdateResult,
} from "../../lib/types";

export type RunLogTone = "info" | "success" | "warning" | "danger";

export interface RecoveryRunLogEntry {
  tone: RunLogTone;
  title: string;
  detail: string;
}

export interface RecoveryRunSnapshotOptions {
  maxEntries?: number;
  fallbackMaxRestarts?: number;
}

export interface RecoveryRunSnapshot {
  entries: RecoveryRunLogEntry[];
  hiddenEntries: number;
  processed: number;
  downloadedChapters: number;
  skippedCount: number;
  restarts: number;
  maxRestarts: number;
}

const DEFAULT_MAX_RECOVERY_LOG_ENTRIES = 120;
const DEFAULT_MAX_RESTARTS = 2;

const eventToneByKind: Record<FavoriteUpdateRecoveryEventKind, RunLogTone> = {
  started: "info",
  run_completed: "success",
  comic_downloaded: "success",
  comic_skipped: "warning",
  error: "danger",
  restarted: "warning",
  completed: "success",
  restart_limit_reached: "danger",
};

export function buildRecoveryRunLog(
  result?: RecoveringFavoriteUpdateResult,
  realtimeEvents: FavoriteUpdateRecoveryEvent[] = [],
): RecoveryRunLogEntry[] {
  return recoveryEventsFromInputs(result, realtimeEvents).map(logEntryFromRecoveryEvent);
}

export function buildRecoveryRunSnapshot(
  result?: RecoveringFavoriteUpdateResult,
  realtimeEvents: FavoriteUpdateRecoveryEvent[] = [],
  options: RecoveryRunSnapshotOptions = {},
): RecoveryRunSnapshot {
  const events = recoveryEventsFromInputs(result, realtimeEvents);
  const maxEntries = Math.max(
    1,
    options.maxEntries ?? DEFAULT_MAX_RECOVERY_LOG_ENTRIES,
  );
  const entries = events.map(logEntryFromRecoveryEvent);
  const hiddenEntries = Math.max(0, entries.length - maxEntries);
  const latestEvent = events.at(-1);

  return {
    entries: entries.slice(hiddenEntries),
    hiddenEntries,
    processed: result?.processed ?? latestEvent?.processed ?? 0,
    downloadedChapters:
      result?.downloadedChapters ?? latestEvent?.downloadedChapters ?? 0,
    skippedCount: result?.skippedCount ?? latestEvent?.skippedCount ?? 0,
    restarts: result?.restarts ?? latestEvent?.restarts ?? 0,
    maxRestarts:
      result?.maxRestarts ?? options.fallbackMaxRestarts ?? DEFAULT_MAX_RESTARTS,
  };
}

function recoveryEventsFromInputs(
  result?: RecoveringFavoriteUpdateResult,
  realtimeEvents: FavoriteUpdateRecoveryEvent[] = [],
): FavoriteUpdateRecoveryEvent[] {
  if (!result && realtimeEvents.length === 0) {
    return [];
  }

  const events = result?.events.length ? result.events : realtimeEvents;
  if (events.length > 0) {
    return events;
  }

  return [
    {
      kind:
        result?.stoppedReason === "completed"
          ? "completed"
          : "restart_limit_reached",
      message:
        result?.stoppedReason === "completed"
          ? "自动恢复长跑完成"
          : "已达到自动重启上限",
      processed: result?.processed ?? 0,
      downloadedChapters: result?.downloadedChapters ?? 0,
      skippedCount: result?.skippedCount ?? 0,
      restarts: result?.restarts ?? 0,
    },
  ];
}

export function recoveryNeedsAttention(
  result?: RecoveringFavoriteUpdateResult,
  error?: string,
): boolean {
  if (error) {
    return true;
  }

  return result?.stoppedReason === "restart_limit_reached";
}

function logEntryFromRecoveryEvent(
  event: FavoriteUpdateRecoveryEvent,
): RecoveryRunLogEntry {
  return {
    tone: eventToneByKind[event.kind],
    title: event.message,
    detail: `已处理 ${event.processed} 本，下载 ${event.downloadedChapters} 话，跳过 ${event.skippedCount} 本，重启 ${event.restarts} 次`,
  };
}
