# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

工作周报助手 (Weekly Report Assistant) — a Tauri v2 desktop app that drives weekly-report generation from Pomodoro time-tracking. The workflow is: write a plain-text weekly plan → run Pomodoro sessions against planned/adhoc tasks → on Friday auto-generate a Markdown weekly report, carrying unfinished tasks into next week's plan.

The UI and all user-facing strings are Chinese (zh-CN). Match this when adding UI text.

## Problem-tracking workflow

User-reported problems are **recorded, not fixed immediately**. The user decides modification order and priority; when asked to fix generally (no specific `P###`), Claude assesses priority by impact and fixes only the top 3 — not all at once.

- Problem list lives at `PROBLEMS.md` (repo root) — plain Markdown so other tools can read it.
- When the user reports a problem, append a new row to the **问题清单** table in `PROBLEMS.md`: next `P###` id, 状态 `待处理`, 模块/页面, 问题描述, today's date. Leave 优先级 blank for the user to fill.
- Do **not** start modifying code for a recorded problem unless the user explicitly asks (e.g. "修 P002"/"开始处理…"). If unsure whether something is blocking/urgent, ask before recording vs. fixing.
- When fixed and self-checked (build/type-check passing), set 状态 `已修复` (note what was done in 备注); when the user confirms it in the running app, set `已关闭` and delete that problem's records (table row + its 方案 section) from `PROBLEMS.md`.
- Status values: 待处理 / 已修复 / 已关闭. Priority values (高 / 中 / 低) — user-assigned, or assessed by Claude by impact when the user hasn't assigned them.

## Commands

Frontend (run from repo root, uses pnpm):
- `pnpm dev` — Vite dev server only (port 1420, fixed; Tauri expects this).
- `pnpm build` — type-check (`vue-tsc --noEmit`) then Vite build to `dist/`.
- `pnpm tauri:dev` — full app: starts Vite + launches the Rust shell (this is the normal dev loop).
- `pnpm tauri:build` — production bundle.

Rust (run from `src-tauri/`):
- `cargo test` — run the unit tests in `parser.rs` and `report.rs`.
- `cargo test <name>` — single test, e.g. `cargo test minutes_to_days_basic`.
- `cargo build` — compile backend only (faster iteration when not touching UI).

There is no frontend test runner and no linter configured. Type errors surface only via `pnpm build` / `vue-tsc`.

## Architecture

Two-process Tauri app: a Vue 3 frontend (`src/`) and a Rust backend (`src-tauri/src/`). They communicate exclusively through Tauri `invoke` commands. Every backend command is wrapped in `src/api.ts` on the TS side, and the shared data shapes are duplicated in `src/types/index.ts` (mirroring `src-tauri/src/models.rs`) — keep these two in sync when changing models.

