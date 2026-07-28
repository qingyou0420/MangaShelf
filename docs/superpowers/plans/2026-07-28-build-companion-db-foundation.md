# Build and Companion Database Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore a fully green local verification path, version the companion database schema, and make each comic scan commit atomically without aborting the rest of the bookshelf.

**Architecture:** Keep the existing `CompanionDatabase` abstraction, add a focused child migration module, and introduce one transaction-oriented `save_comic_scan` write boundary. Filesystem work remains outside SQLite transactions; the bookshelf orchestrator computes a complete per-comic state and then performs exactly one database call.

**Tech Stack:** TypeScript 5.8, Vite 7, Vitest 4, Rust 2021, rusqlite 0.40.1, SQLite.

## Global Constraints

- This is a private Windows-only application for one fixed machine.
- Keep React + Tauri + Rust + SQLite; do not add an ORM, server, CI service, or state framework.
- Do not read from, write to, migrate, or delete the real MangaCon database, companion database, cover cache, or backup files while implementing or testing.
- Use temporary directories and fixture databases for every destructive or migration test.
- Preserve existing `README.md` and `docs/mangacon-companion-development.md` user changes unless a later explicit task names them.
- Use test-first development for every behavior change and observe the expected RED failure before implementation.
- Do not combine the full bookshelf scan into one transaction; the transaction boundary is one comic.
- Do not remove legacy automation, tables, or dependencies in this plan.

---

### Task 1: Restore Frontend Build and Strict Rust Lint

**Files:**
- Modify: `tsconfig.json:3-5`
- Modify: `src-tauri/src/mangacon/database.rs:243-260`
- Modify: `src-tauri/src/lib.rs:768-773`

**Interfaces:**
- Consumes: existing Node and Rust toolchains.
- Produces: no new runtime interface; establishes ES2022 as the explicit WebView2 JavaScript baseline.

- [ ] **Step 1: Verify the existing frontend build failure**

Run:

```powershell
npm run build
```

Expected: exit code 1 with seven `TS2550` errors stating that `Array.prototype.at` is unavailable under the ES2020 library.

- [ ] **Step 2: Verify the existing strict Clippy failure**

Run:

```powershell
cargo clippy --locked --manifest-path src-tauri\Cargo.toml --all-targets --all-features -- -D warnings
```

Expected: exit code 1 with `explicit_counter_loop` at `mangacon/database.rs` and `redundant_closure` at `lib.rs`.

- [ ] **Step 3: Raise the explicit TypeScript runtime baseline**

Change only the two baseline entries:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "DOM.Iterable"]
  }
}
```

Keep tests inside `tsconfig.json`'s existing `include: ["src"]`; do not hide the failing files from type checking.

- [ ] **Step 4: Apply the two behavior-preserving Clippy fixes**

Replace the repair task counter with an iterator carrying the database order:

```rust
let first_order_index = next_task_order_index(&transaction)?;
for (next_order_index, task) in
    (first_order_index..).zip(failed_tasks.iter().take(limit))
{
    // existing body, using next_order_index without manual increment
}
```

Pass the existing function item directly:

```rust
launch_mangacon_process,
```

- [ ] **Step 5: Verify GREEN**

Run:

```powershell
npm run build
npm test -- --run
cargo test --locked --manifest-path src-tauri\Cargo.toml
cargo clippy --locked --manifest-path src-tauri\Cargo.toml --all-targets --all-features -- -D warnings
```

Expected: all four commands exit 0; Vitest reports 53 passing tests and Rust reports 71 passing plus 13 ignored environment tests.

- [ ] **Step 6: Commit**

```powershell
git add -- tsconfig.json src-tauri/src/mangacon/database.rs src-tauri/src/lib.rs
git commit -m "build: restore local verification baseline"
```

---

### Task 2: Add One Read-Only Local Verification Command

**Files:**
- Create: `scripts/verify.ps1`
- Modify: `package.json:6-12`

**Interfaces:**
- Consumes: `npm`, `cargo`, and `git` available on PATH.
- Produces: `npm run verify`, which exits immediately with the first failing child command's exit code.

- [ ] **Step 1: Verify RED**

Run:

```powershell
npm run verify
```

Expected: exit code 1 with `Missing script: "verify"`.

- [ ] **Step 2: Create the verification script**

Create `scripts/verify.ps1` with this command order and no write-producing formatter or fixer:

```powershell
$ErrorActionPreference = "Stop"

