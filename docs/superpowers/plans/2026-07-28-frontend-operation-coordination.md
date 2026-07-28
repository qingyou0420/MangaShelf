# Frontend Operation Coordination Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the dashboard and legacy automation page one operation owner, prevent stale asynchronous completion from clearing newer state, derive task counts from the latest database status, and make repair monitoring restartable, retryable, and explicit on timeout.

**Architecture:** Add two small hooks rather than a global state library. `useOperationController` owns the active operation and monotonically increasing run ID; `useRepairMonitor` owns one monitor generation and delegates policy decisions to a pure function. Existing App handlers and AutomationView JSX remain in place and consume these focused interfaces.

**Tech Stack:** React 19, TypeScript 5.8, Vitest 4, Testing Library, fake timers.

## Global Constraints

- This is a private single-user desktop application; do not add Redux, Zustand, XState, a router, or background services.
- Preserve the existing import, bookshelf, queue, repair, resume, status, and legacy automation behavior.
- Do not merge import and bookshelf scan.
- Do not change Tauri commands or the recovery event protocol in this plan.
- Use one frontend operation controller shared by App and AutomationView.
- The controller must reject overlap synchronously in the same event loop and clear state only for the current run ID.
- A new repair-monitor cycle resets all counters and invalidates old timers and in-flight responses.
- Keep the existing 30-second normal interval, 120-check maximum, and 200-task repair limit.
- Retry status-read failures after 5, 15, and 30 seconds, then stop with a visible error.
- A foreground operation delays monitor work without consuming a check.
- Preserve existing user-owned `README.md` and development-document changes.
- Write each behavior test first and observe its expected RED failure.

---

### Task 1: Build the Run-ID Operation Controller

**Files:**
- Create: `src/hooks/useOperationController.ts`
- Create: `src/hooks/useOperationController.test.tsx`

**Interfaces:**
- Produces:

```ts
export type OperationKind =
  | "import-favorites"
  | "scan-bookshelf"
  | "queue-updates"
  | "resume-unfinished"
  | "repair-failed"
  | "refresh-task-status"
  | "auto-repair"
  | `automation:${string}`;

export interface ActiveOperation {
  runId: number;
  kind: OperationKind;
  startedAt: number;
}

export interface OperationController {
  active?: ActiveOperation;
  tryStart(kind: OperationKind): ActiveOperation | undefined;
  finish(runId: number): void;
  isCurrent(runId: number): boolean;
  isBusy(): boolean;
}

export function useOperationController(): OperationController;
```

- [ ] **Step 1: Write the synchronous exclusion test**

Using `renderHook`, call `tryStart("scan-bookshelf")` and immediately call `tryStart("import-favorites")` inside the same `act`.

Assert:

```ts
expect(first).toMatchObject({ runId: 1, kind: "scan-bookshelf" });
expect(second).toBeUndefined();
expect(result.current.active?.kind).toBe("scan-bookshelf");
```

This test catches an implementation that relies only on asynchronous React state.

- [ ] **Step 2: Run the exclusion test to verify RED**

Run:

```powershell
npm test -- --run src/hooks/useOperationController.test.tsx
```

Expected: module-not-found failure because the hook does not exist.

- [ ] **Step 3: Write the stale-finally test**

Start run 1, finish run 1, start run 2, then call `finish(run1.runId)` again. Assert run 2 remains active and `isCurrent(run2.runId)` is true.

The production change that makes this test pass is checking the active run ID before clearing both the ref and state.

- [ ] **Step 4: Implement the minimal controller**

Use one ref for synchronous ownership, one state value for rendering, and one monotonically increasing ref:

```ts
const activeRef = useRef<ActiveOperation | undefined>(undefined);
const nextRunIdRef = useRef(1);
const [active, setActive] = useState<ActiveOperation | undefined>(undefined);
```

`tryStart` returns `undefined` when `activeRef.current` exists. `finish` clears only when `activeRef.current?.runId === runId`. Wrap methods in `useCallback` and the returned object in `useMemo` so consumers do not receive a new controller on every render.

- [ ] **Step 5: Verify GREEN**

Run:

```powershell
npm test -- --run src/hooks/useOperationController.test.tsx
```

Expected: synchronous exclusion, release, monotonic run IDs, and stale-finally tests pass.

- [ ] **Step 6: Commit**

