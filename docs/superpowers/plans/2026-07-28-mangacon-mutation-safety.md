# MangaCon Mutation Safety Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure MangaCon task mutations use transaction-free filesystem discovery, verified SQLite snapshots, short revalidating write transactions, truthful staged results, and one backend operation gate.

**Architecture:** Separate the existing `mangacon::database` workflow into discovery and commit phases. A dedicated backup module creates and verifies online SQLite snapshots; a dedicated operation module serializes process-wide MangaCon mutations without holding a mutex guard throughout the operation. Database commit remains authoritative even if the subsequent restart or confirmation produces warnings.

**Tech Stack:** Rust 2021, Tauri 2 managed state, rusqlite 0.40.1 with `bundled` and `backup`, SQLite Online Backup API, Serde, Vitest for frontend result rendering.

## Global Constraints

- This is a private Windows-only application for one fixed machine.
- Keep MangaCon's journal mode and external schema unchanged.
- Never point a test or development command at the real `MangaCon.dat`, real companion DB, or real backup directory.
- Use `tempfile` fixtures for every backup, queue, repair, resume, locking, and retention test.
- Preserve the existing queue selection, deduplication, badge clearing, task ordering, repair limit, and resume configuration semantics.
- Filesystem scanning must happen after the MangaCon read connection is released and before the short write transaction begins.
- No normal no-op queue, repair, or resume call may create a backup.
- Publish a backup only after `PRAGMA quick_check` returns exactly `ok`.
- Retain exactly the newest 20 valid backups whose file names exactly match the current database's companion-backup pattern.
- A database error before commit returns `Err`; restart, confirm, or old-backup cleanup failure after a successful commit returns `Ok` with typed warnings.
- Do not change frontend operation coordination or polling in this plan beyond displaying the new staged result.

---

### Task 1: Extract Transaction-Free Update Discovery

**Files:**
- Modify: `src-tauri/src/mangacon/database.rs:86-192`
- Modify: `src-tauri/src/mangacon/database.rs:389-628`
- Test: `src-tauri/src/mangacon/database.rs:823-1205`

**Interfaces:**
- Produces:

```rust
struct MangaConCandidateSnapshot {
    badged: Vec<MangaConUpdateCandidate>,
    inactive_by_uri: HashMap<String, Vec<MangaConUpdateCandidate>>,
    existing_tasks: HashSet<(String, String)>,
}

struct MatchedLocalComic {
    comic_id: String,
    local_path: PathBuf,
}

struct MangaConUpdatePlan {
    total_updates: usize,
    candidates: Vec<MangaConUpdateCandidate>,
}

trait LocalChapterIndexProvider {
    fn load(
        &self,
        comic_id: &str,
        local_path: &Path,
        companion_database_path: &Path,
    ) -> Result<LocalChapterTitleIndex>;
}
```

Production entry:

```rust
fn discover_updates(
    database_path: &Path,
    companion_database_path: Option<&Path>,
    max_updates: Option<u32>,
) -> Result<MangaConUpdatePlan>;
```

- [ ] **Step 1: Write a discovery purity test**

Add:

```rust
#[test]
fn discovery_does_not_mutate_mangacon_database_or_create_backup() {
    let (_temp, path) = create_fixture_db();

    let plan = discover_updates(&path, None, None).expect("discover");

    assert_eq!(plan.candidates.len(), 1);
    assert_eq!(task_count(&path), 0);
    assert!(backup_paths(&path).is_empty());
}
```

The production change that makes this test pass is a read-only discovery entry point; asserting only source text or mocked calls is not acceptable.

- [ ] **Step 2: Run the purity test to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::database::tests::discovery_does_not_mutate_mangacon_database_or_create_backup -- --exact
```

Expected: compile failure because `discover_updates` and `MangaConUpdatePlan` do not exist.

- [ ] **Step 3: Write the connection-release test**

Implement a test-only `LockProbeChapterIndexProvider`. During `load`, it opens a second connection to the fixture MangaCon database with zero busy timeout and executes `BEGIN IMMEDIATE; ROLLBACK;`.

Assert:

```rust
let plan = discover_updates_with_provider(
    &manga_db_path,
    Some(&companion_db_path),
    None,
    &probe,
).expect("discover");