function Invoke-VerificationStep {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [scriptblock]$Command
    )

    Write-Host "==> $Name"
    & $Command
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

Invoke-VerificationStep "Frontend tests" { npm test -- --run }
Invoke-VerificationStep "Frontend build" { npm run build }
Invoke-VerificationStep "Rust format" {
    cargo fmt --manifest-path src-tauri\Cargo.toml --all -- --check
}
Invoke-VerificationStep "Rust tests" {
    cargo test --locked --manifest-path src-tauri\Cargo.toml
}
Invoke-VerificationStep "Rust clippy" {
    cargo clippy --locked --manifest-path src-tauri\Cargo.toml --all-targets --all-features -- -D warnings
}
Invoke-VerificationStep "Whitespace" { git diff --check }
```

- [ ] **Step 3: Register the package script**

Add:

```json
"verify": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1"
```

- [ ] **Step 4: Verify GREEN**

Run:

```powershell
npm run verify
```

Expected: all six named steps run in order and the command exits 0.

- [ ] **Step 5: Commit**

```powershell
git add -- package.json scripts/verify.ps1
git commit -m "build: add local verification command"
```

---

### Task 3: Introduce Versioned, Backward-Compatible Migrations

**Files:**
- Create: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/db.rs:1-90`
- Test: `src-tauri/src/db.rs:462-648`

**Interfaces:**
- Consumes: an open SQLite connection with `foreign_keys` enabled.
- Produces:

```rust
// src-tauri/src/db/migrations.rs
pub(super) const CURRENT_SCHEMA_VERSION: i64 = 1;
pub(super) fn migrate(connection: &rusqlite::Connection) -> anyhow::Result<()>;
```

`CompanionDatabase::migrate(&self) -> anyhow::Result<()>` remains source-compatible and delegates to the child module.

- [ ] **Step 1: Write a failing version/idempotence test**

Add:

```rust
#[test]
fn migrate_records_current_schema_version_and_is_idempotent() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db = CompanionDatabase::open(temp.path().join("state.sqlite")).expect("open");

    db.migrate().expect("first migration");
    db.migrate().expect("second migration");

    assert_eq!(
        db.query_pragma_i64("user_version").expect("version"),
        migrations::CURRENT_SCHEMA_VERSION
    );
    assert!(db.has_table("comics").expect("comics"));
    assert!(db.has_table("chapters").expect("chapters"));
}
```

The test-only `query_pragma_i64` helper must query the real connection and remain under `#[cfg(test)]`.

- [ ] **Step 2: Run the targeted test to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml db::tests::migrate_records_current_schema_version_and_is_idempotent -- --exact
```

Expected: compile failure because `migrations` and `query_pragma_i64` do not exist, or assertion failure because `user_version` remains 0.

- [ ] **Step 3: Write a failing legacy UNIQUE-schema migration test**

Create a real unversioned fixture whose `comics.source_uri` is `UNIQUE`, plus one comic, tag, and chapter. After migration assert:

```rust
assert_eq!(comic_count(&connection), 1);
assert_eq!(tag_count(&connection), 1);
assert_eq!(chapter_count(&connection), 1);
assert_eq!(foreign_key_violation_count(&connection), 0);

connection.execute(
    "INSERT INTO comics (id, name, location, source_uri) VALUES (?1, ?2, ?3, ?4)",
    params!["second-id", "Second", "Second", "shared-source"],
).expect("same source_uri must no longer be globally unique");
```

Seed the original row with `source_uri = 'shared-source'`. Also assert the added `cover_path`, `latest_chapter_title`, `local_fingerprint`, and `has_update` columns exist and `user_version == 1`.

- [ ] **Step 4: Run the legacy test to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml db::tests::migrates_unversioned_unique_source_uri_schema_without_data_loss -- --exact
```

Expected: the second insert fails with the historical `UNIQUE` constraint.

- [ ] **Step 5: Write failing index and future-version tests**

Add:

```rust
#[test]
fn migration_creates_chapter_lookup_index() {
    // migrate a temporary database
    // assert PRAGMA index_info('idx_chapters_comic_ordinal_title')
    // returns comic_id, ordinal, title in that order
}

#[test]
fn migration_rejects_newer_schema_version() {
    // set PRAGMA user_version = CURRENT_SCHEMA_VERSION + 1
    // assert the error includes both actual and supported versions
}
```

