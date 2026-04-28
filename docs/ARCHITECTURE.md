# OLManager Architecture

This document is a practical map for contributors. It describes the current architecture in the repository, not an aspirational rewrite.

OLManager is a desktop game built with **Tauri v2**: a **React + TypeScript** frontend runs in the WebView, while gameplay state, persistence, and long-running simulation logic live in the **Rust backend**.

## System overview

```text
React UI (src/)
  pages, components, hooks, stores, services
        │
        │ @tauri-apps/api invoke("command_name")
        ▼
Tauri command layer (src-tauri/src/commands/)
        │
        ├─ application services (src-tauri/src/application/)
        ├─ in-memory session state (ofm_core::state::StateManager)
        ├─ domain/gameplay crates (domain, ofm_core, engine)
        └─ persistence crate (db, SQLite save files)
```

The frontend should present state, collect user intent, and call typed service functions. The backend owns authoritative game state, simulations, save/load, and mutations that affect the career.

## Frontend architecture

Frontend code lives under `src/` and is built by Vite.

- `main.tsx` mounts React in `StrictMode`, wraps the app with `ThemeProvider`, and initializes i18n.
- `App.tsx` defines lazy-loaded routes with `react-router-dom` for `/`, `/select-team`, `/dashboard`, `/match`, and `/settings`.
- `pages/` contains route-level screens such as the main menu, dashboard, team selection, match simulation, and settings.
- `components/` contains feature UI and reusable UI pieces. Several feature areas have local view-model/helper files and tests.
- `services/` is the frontend IPC adapter layer. Services call Tauri commands via `invoke(...)` and expose TypeScript-friendly functions such as `advanceTimeWithMode`, `skipToMatchDay`, and player/training/staff actions.
- `store/` uses Zustand for client-side UI/session state. `gameStore.ts` tracks active game data returned by Rust; `settingsStore.ts` loads and persists settings through backend commands.
- `hooks/` contains UI orchestration hooks. For example, `useAdvanceTime` coordinates modals/navigation and delegates the actual mutation to `advanceTimeService`.
- `i18n/` configures `i18next`/`react-i18next` and locale JSON files.
- `lib/` and `utils/` hold frontend-only helpers, formatting, lightweight calculations, and backend-to-UI translation utilities.

Frontend tests use **Vitest**, **jsdom**, and **React Testing Library**. The test configuration is in `vite.config.ts`; tests are colocated as `*.test.ts` and `*.test.tsx` under `src/`.

## Tauri boundary

Tauri commands are registered in `src-tauri/src/lib.rs` with `tauri::generate_handler![...]`. Command modules live in `src-tauri/src/commands/` and are grouped by feature (`game`, `time`, `transfers`, `squad`, `staff`, `settings`, `live_match`, stats, etc.).

Use this boundary deliberately:

- Frontend code should call command names through small service functions in `src/services/`, not scatter raw `invoke(...)` calls throughout components.
- Tauri commands should validate inputs, load/update `StateManager`, call application/core/db functions, and return serializable DTOs or domain structures.
- Business rules that must be consistent across UI flows belong in Rust (`ofm_core`, `engine`, or application services), not in React components.
- UI-only state, presentation preferences, and navigation belong in React/Zustand/hooks.

The backend keeps process-level state with Tauri-managed objects:

- `ofm_core::state::StateManager` stores the active `Game`, stats state, live match session, and active save id behind mutexes.
- `SaveManagerState` wraps `db::save_manager::SaveManager` for save listing/loading/saving/deleting.

## Rust workspace and crate responsibilities

The Rust backend is a workspace declared in `src-tauri/Cargo.toml`.

### `domain`

`src-tauri/crates/domain` defines serializable domain data types: players, teams, leagues, managers, staff, messages, news, season context, stats, negotiations, and identity structures.

Keep this crate model-focused. It currently depends only on general-purpose libraries such as `serde`, `serde_json`, and `log`, and should not know about Tauri, SQLite, or frontend concerns.

### `engine`

`src-tauri/crates/engine` contains match simulation logic. It exposes simulation functions and match types such as `simulate`, `LiveMatchState`, `MatchCommand`, `MatchSnapshot`, `MatchReport`, and `TeamData`.

This crate is intentionally separate from Tauri and persistence so match simulation can be tested independently.

### `ofm_core`

`src-tauri/crates/ofm_core` contains gameplay/application domain logic: game state, clock, club systems, contracts, finances, training, scouting, transfers, schedules, turns, live match management, season logic, player events, generated messages/news, and job offers.