assert!(probe.immediate_write_lock_was_available());
assert_eq!(plan.candidates[0].volume_key, "missing-chapter");
```

- [ ] **Step 4: Run the connection-release test to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::database::tests::local_filesystem_scan_runs_after_mangacon_read_scope_is_released -- --exact
```

Expected: compile failure on the provider interface. An implementation that keeps the old transaction alive will instead fail with `database is locked`.

- [ ] **Step 5: Implement the three discovery phases**

Implement:

```text
open MangaCon read-only
  -> copy candidate rows, existing task keys, and matched local comic rows into owned values
drop MangaCon connection
  -> use LocalChapterIndexProvider for filesystem/companion lookup
  -> merge and limit candidates into MangaConUpdatePlan
```

Use:

```rust
Connection::open_with_flags(
    database_path,
    OpenFlags::SQLITE_OPEN_READ_ONLY,
)?;
```

Do not pass a `Connection` or `Transaction` into `LocalChapterIndexProvider::load`.

- [ ] **Step 6: Verify GREEN and preserve selection behavior**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::database::tests
```

Expected: discovery purity and lock-probe tests pass together with existing badge, missing-local-chapter, stale-cache, deduplication, limit, and remaining-badge tests.

- [ ] **Step 7: Commit**

```powershell
git add -- src-tauri/src/mangacon/database.rs
git commit -m "refactor: separate MangaCon update discovery"
```

---

### Task 2: Create Verified Online Backups and Retain Twenty

**Files:**
- Modify: `src-tauri/Cargo.toml:25`
- Modify: `src-tauri/Cargo.lock`
- Create: `src-tauri/src/mangacon/backup.rs`
- Modify: `src-tauri/src/mangacon/mod.rs:1-7`
- Test: `src-tauri/src/mangacon/backup.rs`

**Interfaces:**
- Produces:

```rust
pub(super) const BACKUP_RETENTION_COUNT: usize = 20;

pub(super) fn create_verified_online_backup(
    database_path: &Path,
) -> anyhow::Result<PathBuf>;

fn create_verified_online_backup_with<V>(
    database_path: &Path,
    verify: V,
) -> anyhow::Result<PathBuf>
where
    V: FnOnce(&Connection) -> anyhow::Result<()>;

pub(super) fn prune_verified_backups(
    database_path: &Path,
    keep: usize,
) -> anyhow::Result<()>;
```

- [ ] **Step 1: Write a WAL snapshot test**

Create a temporary database in WAL mode, create table `items`, insert and commit one row while keeping a connection open, and avoid a manual checkpoint.

Assert:

```rust
let backup_path = create_verified_online_backup(&source_path).expect("backup");
let copied = Connection::open_with_flags(
    backup_path,
    OpenFlags::SQLITE_OPEN_READ_ONLY,
).expect("open backup");
let count: i64 = copied
    .query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
    .expect("count");
assert_eq!(count, 1);
```

- [ ] **Step 2: Run the WAL test to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::backup::tests::online_backup_includes_committed_wal_pages -- --exact
```

Expected: compile failure because the backup module does not exist.

- [ ] **Step 3: Write failed-verification and retention tests**

Verification failure:

```rust
let result = create_verified_online_backup_with(&source_path, |_| {
    anyhow::bail!("injected quick_check failure")
});
assert!(result.is_err());
assert!(formal_backup_paths(&source_path).is_empty());
assert!(temporary_backup_paths(&source_path).is_empty());
```

Retention:

```rust
create_exact_backups(&source_path, 22);
create_similar_but_unrelated_files(&source_path);

prune_verified_backups(&source_path, 20).expect("prune");

assert_eq!(formal_backup_paths(&source_path).len(), 20);
assert!(similar_but_unrelated_files(&source_path)
    .iter()
    .all(|path| path.exists()));
```

Use distinct numeric timestamps and assert the two oldest exact matches are removed.

- [ ] **Step 4: Run both tests to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::backup::tests
```

Expected: compile failure on the new functions.

- [ ] **Step 5: Enable and use rusqlite's backup feature**

Change:

```toml
rusqlite = { version = "0.40.1", features = ["bundled", "backup"] }
```

Use two connections and a bounded step loop:

```rust
let source = Connection::open_with_flags(
    database_path,
    OpenFlags::SQLITE_OPEN_READ_ONLY,
)?;
let mut destination = Connection::open(&temporary_path)?;
let backup = Backup::new(&source, &mut destination)?;