Use literal expected column names; do not compute the expected list with migration helpers.

- [ ] **Step 6: Run both tests to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml migration_
```

Expected: the index test reports a missing index and the future-version test unexpectedly succeeds.

- [ ] **Step 7: Implement migration v1**

Inside `migrations.rs`:

```rust
pub(super) const CURRENT_SCHEMA_VERSION: i64 = 1;
const CHAPTER_LOOKUP_INDEX: &str = "idx_chapters_comic_ordinal_title";

pub(super) fn migrate(connection: &Connection) -> Result<()> {
    let version = user_version(connection)?;
    anyhow::ensure!(
        version <= CURRENT_SCHEMA_VERSION,
        "companion database schema {version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
    );

    let foreign_keys_were_enabled = foreign_keys_enabled(connection)?;
    connection.pragma_update(None, "foreign_keys", "OFF")?;
    let migration_result = migrate_to_v1(connection);
    connection.pragma_update(
        None,
        "foreign_keys",
        if foreign_keys_were_enabled { "ON" } else { "OFF" },
    )?;
    migration_result?;
    ensure_no_foreign_key_violations(connection)
}
```

`migrate_to_v1` must use one `unchecked_transaction`, create missing tables, add the four legacy columns, inspect `PRAGMA index_list/index_info`, and rebuild `comics` only when a unique index exactly covers `source_uri`. After the optional rebuild:

```sql
CREATE INDEX IF NOT EXISTS idx_chapters_comic_ordinal_title
ON chapters(comic_id, ordinal, title);
PRAGMA user_version = 1;
```

Before rebuilding, add all missing columns to the old table so the data copy always selects the complete final column list. Create `comics_v1`, copy every final column including `updated_at`, drop the old table, then rename `comics_v1` to `comics`. Do not alter `favorites` or `automation_runs`.

- [ ] **Step 8: Verify GREEN**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml db::tests
```

Expected: all database tests pass, including repeated migration, legacy UNIQUE removal, index columns, future-version rejection, and foreign-key preservation.

- [ ] **Step 9: Commit**

```powershell
git add -- src-tauri/src/db.rs src-tauri/src/db/migrations.rs
git commit -m "feat: version companion database migrations"
```

---

### Task 4: Add an Atomic Per-Comic Scan Write Boundary

**Files:**
- Modify: `src-tauri/src/db.rs:204-347`
- Test: `src-tauri/src/db.rs:462-648`

**Interfaces:**
- Consumes: a fully computed `Comic`, an explicit chapter write mode, and the fingerprint to store.
- Produces:

```rust
pub(crate) enum ChapterIndexWrite<'a> {
    Preserve,
    Replace(&'a [Chapter]),
}

pub(crate) fn save_comic_scan(
    &mut self,
    comic: &Comic,
    chapter_write: ChapterIndexWrite<'_>,
    local_fingerprint: Option<&str>,
) -> anyhow::Result<()>;
```

- [ ] **Step 1: Write the rollback test**

Seed a comic with tag `old-tag`, one old chapter, and `old-fingerprint`. Attempt a replacement with an updated comic, one valid chapter, and a second chapter whose `comic_id` is `missing-comic`.

Assert:

```rust
let error = db
    .save_comic_scan(
        &updated,
        ChapterIndexWrite::Replace(&[valid, invalid]),
        Some("new-fingerprint"),
    )
    .expect_err("foreign key failure must abort the comic transaction");
assert!(error.to_string().contains("FOREIGN KEY"));

let stored = db.list_comics().expect("comics").remove(0);
assert_eq!(stored.name, "Old name");
assert_eq!(stored.tags, vec!["old-tag"]);
assert_eq!(db.list_chapters_for_comic(&stored.id).expect("chapters"), vec![old_chapter]);
assert_eq!(
    db.local_fingerprint_for_comic(&stored.id).expect("fingerprint").as_deref(),
    Some("old-fingerprint")
);
```

- [ ] **Step 2: Run the rollback test to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml db::tests::save_comic_scan_rolls_back_all_fields_when_chapter_write_fails -- --exact
```

Expected: compile failure because `ChapterIndexWrite` and `save_comic_scan` do not exist.

- [ ] **Step 3: Write the Preserve/Replace semantics test**

First call `save_comic_scan(..., ChapterIndexWrite::Preserve, ...)` and assert the old chapter remains while comic fields and fingerprint change. Then call it with `ChapterIndexWrite::Replace(&[])` and assert chapters are empty and the other fields commit.

- [ ] **Step 4: Run the semantics test to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml db::tests::save_comic_scan_preserves_or_clears_chapters_as_requested -- --exact
```

