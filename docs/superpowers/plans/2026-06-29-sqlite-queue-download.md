# SQLite Queue Download Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace fragile screenshot/mouse update automation with MangaCon SQLite task queue injection and automatic restart-confirmation handling.

**Architecture:** Add a focused `mangacon::database` adapter that reads update badges from `MangaCon.dat` and inserts pending rows into `mc3_tasks`. Add a focused `mangacon::confirm` helper that accepts MangaCon's "continue unfinished downloads" modal through Win32 messages. Expose a Tauri command used by the dashboard and automation view.

**Tech Stack:** Rust, Tauri 2, rusqlite, Win32 WindowsAndMessaging, React, Vitest.

---

### Task 1: MangaCon SQLite Queue Adapter

**Files:**
- Create: `src-tauri/src/mangacon/database.rs`
- Modify: `src-tauri/src/mangacon/mod.rs`

- [ ] **Step 1: Write failing Rust tests**

Create tests that build a temporary MangaCon-compatible SQLite database with `mc3_mangas`, `mc3_volumes`, `mc3_badges`, and `mc3_tasks`. Assert that `queue_all_badged_updates()` finds only `mc3_badges.category = 1` + `mc3_volumes.status = 1`, inserts rows into `mc3_tasks`, creates `extra` as `{"mid":31,"vid":37246}`, and skips duplicate unfinished tasks.

- [ ] **Step 2: Run targeted test to verify RED**

Run: `cargo test mangacon::database --lib`

Expected: compile failure because `mangacon::database` does not exist.

- [ ] **Step 3: Implement minimal adapter**

Implement `MangaConDatabase::open(path)`, `list_badged_updates()`, and `queue_all_badged_updates(limit)`. Use a transaction, compute `order_index` from `MAX(order_index)`, leave `errors` and `finished_tick` null, and return inserted/skipped task summaries.

- [ ] **Step 4: Run targeted test to verify GREEN**

Run: `cargo test mangacon::database --lib`

Expected: all new database tests pass.

### Task 2: Continue Dialog Auto-Confirm

**Files:**
- Create: `src-tauri/src/mangacon/confirm.rs`
- Modify: `src-tauri/src/mangacon/mod.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Write failing pure tests**

Test the dialog title/message/button matching helpers so that only a MangaCon dialog containing "继续之前未完成的下载任务吗？" and a "是" button is accepted.

- [ ] **Step 2: Run targeted test to verify RED**

Run: `cargo test mangacon::confirm --lib`

Expected: compile failure because `mangacon::confirm` does not exist.

- [ ] **Step 3: Implement Win32 helper**

Implement `confirm_continue_download_dialog()` using `FindWindowW`, `EnumChildWindows`, `GetWindowTextW`, and `SendMessageW(BM_CLICK)` against the "是" button. Do not move the mouse.

- [ ] **Step 4: Run targeted test to verify GREEN**

Run: `cargo test mangacon::confirm --lib`

Expected: matching helper tests pass.

### Task 3: Tauri Command and Frontend API

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/api.ts`
- Modify: `src/features/automation/AutomationView.tsx`
- Modify: `src/features/dashboard/Dashboard.tsx`
- Test: `src/features/dashboard/Dashboard.test.tsx`

- [ ] **Step 1: Write failing frontend tests**

Update dashboard tests to expect "数据库队列" wording and verify the button still invokes the update callback. Add API type coverage for the new queue result in `src/lib/api.test.ts` if needed.

- [ ] **Step 2: Run targeted tests to verify RED**

Run: `npm test -- --run src/features/dashboard/Dashboard.test.tsx`

Expected: failing assertion because the UI still presents old automation language.

- [ ] **Step 3: Wire command**

Expose `queue_mangacon_updates(mangacon_database_path, executable_path, max_updates)` from Rust. It queues updates, launches MangaCon when tasks were queued, then attempts auto-confirm for a short bounded loop.

- [ ] **Step 4: Wire UI**

Replace dashboard one-click update behavior with the new command result, and keep the automation page as a monitoring/fallback surface. Show queued, skipped, total update counts, and confirmation status.

- [ ] **Step 5: Run targeted tests to verify GREEN**

Run: `npm test -- --run src/features/dashboard/Dashboard.test.tsx`

Expected: dashboard tests pass.

### Task 4: Full Verification, Commit, Release

**Files:**
- Modify: `package.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`

- [ ] **Step 1: Bump version**

Bump app version from `1.0.1` to `1.0.2` in package and Tauri metadata.

- [ ] **Step 2: Run full verification**

Run:
`npm test -- --run`
`cargo test`
`npm run build`
`npm run tauri build`

- [ ] **Step 3: Commit**

Commit with: `feat: queue MangaCon updates through SQLite`

- [ ] **Step 4: Release artifact**

Confirm the generated Windows installer exists under `src-tauri/target/release/bundle/nsis/`.