let deadline = Instant::now() + Duration::from_secs(30);
loop {
    match backup.step(100)? {
        StepResult::Done => break,
        StepResult::More => {}
        StepResult::Busy | StepResult::Locked if Instant::now() < deadline => {
            thread::sleep(Duration::from_millis(25));
        }
        StepResult::Busy | StepResult::Locked => {
            anyhow::bail!("timed out creating SQLite backup");
        }
        _ => anyhow::bail!("unexpected SQLite backup state"),
    }
}
drop(backup);
```

Query `PRAGMA quick_check` on `destination`, require the literal string `ok`, close the connection, and rename the same-directory temporary file to:

```text
<database-file-name>.companion-backup-<unix-milliseconds>
```

On every error, remove only the exact temporary file created by this call. Retention matches the exact database file name plus `.companion-backup-` and an all-digit suffix.

- [ ] **Step 6: Verify GREEN**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::backup::tests
```

Expected: WAL, failed-publication, exact-name retention, and newest-twenty tests pass.

- [ ] **Step 7: Commit**

```powershell
git add -- src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/mangacon/mod.rs src-tauri/src/mangacon/backup.rs
git commit -m "feat: create verified MangaCon backups"
```

---

### Task 3: Commit Queue Plans in a Short Immediate Transaction

**Files:**
- Modify: `src-tauri/src/mangacon/database.rs:27-36`
- Modify: `src-tauri/src/mangacon/database.rs:107-192`
- Modify: `src-tauri/src/mangacon/database.rs:710-820`
- Test: `src-tauri/src/mangacon/database.rs:870-1205`

**Interfaces:**
- Changes:

```rust
pub struct QueueMangaConUpdatesResult {
    pub backup_path: Option<PathBuf>,
    pub database_committed: bool,
    pub backup_cleanup_warning: Option<String>,
    // existing counts and tasks
}

fn apply_queue_plan(
    connection: &mut Connection,
    plan: &MangaConUpdatePlan,
) -> Result<QueueMutationSummary>;
```

Test-only hook:

```rust
fn apply_queue_plan_with_hook<F>(
    connection: &mut Connection,
    plan: &MangaConUpdatePlan,
    after_begin: F,
) -> Result<QueueMutationSummary>
where
    F: FnOnce();
```

- [ ] **Step 1: Write the no-op test**

Remove all update candidates from the fixture and call the public queue entry.

Assert:

```rust
assert!(!result.database_committed);
assert_eq!(result.backup_path, None);
assert!(backup_paths(&path).is_empty());
```

- [ ] **Step 2: Run the no-op test to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::database::tests::no_op_queue_does_not_create_backup -- --exact
```

Expected: current code creates a backup and the result cannot express `None`.

- [ ] **Step 3: Write revalidation and IMMEDIATE-lock tests**

Revalidation:

```rust
let plan = discover_updates(&path, None, None).expect("discover");
insert_matching_task_from_second_connection(&path);
let result = commit_discovered_updates(&path, plan).expect("commit");
assert_eq!(matching_task_count(&path), 1);
assert_eq!(result.skipped_existing, 1);
```

Immediate lock:

```rust
let mut observed_busy = false;
apply_queue_plan_with_hook(&mut connection, &plan, || {
    observed_busy = second_writer_begin_immediate(&path).is_busy();
}).expect("apply");
assert!(observed_busy);
```

- [ ] **Step 4: Run both tests to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::database::tests::queue_revalidates_existing_task_after_discovery -- --exact
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::database::tests::mutation_acquires_immediate_writer_lock_before_revalidation -- --exact
```

Expected: compile failure because staged commit functions do not exist.

- [ ] **Step 5: Implement queue discovery/backup/commit**

The public flow must be:

```rust
let plan = discover_updates(database_path, companion_database_path, max_updates)?;
if plan.has_no_mutation() {
    return Ok(QueueMangaConUpdatesResult::no_op(plan.total_updates));
}

let backup_path = create_verified_online_backup(database_path)?;
let mut connection = Connection::open_with_flags(
    database_path,
    OpenFlags::SQLITE_OPEN_READ_WRITE,
)?;
let summary = apply_queue_plan(&mut connection, &plan)?;
let cleanup_warning =
    prune_verified_backups(database_path, BACKUP_RETENTION_COUNT)
        .err()
        .map(|error| error.to_string());
```