```powershell
git add -- src/hooks/useOperationController.ts src/hooks/useOperationController.test.tsx
git commit -m "feat: add frontend operation controller"
```

---

### Task 2: Coordinate App and Dashboard Operations

**Files:**
- Modify: `src/App.tsx:56-354`
- Modify: `src/features/dashboard/Dashboard.tsx:17-117`
- Test: `src/App.test.tsx:206-437`
- Test: `src/features/dashboard/Dashboard.test.tsx:50-102`

**Interfaces:**
- Consumes: `useOperationController`.
- Changes `DashboardProps` to accept:

```ts
activeOperation?: ActiveOperation;
```

The existing individual `isImporting`, `isScanning`, `isUpdating`, `isRepairing`, `isResuming`, and `isRefreshingTaskStatus` props are removed after their visual state is derived from `activeOperation.kind`.

- [ ] **Step 1: Write the dashboard overlap test**

Create a deferred `syncBookshelfMatches` promise. Render App, click `扫描本地书架`, and while the promise is unresolved assert all dashboard action buttons are disabled:

```ts
for (const name of [
  "一键更新收藏",
  "继续未完成下载",
  "修复失败图片",
  "扫描本地书架",
  "导入漫画控收藏",
  "刷新任务状态",
]) {
  expect(screen.getByRole("button", { name })).toBeDisabled();
}
```

Attempt the import click and assert `importFavoritesMock` was not called.

- [ ] **Step 2: Run the overlap test to verify RED**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: buttons other than the bookshelf button remain enabled and import can run.

- [ ] **Step 3: Add one App operation wrapper**

Create:

```ts
async function runAppOperation(
  kind: OperationKind,
  task: (run: ActiveOperation) => Promise<void>,
) {
  const run = operationController.tryStart(kind);
  if (!run) {
    return;
  }
  try {
    await task(run);
  } finally {
    operationController.finish(run.runId);
  }
}
```

Wrap import, bookshelf scan, queue, manual repair, resume, and manual status refresh. Remove the six duplicated boolean state values and derive loading props from `activeOperation?.kind`.

Split the current nested status refresh:

```ts
async function fetchAndApplyTaskStatus(announce: boolean): Promise<MangaConTaskStatus>;
```

The public refresh handler acquires `"refresh-task-status"` and calls this helper. Resume calls the helper inside its existing `"resume-unfinished"` run instead of attempting a nested operation.

- [ ] **Step 4: Disable the whole dashboard conflict group**

In `Dashboard`, compute:

```ts
const isOperationActive = activeOperation !== undefined;
```

All six action buttons use `disabled={isOperationActive}`. Button labels may still use `activeOperation.kind` to show which action is running, but no new spinner component is required.

- [ ] **Step 5: Verify GREEN**

Run:

```powershell
npm test -- --run src/App.test.tsx src/features/dashboard/Dashboard.test.tsx
```

Expected: overlap is rejected, nested resume refresh still works, and all existing callbacks and messages remain covered.

- [ ] **Step 6: Commit**

```powershell
git add -- src/App.tsx src/App.test.tsx src/features/dashboard/Dashboard.tsx src/features/dashboard/Dashboard.test.tsx
git commit -m "feat: coordinate dashboard operations"
```

---

### Task 3: Share the Controller with AutomationView

**Files:**
- Modify: `src/App.tsx:421-426`
- Modify: `src/features/automation/AutomationView.tsx:75-207`
- Modify: `src/features/automation/AutomationView.tsx:234-420`
- Modify: `src/features/automation/AutomationView.tsx:486-620`
- Modify: `src/features/automation/AutomationView.tsx:946-972`
- Test: `src/features/automation/AutomationView.test.tsx:45-536`
- Test: `src/App.test.tsx`

**Interfaces:**
- Consumes: `OperationController`.
- Adds:

```ts
interface AutomationViewProps {
  // existing props
  operationController?: OperationController;
}
```

AutomationView always creates an internal controller for standalone tests, then selects the passed App controller when present:

```ts
const internalOperationController = useOperationController();
const controller = operationController ?? internalOperationController;
```

- [ ] **Step 1: Write the all-buttons-disabled automation test**

Make `service.findWindows()` return a deferred promise. Click `查找漫画控窗口`, then assert `重启漫画控`, `刷新状态`, and `自动恢复更新全部` are disabled.

- [ ] **Step 2: Run the automation test to verify RED**