It depends on `domain` and `engine`. The central career object is `ofm_core::game::Game`, and runtime session state is managed by `ofm_core::state::StateManager`.

### `db`

`src-tauri/crates/db` owns SQLite persistence. It contains:

- `GameDatabase`, which opens per-save SQLite databases and applies migrations.
- `migrations` and `sql/`, which define schema evolution.
- `repositories/`, which map domain/core state to tables.
- `GamePersistenceReader` and `GamePersistenceWriter`, which reconstruct and persist `Game`/stats state.
- `SaveManager`, `SaveIndexManager`, and `save_index`, which manage save files, metadata, checksums, and save discovery.
- `legacy_migration`, which handles old save migration on startup.

The `db` crate depends on `domain` and `ofm_core`, but gameplay code should not depend on SQLite details.

### Tauri app crate

`src-tauri/src` wires the desktop application together. `lib.rs` configures plugins, logging, managed state, app data directories, legacy save migration, and command registration. `application/` contains backend orchestration that is too app-specific for the pure crates, such as time advancement and live-match flow coordination.

## Persistence and save/load model

OLManager uses a per-save SQLite model:

1. On startup, Tauri creates the app data directory and initializes `SaveManager` in an app-data `saves/` directory.
2. Starting a new game creates a new save database through `SaveManager::create_save` and stores its id in `StateManager`.
3. `GameDatabase::open` creates or opens a `.db` file and applies all migrations before use.
4. `GamePersistenceWriter` writes game metadata, manager, teams, players, staff, messages, news, league, objectives, scouting assignments, and stats through repositories.
5. `GamePersistenceReader` loads the same tables back into an `ofm_core::game::Game` and refreshes derived season context.
6. The save index records save id, name, manager name, db filename, checksum, creation time, and last played time.
7. `save_game` persists the active game and stats. `exit_to_menu` auto-saves when there is an active save id, then clears in-memory state.

Settings are separate from career saves: `get_settings`/`save_settings` read and write `settings.json` in the app data directory.

## Dependency direction and architectural rules

The current code supports this dependency direction:

```text
React UI → frontend services → Tauri commands/application
Tauri commands/application → ofm_core / engine / db
db → ofm_core + domain
ofm_core → domain + engine
engine → standalone simulation types/logic
domain → serializable model types only
```

Contributor rules of thumb:

- Do not put durable business rules only in React. If a rule changes saved game state or simulation results, implement it in Rust and expose it through a command.
- Keep `domain` free of Tauri, SQLite, and UI-specific code.
- Keep `engine` focused on simulation. Do not make it depend on save files or Tauri commands.
- Keep persistence behind `db` repositories/persistence readers/writers. Do not issue SQLite queries from command modules.
- Keep command modules thin enough to be understandable: parse/validate input, call core/application/db, update `StateManager`, return data.
- Keep frontend `services/` as the IPC boundary. Components and hooks should use service functions instead of raw command strings when possible.

## Testing strategy

- Frontend: `npm test` runs Vitest in jsdom. Use React Testing Library for components/pages/hooks and plain Vitest for helpers, stores, and services.
- TypeScript contract checks: `npm run build:types` runs the release TypeScript config without creating a Tauri production bundle.
- Rust formatting/linting: use `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, `cargo check`, and `cargo clippy --workspace --all-targets -- -D warnings`.
- Rust tests: `cargo test --manifest-path src-tauri/Cargo.toml --workspace` covers crates such as `engine`, `ofm_core`, `db`, and command-level tests.

Do not run production Tauri bundle builds for normal documentation or PR validation work.

## Adding a new feature safely

1. Decide where the rule belongs. UI-only behavior goes in React; game-state mutations and simulations go in Rust.
2. Add or extend domain types in `domain` only when the model needs new durable fields or shared serializable structures.
3. Implement gameplay behavior in `ofm_core` or simulation behavior in `engine` with crate-level tests.
4. If the feature must be saved, add a migration and repository/persistence updates in `db`.
5. Expose the behavior through a Tauri command in `src-tauri/src/commands/` and register it in `lib.rs`.
6. Add a typed frontend service wrapper in `src/services/`.
7. Update Zustand stores/hooks/pages/components only for presentation and UI flow.
8. Add or update tests at the lowest useful layer first, then add UI tests for user-visible behavior.
9. Update docs and data provenance notes when the feature touches inherited assets, generated data, or third-party sources.

When in doubt, follow the dependency direction above. The UI can ask for a change; Rust decides whether the career state is valid.