`apply_queue_plan_with_hook` begins:

```rust
let transaction = connection
    .transaction_with_behavior(TransactionBehavior::Immediate)?;
after_begin();
```

Inside that transaction, re-read existing unfinished tasks, revalidate each candidate, assign order indexes, insert tasks, clear markers, update badges/config, and commit. Never trust only the discovery snapshot.

- [ ] **Step 6: Verify all queue behavior**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::database::tests
```

Expected: no-op, revalidation, IMMEDIATE locking, existing deduplication, limit, marker, badge, and local-gap tests pass.

- [ ] **Step 7: Commit**

```powershell
git add -- src-tauri/src/mangacon/database.rs
git commit -m "feat: commit MangaCon queue plans safely"
```

---

### Task 4: Apply the Same No-Op and Backup Rules to Repair and Resume

**Files:**
- Modify: `src-tauri/src/mangacon/database.rs:61-76`
- Modify: `src-tauri/src/mangacon/database.rs:235-292`
- Modify: `src-tauri/src/mangacon/database.rs:755-820`
- Test: `src-tauri/src/mangacon/database.rs:1283-1425`

**Interfaces:**
- Changes both results to:

```rust
pub backup_path: Option<PathBuf>,
pub database_committed: bool,
pub backup_cleanup_warning: Option<String>,
```

- [ ] **Step 1: Write no-op repair and resume tests**

Add:

```rust
#[test]
fn no_failed_tasks_repair_does_not_create_backup() {
    let (_temp, path) = create_fixture_db();
    let result = requeue_failed_tasks_for_repair(&path, None).expect("repair");
    assert!(!result.database_committed);
    assert_eq!(result.backup_path, None);
    assert!(backup_paths(&path).is_empty());
}

#[test]
fn no_unfinished_tasks_resume_does_not_create_backup() {
    let (_temp, path) = create_fixture_db();
    clear_unfinished_tasks(&path);
    let result = prepare_unfinished_tasks_for_resume(&path).expect("resume");
    assert!(!result.database_committed);
    assert_eq!(result.backup_path, None);
    assert!(backup_paths(&path).is_empty());
}
```

- [ ] **Step 2: Run both tests to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::database::tests::no_failed_tasks_repair_does_not_create_backup -- --exact
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::database::tests::no_unfinished_tasks_resume_does_not_create_backup -- --exact
```

Expected: current code creates backups before checking candidates/tasks.

- [ ] **Step 3: Implement read-plan/backup/short-commit flows**

For repair, discover failed task IDs read-only, return no-op when empty, create the verified backup, then revalidate and requeue under one `IMMEDIATE` transaction.

For resume, count unfinished tasks read-only, return no-op at zero, create the verified backup, then re-count and set `continue_last_session_tasks` under one `IMMEDIATE` transaction.

Both functions prune after a successful commit. A prune error populates `backup_cleanup_warning` without changing `database_committed`.

- [ ] **Step 4: Verify GREEN**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::database::tests
```

Expected: no-op behavior and all existing repair/resume ordering, limit, error-count, and config tests pass.

- [ ] **Step 5: Commit**

```powershell
git add -- src-tauri/src/mangacon/database.rs
git commit -m "feat: avoid no-op MangaCon backups"
```

---

### Task 5: Return Truthful Post-Commit Warnings

**Files:**
- Modify: `src-tauri/src/lib.rs:243-278`
- Modify: `src-tauri/src/lib.rs:386-465`
- Modify: `src-tauri/src/lib.rs:537-589`
- Modify: `src/lib/types.ts:210-239`
- Modify: `src/App.tsx:199-354`
- Test: `src-tauri/src/lib.rs:1131-1504`
- Test: `src/App.test.tsx:309-437`

**Interfaces:**
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MangaConOperationWarningStage {
    BackupCleanup,
    Restart,
    Confirm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaConOperationWarning {
    pub stage: MangaConOperationWarningStage,
    pub message: String,
}
```

All three command results contain:

```rust
pub database_committed: bool,
pub backup_path: Option<String>,
pub warnings: Vec<MangaConOperationWarning>,
```

The TypeScript interfaces mirror the same camelCase JSON fields and warning stage literals.