### Backend (`src-tauri/src/`)
- `lib.rs` — Tauri builder; registers plugins (clipboard, dialog, fs), runs `db::init_db` on setup, and lists every `#[tauri::command]` in `generate_handler!`. **Any new command must be added here or it won't be callable from the frontend.**
- `commands.rs` — all `#[tauri::command]` functions. Convention: every command returns `Result<T, String>` (Tauri v2 requires `Err: Serialize`; `anyhow::Error` doesn't impl it, so errors are stringified via the `s()` helper). DB access is via `State<'_, DbState>`, a `Mutex<Connection>`.
- `db.rs` — SQLite schema (`SCHEMA` const) and `init_db`. The single `weekly.db` lives in the OS app-data dir. Four tables: `weeks`, `planned_tasks`, `adhoc_tasks`, `pomodoro_sessions`.
- `parser.rs` — parses the free-text weekly plan into structured tasks. See "Plan parser" below. Has unit tests.
- `report.rs` — derives task status from Pomodoro minutes, converts minutes→days, and renders Markdown. Has unit tests.
- `models.rs` — serde structs/enums shared with the frontend.

### Frontend (`src/`)
- `main.ts` — Pinia + vue-router (hash history). Three routes: `/plan`, `/timer`, `/report` (default redirect `/` → `/timer`).
- `api.ts` — one typed wrapper per backend command; this is the only place that calls `invoke`.
- `stores/` — Pinia stores (`plan`, `timer`, `report`) hold all state; views are mostly thin.
- `views/` — `PlanInputView` (plan entry + live preview), `TimerView` (Pomodoro), `ReportView` (report + Friday carry-over). `components/` holds the modals and the countdown ring.
- Uses Naive UI (`NConfigProvider` with `zhCN` locale) wrapped around the shell in `App.vue`.

## Domain rules worth knowing

### Workflow & timing
The app is organized around a Tuesday-start work week with fixed checkpoints:
- **Input window** (Mon evening → Tue morning): write this week's plan on the Plan page.
- **Tue afternoon reminder**: if the current week has no plan, show an in-app banner. Spec calls for the banner on the Timer page top **and** a hint on the Plan page (see Known gaps — the Plan-page hint isn't wired yet).
- **Execution** (Tue → next Mon): run Pomodoro sessions, each tagged to a task.
- **Friday report** (manually triggered): aggregate stats → carry-over checkbox modal → generate Markdown report.
- **Week boundary**: `week_start` = this Tuesday 00:00, `week_end` = next Monday 23:59. A Pomodoro session belongs to whichever week its `started_at` falls in. Reminders are in-app only — no system notifications, to avoid permission prompts.

### Rules
- **Work week is Tuesday→next Monday** (`current_week_range` in `parser.rs`). A Monday is treated as still belonging to the *previous* week (offset `-6`), since Monday is for wrapping up last week's tasks. This "current week" lookup key (`week_start`) is how the app finds/creates the active `weeks` row everywhere.
- **1d = 8h = 480min** throughout. Estimate parsing and minute→day conversion both assume this.
- **Plan parser** (`parser.rs`): input is split into blocks by blank lines; each block's first line is the project name, subsequent lines are tasks. A leading `N.`/`N、`/`N)` gives `sort_order`; unnumbered tasks get a globally-continued sequence (max existing + 1, e.g. if the max numbered task is 5 the next unnumbered one becomes 6). Trailing estimate supports `0.5d`, `-0.5d`, `0.25` (no `d`), and comma decimals like `1,25d`. Parse failures yield `estimate_d = 0` and an entry in `errors`, not a hard error. The canonical input example and full rules live in the `parser.rs` module doc and are covered by unit tests there.
- **Task status** (`report.rs::determine_status`) — the spec's status table:

  | 状态 | 判定 | 用时来源 | 进下周计划 |
  |---|---|---|---|
  | [已完成] done | 番茄钟累计 ≥ 预估（预估=0 时走过番茄钟即完成） | 番茄钟实际换算 d | 否 |
  | [进行中] in_progress | 累计 < 预估 且 > 0，周五未勾选 | 番茄钟实际换算 d | 是 |
  | [计划下周一完成] next_monday | 未达预估 且周五弹窗勾选 | **预估值**（本周不冻结） | 否 |
  | [未开始] not_started | 累计 = 0 且周五未勾选 | 不显示用时 | 是 |

  Minutes→days rounds the fractional part *up* to the nearest of `[0.1, 0.2, 0.25, 0.5, 0.75]`; full days accumulate separately (e.g. 180min→0.5d, 30min→0.1d, 600min→1.25d).
- **Friday carry-over** (`carry_over_tasks`): checked tasks are marked `plan_next_monday=1`; all `in_progress`/`not_started` planned tasks are copied into next week's `planned_tasks` with `carried_from` pointing back at this week's task id.
- **Report output format** (`render_markdown` in `report.rs`): title line; per-project `## 项目（总用时 Nd）` headers (adhoc tasks appended to their chosen project, indistinguishable from planned in totals); task lines `序号.内容 [状态] 用时d` where 未开始 omits the time; final `## 下周计划` listing 进行中 (with remaining estimate) + 未开始, preserving original `sort_order` numbers.
- **End-of-focus task picker** (`TaskPickerModal.vue` + `stores/timer.ts`): when a focus Pomodoro ends, a modal asks "这个番茄钟你在做什么？". It default-highlights the previous session's task (via `lastTaskSource`/`lastTaskId`), lists this week's planned + historical adhoc tasks grouped by project, and lets the user create an adhoc task (project + title) — which then writes one `pomodoro_sessions` row bound to the chosen task.

## Known gaps vs. the MVP spec
These are verified divergences between the spec and the current code — treat as TODOs, not intentional designs:
- **Report title lacks ISO week number and weekday names** — `render_markdown` (`report.rs:210`) emits `# 工作周报（2024-06-11 ~ 2024-06-17）`; spec wants `# 工作周报（2024-W24 · 06/11 周二 ~ 06/17 周一）`.
- **Plan-page reminder hint missing** — `needs_plan_reminder` (`commands.rs:446`) drives a Timer-page banner (`TimerView.vue:29`) via `stores/report.ts`, but the spec'd hint on the Plan page is not wired (`PlanInputView.vue` / `stores/plan.ts` don't consume `needsReminder`).
- **Adhoc task project not enforced** — spec says adhoc creation "必须选/建项目", but `stores/timer.ts:161` passes `""` when blank and `create_adhoc_task` (`commands.rs:277`) accepts it.

## Conventions

- Backend errors: return `Result<T, String>`, convert via `.map_err(s)?`. Don't return `anyhow::Error` from commands.
- DB: lock the connection with `state.0.lock().unwrap()` per command. `ensure_week_id` is the helper to get-or-create the current week's id.
- When adding a Tauri command: implement in `commands.rs`, register in `lib.rs` `generate_handler!`, add a typed wrapper in `src/api.ts`, and mirror any new model in `src/types/index.ts`.
- `@` alias → `src/` (configured in both `vite.config.ts` and `tsconfig.json`).