Run:

```powershell
npm test -- --run src/features/automation/AutomationView.test.tsx
```

Expected: only the find button is disabled because existing buttons compare `busyAction` with their own action string.

- [ ] **Step 3: Write the non-overwrite test**

While the deferred find action is active, attempt to click restart and assert `service.restart` is not called. Resolve find and assert the operation clears once.

- [ ] **Step 4: Write the cross-navigation App test**

Start a deferred `triggerAllFavoriteUpdatesWithRecovery`, navigate to Dashboard, assert all dashboard actions remain disabled, navigate back to Automation, and assert restart/recovery controls are still disabled.

This test catches losing operation ownership when the conditionally mounted AutomationView unmounts.

- [ ] **Step 5: Replace local busy ownership**

Change `runAction` without rewriting the existing handlers:

```ts
async function runAction(action: string, callback: () => Promise<void>) {
  const run = controller.tryStart(`automation:${action}`);
  if (!run) {
    return;
  }
  setError(undefined);
  try {
    await callback();
  } catch (cause) {
    if (controller.isCurrent(run.runId)) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  } finally {
    controller.finish(run.runId);
  }
}
```

Derive `busyAction` from `controller.active?.kind` only for label text. Every MangaCon action button uses `disabled={controller.isBusy()}`.

- [ ] **Step 6: Verify GREEN**

Run:

```powershell
npm test -- --run src/features/automation/AutomationView.test.tsx src/App.test.tsx
```

Expected: same-page overlap, cross-navigation overlap, stale completion protection, recovery events, and normal unlisten tests pass.

- [ ] **Step 7: Commit**

```powershell
git add -- src/App.tsx src/App.test.tsx src/features/automation/AutomationView.tsx src/features/automation/AutomationView.test.tsx
git commit -m "feat: share operation ownership with automation"
```

---

### Task 4: Derive the Task Card from Current Status

**Files:**
- Modify: `src/App.tsx:68-70`
- Modify: `src/App.tsx:199-354`
- Modify: `src/App.tsx:397-416`
- Test: `src/App.test.tsx:309-437`

**Interfaces:**
- Replaces `queuedUpdateCount` with:

```ts
const [optimisticPendingTasks, setOptimisticPendingTasks] = useState(0);
const pendingTasks =
  mangaConTaskStatus?.activeTasks ?? optimisticPendingTasks;
```

- [ ] **Step 1: Write the status-authority test**

Queue returns `queued: 33`. The first monitor status returns `activeTasks: 7`; the next status returns 0. Advance fake timers and assert the task metric changes from optimistic 33 to 7 and then 0.

- [ ] **Step 2: Run the authority test to verify RED**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: the task metric remains 33 because monitor updates only `mangaConTaskStatus`.

- [ ] **Step 3: Write the new-cycle fallback test**

First establish an old task status of 0. Start a new queue result with `queued: 5` and keep the first monitor request pending. Assert the task metric displays 5 before fresh status arrives.

The production change that makes this pass is clearing stale `mangaConTaskStatus` at the start of a new queue/repair/resume cycle before setting the optimistic count.

- [ ] **Step 4: Implement the single authority**

On queue, repair, and resume success:

```ts
setMangaConTaskStatus(undefined);
setOptimisticPendingTasks(newCount);
```

Every successful status read sets `mangaConTaskStatus`; no monitor path writes the optimistic value. Pass the derived `pendingTasks` to Dashboard.

- [ ] **Step 5: Verify GREEN and commit**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: optimistic fallback and live-status authority tests pass.

Commit:

```powershell
git add -- src/App.tsx src/App.test.tsx
git commit -m "fix: derive pending tasks from live status"
```

---

### Task 5: Implement Pure Repair-Monitor Decisions

**Files:**
- Create: `src/hooks/useRepairMonitor.ts`
- Create: `src/hooks/useRepairMonitor.test.ts`

**Interfaces:**
- Produces:

```ts
export interface RepairMonitorPolicy {
  intervalMs: number;
  maxChecks: number;
  retryDelaysMs: readonly number[];
  deadlineMs: number;
}

export type RepairMonitorDecision =
  | { kind: "schedule"; delayMs: number; checks: number; retries: number }
  | { kind: "retry"; delayMs: number; checks: number; retries: number }
  | { kind: "repair"; checks: number; retries: number }
  | { kind: "complete"; checks: number; retries: number }
  | { kind: "timeout"; reason: "checks" | "deadline" | "status-errors" };

export type RepairMonitorTimeoutReason =
  | "checks"
  | "deadline"
  | "status-errors";

export function decideRepairMonitorStep(input: {
  policy: RepairMonitorPolicy;
  checks: number;
  retries: number;
  now: number;
  deadline: number;
  operationBusy: boolean;
  status?: MangaConTaskStatus;
  statusError?: unknown;
}): RepairMonitorDecision;
```

- [ ] **Step 1: Write table-driven RED tests**

Use hand-derived literal expectations for:

```text
busy operation -> schedule normal interval, checks unchanged
active tasks below limit -> schedule normal interval, checks + 1
active tasks at maxChecks -> timeout/checks
now at deadline -> timeout/deadline
first error -> retry 5000 ms
second error -> retry 15000 ms
third error -> retry 30000 ms
fourth error -> timeout/status-errors
zero active + failed > 0 -> repair
zero active + zero failed -> complete
```

Do not compute expected delays using the function under test.

- [ ] **Step 2: Run decision tests to verify RED**

Run:

```powershell
npm test -- --run src/hooks/useRepairMonitor.test.ts
```

Expected: module-not-found failure.

- [ ] **Step 3: Implement the pure decision function**

Use the exact production policy:

```ts
export const DEFAULT_REPAIR_MONITOR_POLICY: RepairMonitorPolicy = {
  intervalMs: 30_000,
  maxChecks: 120,
  retryDelaysMs: [5_000, 15_000, 30_000],
  deadlineMs: 60 * 60 * 1000,
};
```

Check the deadline first. Before the deadline, `operationBusy` schedules another normal interval without consuming a check. An error consumes a retry but not a normal check. A successful status resets retries to 0.

- [ ] **Step 4: Verify GREEN**

Run:

```powershell
npm test -- --run src/hooks/useRepairMonitor.test.ts
```

Expected: all ten policy branches pass.

- [ ] **Step 5: Commit**

```powershell
git add -- src/hooks/useRepairMonitor.ts src/hooks/useRepairMonitor.test.ts
git commit -m "feat: define repair monitor policy"
```

---

### Task 6: Add the Generation-Safe Repair Monitor Hook

**Files:**
- Modify: `src/hooks/useRepairMonitor.ts`
- Modify: `src/hooks/useRepairMonitor.test.ts`

**Interfaces:**
- Adds:

```ts
export interface RepairMonitor {
  begin(initialDelayMs?: number): void;
  stop(): void;
  isRunning: boolean;
}

export function useRepairMonitor(options: {
  policy?: RepairMonitorPolicy;
  readStatus(): Promise<MangaConTaskStatus>;
  isOperationBusy(): boolean;
  onStatus(status: MangaConTaskStatus): void;
  onRepair(): Promise<"started" | "busy">;
  onComplete(): void;
  onTimeout(reason: RepairMonitorTimeoutReason): void;
  onRetry(error: unknown, retryNumber: number): void;
}): RepairMonitor;
```

- [ ] **Step 1: Write the new-generation reset test**

Use fake timers and a policy with `maxChecks: 2`, `intervalMs: 10`, and a short deadline. Run cycle A to timeout on active tasks. Call `begin()` again and advance two intervals. Assert cycle B performs two fresh status reads before timing out.

- [ ] **Step 2: Run the reset test to verify RED**

Run:

```powershell
npm test -- --run src/hooks/useRepairMonitor.test.ts
```

Expected: compile failure because the hook interface does not exist.

- [ ] **Step 3: Write retry, stale-response, and busy-operation tests**

Retry: first `readStatus` rejects, second resolves active; advance 5 seconds and assert two calls plus continued scheduling.

Stale response: leave cycle A's request deferred, call `begin()` for cycle B, resolve A, and assert A invokes none of `onStatus`, `onRepair`, `onComplete`, or `onTimeout`.

Busy operation: return failed tasks while `isOperationBusy()` is true; assert `onRepair` is not called and the next normal interval is scheduled without incrementing checks.

- [ ] **Step 4: Implement generation and timer ownership**

Use refs for timer ID, generation, checks, retries, deadline, and mounted state. `begin` must:

```text
clear old timer
increment generation
reset checks and retries
set deadline = Date.now() + policy.deadlineMs
schedule the first tick for that generation
```