- [ ] **Step 1: Write restart and confirmation warning tests**

Add hook-level tests:

```rust
#[test]
fn committed_queue_returns_warning_when_restart_fails() {
    let result = finish_queue_with_hooks(
        committed_queue_result(),
        || Err("restart failed".to_string()),
        || panic!("confirm must not run"),
    ).expect("post-commit warning stays successful");

    assert!(result.database_committed);
    assert!(result.backup_path.is_some());
    assert_eq!(result.warnings[0].stage, MangaConOperationWarningStage::Restart);
}

#[test]
fn committed_queue_returns_confirm_warning_and_launch_pid() {
    let result = finish_queue_with_hooks(
        committed_queue_result(),
        || Ok(LaunchResult { pid: 3052 }),
        || Err("confirm failed".to_string()),
    ).expect("post-commit warning stays successful");

    assert!(result.launched);
    assert_eq!(result.launch_pid, Some(3052));
    assert_eq!(result.warnings[0].stage, MangaConOperationWarningStage::Confirm);
}
```

Also add a pre-commit failure test asserting that a backup verification error remains `Err` and neither restart nor confirm hook is called.

- [ ] **Step 2: Run the Rust tests to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml tests::committed_queue_returns_warning_when_restart_fails -- --exact
cargo test --locked --manifest-path src-tauri\Cargo.toml tests::committed_queue_returns_confirm_warning_and_launch_pid -- --exact
cargo test --locked --manifest-path src-tauri\Cargo.toml tests::pre_commit_database_error_is_still_an_error -- --exact
```

Expected: compile failure on the new staged-result helpers and types.

- [ ] **Step 3: Implement one post-commit helper**

Implement:

```rust
fn run_post_commit_refresh_with_hooks<L, C>(
    should_refresh: bool,
    launch: L,
    confirm: C,
) -> MangaConPostCommitOutcome
where
    L: FnOnce() -> Result<LaunchResult, String>,
    C: FnOnce() -> Result<ContinueDownloadConfirmResult, String>;
```

When launch fails, append a `Restart` warning and do not call confirm. When launch succeeds but confirm fails, preserve the PID and append `Confirm`. Convert `backup_cleanup_warning` into `BackupCleanup`. Queue, repair, and resume all use this helper.

- [ ] **Step 4: Write the visible-warning frontend test**

Mock a committed queue result containing:

```ts
warnings: [{ stage: "restart", message: "restart failed" }],
databaseCommitted: true,
backupPath: "C:\\temp\\MangaCon.dat.companion-backup-1",
```

Click the update button and assert the visible message contains both the committed queued count and `后续重启需要人工检查`. The test catches replacing a committed success with a generic failure message.

- [ ] **Step 5: Run the frontend test to verify RED**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: the existing message reports only queued counts and ignores warnings.

- [ ] **Step 6: Wire TypeScript and visible warnings**

Update every existing result fixture with `databaseCommitted`, nullable `backupPath`, and `warnings`. Append a short warning suffix to queue, repair, and resume messages. Do not add a new modal or notification framework.

- [ ] **Step 7: Verify GREEN**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml
npm test -- --run src/App.test.tsx src/lib/api.test.ts
npm run build
```

Expected: all commands exit 0 and the new warning rendering test passes.

- [ ] **Step 8: Commit**

```powershell
git add -- src-tauri/src/lib.rs src/lib/types.ts src/App.tsx src/App.test.tsx src/lib/api.test.ts
git commit -m "feat: report MangaCon post-commit warnings"
```

---

### Task 6: Serialize Backend MangaCon Operations

**Files:**
- Create: `src-tauri/src/mangacon/operation.rs`
- Modify: `src-tauri/src/mangacon/mod.rs:1-7`
- Modify: `src-tauri/src/lib.rs:356-589`
- Modify: `src-tauri/src/lib.rs:1095-1128`
- Test: `src-tauri/src/mangacon/operation.rs`

**Interfaces:**
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MangaConOperationKind {
    QueueUpdates,
    RepairTasks,
    ResumeTasks,
    Launch,
    Restart,
    WindowAutomation,
}

#[derive(Default)]
pub struct MangaConOperationGate {
    active: Mutex<Option<MangaConOperationKind>>,
}

