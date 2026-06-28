import type {
  FavoriteUpdateRecoveryEvent,
  FavoriteUpdateRecoveryEventKind,
  FavoriteUpdateRecoveryStoppedReason,
  RecoveringFavoriteUpdateResult,
} from "../../lib/types";

export const RECOVERY_RUN_HISTORY_STORAGE_KEY =
  "mangacon-companion:recovery-run-history:v1";

const MAX_STORED_RECOVERY_EVENTS = 120;

export type RecoveryRunHistoryStatus = "running" | "completed" | "needs_attention";

export interface RecoveryRunHistoryRecord {
  savedAt: string;
  status: RecoveryRunHistoryStatus;
  result?: RecoveringFavoriteUpdateResult;
  events: FavoriteUpdateRecoveryEvent[];
}

export interface RecoveryRunHistoryInput {
  result?: RecoveringFavoriteUpdateResult;
  events?: FavoriteUpdateRecoveryEvent[];
  now?: Date;
}

type RecoveryRunHistoryStorage = Pick<
  Storage,
  "getItem" | "removeItem" | "setItem"
>;

const recoveryEventKinds = new Set<FavoriteUpdateRecoveryEventKind>([
  "started",
  "run_completed",
  "comic_downloaded",
  "comic_skipped",
  "error",
  "restarted",
  "completed",
  "restart_limit_reached",
]);

const stoppedReasons = new Set<FavoriteUpdateRecoveryStoppedReason>([
  "completed",
  "restart_limit_reached",
]);

export function buildRecoveryRunHistoryRecord({
  result,
  events = result?.events ?? [],
  now = new Date(),
}: RecoveryRunHistoryInput): RecoveryRunHistoryRecord {
  return {
    savedAt: now.toISOString(),
    status: historyStatusFromResult(result),
    result,
    events: events.slice(-MAX_STORED_RECOVERY_EVENTS),
  };
}

export function saveRecoveryRunHistory(
  record: RecoveryRunHistoryRecord,
  storage = getBrowserStorage(),
) {
  if (!storage) {
    return;
  }

  storage.setItem(RECOVERY_RUN_HISTORY_STORAGE_KEY, JSON.stringify(record));
}

export function loadRecoveryRunHistory(
  storage = getBrowserStorage(),
): RecoveryRunHistoryRecord | undefined {
  if (!storage) {
    return undefined;
  }

  const raw = storage.getItem(RECOVERY_RUN_HISTORY_STORAGE_KEY);
  if (!raw) {
    return undefined;
  }

  try {
    const parsed: unknown = JSON.parse(raw);
    if (isRecoveryRunHistoryRecord(parsed)) {
      return parsed;
    }
  } catch {
    // Fall through to clear invalid persisted state.
  }

  storage.removeItem(RECOVERY_RUN_HISTORY_STORAGE_KEY);
  return undefined;
}

export function clearRecoveryRunHistory(storage = getBrowserStorage()) {
  storage?.removeItem(RECOVERY_RUN_HISTORY_STORAGE_KEY);
}

function historyStatusFromResult(
  result: RecoveringFavoriteUpdateResult | undefined,
): RecoveryRunHistoryStatus {
  if (!result) {
    return "running";
  }

  return result.stoppedReason === "completed" ? "completed" : "needs_attention";
}

function getBrowserStorage(): RecoveryRunHistoryStorage | undefined {
  if (typeof window === "undefined") {
    return undefined;
  }

  return window.localStorage;
}

function isRecoveryRunHistoryRecord(
  value: unknown,
): value is RecoveryRunHistoryRecord {
  if (!isRecord(value)) {
    return false;
  }

  return (
    typeof value.savedAt === "string" &&
    isRecoveryRunHistoryStatus(value.status) &&
    Array.isArray(value.events) &&
    value.events.every(isRecoveryEvent) &&
    (value.result === undefined || isRecoveryResult(value.result))
  );
}

function isRecoveryRunHistoryStatus(
  value: unknown,
): value is RecoveryRunHistoryStatus {
  return value === "running" || value === "completed" || value === "needs_attention";
}

function isRecoveryResult(value: unknown): value is RecoveringFavoriteUpdateResult {
  if (!isRecord(value)) {
    return false;
  }

  return (
    typeof value.requestedLimit === "number" &&
    typeof value.maxRestarts === "number" &&
    typeof value.restarts === "number" &&
    typeof value.processed === "number" &&
    typeof value.downloadedChapters === "number" &&
    typeof value.skippedCount === "number" &&
    stoppedReasons.has(value.stoppedReason as FavoriteUpdateRecoveryStoppedReason) &&
    (value.lastError === null || typeof value.lastError === "string") &&
    Array.isArray(value.events) &&
    value.events.every(isRecoveryEvent) &&
    Array.isArray(value.runs)
  );
}

function isRecoveryEvent(value: unknown): value is FavoriteUpdateRecoveryEvent {
  if (!isRecord(value)) {
    return false;
  }

  return (
    recoveryEventKinds.has(value.kind as FavoriteUpdateRecoveryEventKind) &&
    typeof value.message === "string" &&
    typeof value.processed === "number" &&
    typeof value.downloadedChapters === "number" &&
    typeof value.skippedCount === "number" &&
    typeof value.restarts === "number"
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