Expected: compile failure on the new interface.

- [ ] **Step 5: Implement the transaction**

Extract connection-oriented helpers:

```rust
fn upsert_chapter_on(connection: &Connection, chapter: &Chapter) -> Result<()>;

fn update_local_fingerprint_on(
    connection: &Connection,
    comic_id: &str,
    fingerprint: Option<&str>,
) -> Result<()>;
```

Implement:

```rust
pub(crate) fn save_comic_scan(
    &mut self,
    comic: &Comic,
    chapter_write: ChapterIndexWrite<'_>,
    local_fingerprint: Option<&str>,
) -> Result<()> {
    let transaction = self.connection.transaction()?;
    upsert_comic_on(&transaction, comic)?;

    if let ChapterIndexWrite::Replace(chapters) = chapter_write {
        transaction.execute(
            "DELETE FROM chapters WHERE comic_id = ?1",
            params![comic.id],
        )?;
        for chapter in chapters {
            upsert_chapter_on(&transaction, chapter)?;
        }
    }

    update_local_fingerprint_on(&transaction, &comic.id, local_fingerprint)?;
    transaction.commit()?;
    Ok(())
}
```

Production bookshelf code must stop calling the three independent write methods after Task 5.

- [ ] **Step 6: Verify GREEN**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml save_comic_scan
cargo test --locked --manifest-path src-tauri\Cargo.toml db::tests
```

Expected: both new behavior tests and all existing DB tests pass.

- [ ] **Step 7: Commit**

```powershell
git add -- src-tauri/src/db.rs
git commit -m "feat: save each comic scan atomically"
```

---

### Task 5: Route Bookshelf Writes Through the Atomic Boundary

**Files:**
- Modify: `src-tauri/src/lib.rs:643-727`
- Test: `src-tauri/src/lib.rs:1285-1408`
- Test: `src-tauri/src/lib.rs:1637-1730`

**Interfaces:**
- Consumes: `CompanionDatabase::save_comic_scan` and `ChapterIndexWrite`.
- Produces: unchanged `SyncBookshelfMatchesResult` for successful scans.

- [ ] **Step 1: Write the orchestration rollback test**

Create a temporary companion DB with one matched comic, old chapter, and old fingerprint. Add:

```sql
CREATE TRIGGER fail_comic_update
BEFORE UPDATE ON comics
WHEN NEW.id = 'cp:rollback'
BEGIN
    SELECT RAISE(ABORT, 'forced comic update failure');
END;
```

Make the comic missing from the temporary bookshelf and call `sync_bookshelf_matches_inner`. After the expected error, reopen the DB and assert its old comic status, old chapter, and old fingerprint remain unchanged.

- [ ] **Step 2: Run the rollback test to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml tests::bookshelf_sync_rolls_back_one_comic_when_metadata_write_fails -- --exact
```

Expected: the test fails because the current missing branch deletes chapters before the comic update trigger aborts.

- [ ] **Step 3: Replace the three-write sequences**

Make the database mutable:

```rust
let mut db = CompanionDatabase::open(&database_path)?;
```

For an unchanged fingerprint:

```rust
db.save_comic_scan(
    comic,
    ChapterIndexWrite::Preserve,
    Some(&fingerprint),
)?;
```

For a successful rescan:

```rust
db.save_comic_scan(
    comic,
    ChapterIndexWrite::Replace(&chapters),
    Some(&fingerprint),
)?;
```

For a missing directory:

```rust
db.save_comic_scan(comic, ChapterIndexWrite::Replace(&[]), None)?;
```

All fingerprinting, chapter scanning, cover lookup, and summary calculation must finish before `save_comic_scan` opens its transaction.