impl MangaConOperationGate {
    pub fn try_acquire(
        &self,
        operation: MangaConOperationKind,
    ) -> Result<MangaConOperationPermit<'_>, MangaConOperationBusy>;
}
```

`MangaConOperationBusy` exposes `code() == "operation_busy"` and the active operation kind. Its `Display` string starts with `operation_busy:` so existing Tauri commands can keep `Result<T, String>` and map the gate error without changing every command error type.

- [ ] **Step 1: Write permit lifecycle tests**

Add:

```rust
#[test]
fn second_mutating_operation_is_rejected_with_active_operation() {
    let gate = MangaConOperationGate::default();
    let first = gate.try_acquire(MangaConOperationKind::QueueUpdates)
        .expect("first permit");

    let error = gate.try_acquire(MangaConOperationKind::Restart)
        .expect_err("overlap rejected");

    assert_eq!(error.code(), "operation_busy");
    assert_eq!(error.active(), MangaConOperationKind::QueueUpdates);
    drop(first);
}

#[test]
fn dropping_permit_releases_gate_after_early_error() {
    let gate = MangaConOperationGate::default();
    {
        let _permit = gate.try_acquire(MangaConOperationKind::RepairTasks)
            .expect("permit");
    }
    assert!(gate.try_acquire(MangaConOperationKind::QueueUpdates).is_ok());
}
```

- [ ] **Step 2: Run lifecycle tests to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::operation::tests
```

Expected: compile failure because the module and gate do not exist.

- [ ] **Step 3: Implement a permit that does not hold the MutexGuard**

`try_acquire` locks only long enough to set `active`. `MangaConOperationPermit::drop` locks again and clears the matching operation. Poisoned mutexes return a stable internal error rather than panicking.

Add:

```rust
pub fn run_guarded_operation<T, F>(
    gate: &MangaConOperationGate,
    kind: MangaConOperationKind,
    operation: F,
) -> Result<T, MangaConOperationError>
where
    F: FnOnce() -> Result<T, String>;
```

- [ ] **Step 4: Write the guarded-body test**

Acquire and hold a queue permit, then call `run_guarded_operation` for restart. Assert the second closure was never entered and the returned error code is `operation_busy`.

- [ ] **Step 5: Run the guarded-body test to verify RED, then implement**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml mangacon::operation::tests::concurrent_operation_never_enters_second_body -- --exact
```

Expected before implementation: compile failure on `run_guarded_operation`. After the minimal implementation: pass.

- [ ] **Step 6: Register and apply the Tauri state**

Register:

```rust
.manage(MangaConOperationGate::default())
```

Acquire permits for:

- ensure/launch/restart MangaCon;
- queue, repair, and resume;
- badge scanning and every command that sends window, mouse, scroll, or download actions.

Do not acquire permits for read-only window enumeration or task-status queries. Hold the permit from command entry through restart/confirm completion.

Each existing command keeps its current serialized success type and maps a gate failure with:

```rust
gate.try_acquire(kind).map_err(|error| error.to_string())?;
```

- [ ] **Step 7: Verify GREEN**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml
cargo clippy --locked --manifest-path src-tauri\Cargo.toml --all-targets --all-features -- -D warnings
```

Expected: all gate, database, command, and existing environment-independent tests pass with no warnings.

- [ ] **Step 8: Commit**

```powershell
git add -- src-tauri/src/mangacon/operation.rs src-tauri/src/mangacon/mod.rs src-tauri/src/lib.rs
git commit -m "feat: serialize MangaCon mutations"
```

---

### Task 7: Verify MangaCon Mutation Safety as a Whole

**Files:**
- No production file changes expected.

**Interfaces:**
- Consumes: Tasks 1-6 and the foundation plan's `npm run verify`.
- Produces: a verified checkpoint ready for frontend operation coordination.

- [ ] **Step 1: Run the unified verification**

Run:

```powershell
npm run verify
```

Expected: exit 0.

- [ ] **Step 2: Confirm tests used only temporary databases**

Run:

```powershell
git diff --check
git status --short
```

Inspect the task reports and confirm every new backup/queue/repair/resume test path originates from `tempfile::tempdir()`. No command should have referenced `E:\书架\mangacon-companion.sqlite` or the real AppData MangaCon database.

- [ ] **Step 3: Record the checkpoint**

No empty commit is required. Record the verified commit range and any deferred minor findings in the subagent-driven-development ledger.