Every timer callback and every Promise continuation compares its captured generation with the current generation before invoking callbacks or scheduling again. `stop` increments generation and clears the timer. Unmount calls `stop`.

If decision is `repair` and `onRepair()` returns `"busy"`, schedule the normal interval without consuming a check. `"started"` ends the monitor; the repair workflow starts a new cycle when it queues work.

- [ ] **Step 5: Verify GREEN**

Run:

```powershell
npm test -- --run src/hooks/useRepairMonitor.test.ts
```

Expected: reset, retry delays, stale response, busy delay, complete, repair, timeout, and unmount cleanup tests pass.

- [ ] **Step 6: Commit**

```powershell
git add -- src/hooks/useRepairMonitor.ts src/hooks/useRepairMonitor.test.ts
git commit -m "feat: add generation-safe repair monitor"
```

---

### Task 7: Replace App's Ad-Hoc Repair Timer

**Files:**
- Modify: `src/App.tsx:47-50`
- Modify: `src/App.tsx:75-76`
- Modify: `src/App.tsx:236-274`
- Test: `src/App.test.tsx:309-437`

**Interfaces:**
- Consumes: `useRepairMonitor` and `useOperationController`.
- Produces: no new public component interface.

- [ ] **Step 1: Write the real 120-check boundary test**

With fake timers, make 120 status reads return active tasks. Assert the UI displays an explicit message equivalent to:

```text
自动监控已达到 60 分钟上限，请手动刷新任务状态或修复失败图片
```

Assert no timer remains scheduled.

- [ ] **Step 2: Run the boundary test to verify RED**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: the current monitor silently stops and retains the prior “完成后将自动检查” message.

- [ ] **Step 3: Write the second-cycle and transient-error tests**

After the first cycle reaches its limit, trigger a second queue result and advance two intervals. Assert two fresh status reads occur.

For transient error, reject the first status read, advance 5 seconds, resolve the second with active tasks, and assert the monitor continues.

- [ ] **Step 4: Wire the hook**

Delete `repairMonitorTimerRef`, `repairMonitorChecksRef`, `startRepairMonitor`, and `monitorAndRepairFailedTasks`.

Configure the hook with:

```ts
readStatus: () => getMangaConTaskStatus({ mangaConDatabasePath: approvedDefaultPaths.mangaConDatabase }),
isOperationBusy: operationController.isBusy,
onStatus: setMangaConTaskStatus,
```

`onRepair` tries to start `"auto-repair"` through the shared controller. It returns `"busy"` when another operation owns the controller; otherwise it awaits the existing internal repair body and returns `"started"`.

Queue, repair, and resume call `repairMonitor.begin()` only after fresh work exists. Timeout and retry callbacks update the existing message area; no new panel is required.

- [ ] **Step 5: Verify GREEN**

Run:

```powershell
npm test -- --run src/App.test.tsx src/hooks/useRepairMonitor.test.ts
```

Expected: real boundary, second cycle, retry, auto-repair, manual repair, queue, and resume tests pass.

- [ ] **Step 6: Commit**

```powershell
git add -- src/App.tsx src/App.test.tsx
git commit -m "fix: make repair monitoring restartable"
```

---

### Task 8: Verify Frontend Coordination as a Whole

**Files:**
- No production file changes expected.

**Interfaces:**
- Consumes: Tasks 1-7 and `npm run verify`.
- Produces: a verified checkpoint ready for bookshelf/cache performance work.

- [ ] **Step 1: Run focused race and timer suites**

Run:

```powershell
npm test -- --run src/hooks/useOperationController.test.tsx src/hooks/useRepairMonitor.test.ts src/App.test.tsx src/features/dashboard/Dashboard.test.tsx src/features/automation/AutomationView.test.tsx
```

Expected: all focused tests pass with no unhandled Promise rejection or fake-timer warning.

- [ ] **Step 2: Run the full project gate**

Run:

```powershell
npm run verify
```

Expected: exit 0.

- [ ] **Step 3: Inspect scope and record the checkpoint**

Run:

```powershell
git diff --check
git status --short
git log --oneline -9
```

Expected: only the hook, App, Dashboard, AutomationView, and named test files changed in this plan; user-owned unrelated files remain untouched. Record the verified commit range in the subagent-driven-development ledger.