- [ ] **Step 4: Verify GREEN**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml bookshelf_sync
```

Expected: rollback, cached-index reuse, cover fallback, matched/missing, and chapter persistence tests all pass.

- [ ] **Step 5: Commit**

```powershell
git add -- src-tauri/src/lib.rs
git commit -m "refactor: commit bookshelf results per comic"
```

---

### Task 6: Isolate a Failed Comic and Continue the Bookshelf

**Files:**
- Modify: `src-tauri/src/lib.rs:288-299`
- Modify: `src-tauri/src/lib.rs:643-727`
- Modify: `src/lib/types.ts:53-61`
- Modify: `src/App.tsx:167-191`
- Test: `src-tauri/src/lib.rs:1637-1730`
- Test: `src/App.test.tsx:206-260`

**Interfaces:**
- Produces:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookshelfSyncFailure {
    pub comic_id: String,
    pub name: String,
    pub message: String,
}

pub struct SyncBookshelfMatchesResult {
    // existing fields
    pub failed: usize,
    pub failures: Vec<BookshelfSyncFailure>,
}
```

The TypeScript result mirrors `failed` and `failures`.

- [ ] **Step 1: Write a failing two-comic continuation test**

Seed two imported comics ordered so `cp:bad` is processed first. Add a trigger that aborts only updates to `cp:bad`; keep both local directories absent.

Assert:

```rust
let result = sync_bookshelf_matches_inner(...).expect("batch continues");
assert_eq!(result.failed, 1);
assert_eq!(result.failures[0].comic_id, "cp:bad");

let good = result.favorites.iter().find(|comic| comic.id == "cp:good").unwrap();
assert_eq!(good.scan_status, ScanStatus::Missing);

let bad = result.favorites.iter().find(|comic| comic.id == "cp:bad").unwrap();
assert_eq!(bad.scan_status, ScanStatus::Error);
```

Reopen the DB and assert the failed comic's previously committed chapters and metadata are unchanged.

- [ ] **Step 2: Run the continuation test to verify RED**

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml tests::bookshelf_sync_continues_after_one_comic_database_failure -- --exact
```

Expected: the current function returns `Err` at the first failed comic and never processes `cp:good`.

- [ ] **Step 3: Isolate the per-comic work**

Move one loop iteration into:

```rust
fn sync_one_bookshelf_comic(
    db: &mut CompanionDatabase,
    comic: &mut Comic,
    bookshelf_root: &Path,
) -> anyhow::Result<ComicSyncOutcome>;
```

`ComicSyncOutcome` carries whether the comic was scanned, matched, or missing. The outer loop catches an error, records `BookshelfSyncFailure`, marks that comic as `ScanStatus::Error` in the returned batch, and continues. Do not overwrite the failed comic's last committed DB state with a partial error record.

- [ ] **Step 4: Add the frontend failure summary RED test**

Mock:

```ts
syncBookshelfMatchesMock.mockResolvedValue({
  imported: 2,
  scanned: 1,
  matched: 1,
  missing: 0,
  orphaned: 0,
  failed: 1,
  failures: [{ comicId: "cp:bad", name: "Bad", message: "forced failure" }],
  favorites: syncedFavorites,
});
```

Click the bookshelf scan button and assert the visible summary includes `失败 1 本`. The production change that this test catches is silently omitting partial failures.

- [ ] **Step 5: Run the frontend test to verify RED**

Run:

```powershell
npm test -- --run src/App.test.tsx
```

Expected: the message does not include the failed count.

- [ ] **Step 6: Wire the result and verify GREEN**

Update `SyncBookshelfMatchesResult` in TypeScript and append the failed count to the existing scan summary. Do not add a new page or logging system.

Run:

```powershell
cargo test --locked --manifest-path src-tauri\Cargo.toml bookshelf_sync
npm test -- --run src/App.test.tsx
```

Expected: both suites pass and a failed comic no longer aborts the good comic.

- [ ] **Step 7: Commit**

```powershell
git add -- src-tauri/src/lib.rs src/lib/types.ts src/App.tsx src/App.test.tsx
git commit -m "feat: isolate bookshelf scan failures"
```

---

### Task 7: Verify the Foundation as a Whole

**Files:**
- No production file changes expected.

**Interfaces:**
- Consumes: Tasks 1-6.
- Produces: a verified checkpoint ready for the MangaCon mutation-safety plan.

- [ ] **Step 1: Run the unified gate**

Run:

```powershell
npm run verify
```

Expected: exit 0 with no TypeScript, test, format, Clippy, or whitespace failure.

- [ ] **Step 2: Inspect repository scope**

Run:

```powershell
git status --short
git log --oneline -7
```

Expected: only files named by this plan changed; pre-existing `README.md` and `docs/mangacon-companion-development.md` state remains untouched.

- [ ] **Step 3: Record the checkpoint**

No empty commit is required. Record the verified commit range in the subagent-driven-development ledger.
