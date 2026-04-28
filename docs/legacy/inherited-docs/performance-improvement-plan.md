# Plan de mejora de rendimiento para OLManager

## Resumen ejecutivo

OLManager ya tiene una base razonable para escalar: el frontend usa Vite con `manualChunks` (`vite.config.ts`), las páginas principales se cargan con `React.lazy` (`src/App.tsx`) y el backend Rust concentra la lógica pesada en crates separados (`src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`). El problema no es "falta total de arquitectura", sino una combinación de **payloads demasiado grandes**, **estado global monolítico**, **clonado masivo del `Game` entero en IPC**, **cómputos repetidos sobre arrays grandes en render**, y **ausencia de profiling/telemetría sistemática**.

La mejora de mayor impacto no pasa por micro-optimizar componentes aislados. Pasa por atacar 4 frentes: **(1) medir primero**, **(2) reducir tamaño y frecuencia de payloads frontend↔Rust**, **(3) cortar rerenders derivados del store global**, y **(4) sacar del critical path los datos pesados y assets innecesarios**. Sin eso, cualquier optimización cosmética será ruido.

---

## Arquitectura actual relevante para rendimiento

### 1. Shell desktop y backend

- La app usa **Tauri v2** con ventana única y `csp: null` en `src-tauri/tauri.conf.json`.
- El backend se monta en `src-tauri/src/lib.rs`:
  - registra `StateManager::new()` como estado global.
  - inicializa `SaveManager` en `setup()`.
  - expone decenas de comandos Tauri por `invoke_handler!`.
- `StateManager` (`src-tauri/crates/ofm_core/src/state.rs`) guarda:
  - `active_game: Mutex<Option<Game>>`
  - `active_stats: Mutex<Option<StatsState>>`
  - `live_match: Mutex<Option<LiveMatchSession>>`
  - `active_save_id: Mutex<Option<String>>`

### 2. Frontend

- Bootstrap en `src/main.tsx` con `React.StrictMode`, `ThemeProvider` y `App`.
- `src/App.tsx`:
  - usa `BrowserRouter`.
  - carga páginas top-level con `lazy()`.
  - carga settings por `useSettingsStore.loadSettings()`.
  - aplica idioma, escala y alto contraste en efectos.
- Rutas principales:
  - `/` → `MainMenu`
  - `/select-team` → `TeamSelection`
  - `/dashboard` → `Dashboard`
  - `/match` → `MatchSimulation`
  - `/settings` → `Settings`

### 3. Estado frontend

- `src/store/gameStore.ts`: un único store Zustand con `gameState: GameStateData | null`.
- `src/store/settingsStore.ts`: otro store Zustand para settings, con persistencia vía Tauri (`get_settings` / `save_settings`).
- El store de juego reemplaza el `gameState` completo con `setGameState(state)` tras casi cualquier comando.

### 4. Persistencia

- Saves por archivo SQLite individual vía `SaveManager` (`src-tauri/crates/db/src/save_manager.rs`).
- Cada save usa `GameDatabase::open()` y aplica migraciones siempre (`src-tauri/crates/db/src/game_database.rs`).
- La escritura completa del juego pasa por `GamePersistenceWriter::write_game()` (`src-tauri/crates/db/src/game_persistence.rs`), que hace upsert de manager, teams, players, staff, messages, news, league, objectives y scouting.
- `StatsState` se persiste con borrado completo + reinsert (`src-tauri/crates/db/src/repositories/stats_repo.rs`).

### 5. Carga de datos / assets

- i18n carga **todas** las traducciones en el bundle inicial (`src/i18n/index.ts`): ~693 KB de JSON sin comprimir.
- Hay múltiples imports directos de JSONs de draft en frontend:
  - `data/lec/draft/players.json` (~97 KB)
  - `data/lec/draft/champions.json` (~266 KB)
  - `data/lec/draft/teams.json`
  - `data/lec/draft/ai-config.json`
- El árbol `public/` pesa ~26.1 MB y contiene muchas fotos PNG pesadas.
- Varias vistas consumen imágenes remotas de `raw.communitydragon.org` y `ddragon.leagueoflegends.com` en runtime (`SquadRosterView.tsx`, `TacticsTab.tsx`, `ChampionDraft.tsx`, `HomeRosterLineupCard.tsx`).

### 6. Pantallas críticas

- `MainMenu.tsx`: creación/carga de partida.
- `TeamSelection.tsx`: listado de equipos + cálculo derivado de roster.
- `Dashboard.tsx`: shell principal, carga `get_active_game`, calcula búsqueda, alerts, navegación interna y renderiza tabs.
- `MatchSimulation.tsx` + `MatchLive.tsx`: orquestación del match day y stepping minuto a minuto por IPC.
- `PlayerProfile.tsx` y `TeamProfile.tsx`: lecturas adicionales de histórico/stats.

---

## Hallazgos concretos del repositorio

### A. Cold start / arranque

#### Hallazgo A1 — i18n mete todos los idiomas en el arranque
- Evidencia: `src/i18n/index.ts` importa `en.json`, `es.json`, `pt.json`, `fr.json`, `de.json`, `it.json`, `pt-BR.json` al iniciar.
- Impacto:
  - aumenta JS parse/compile en WebView.
  - penaliza cold start aunque el usuario solo use un idioma.
- Tamaño observado: locales ~693518 bytes en total.

#### Hallazgo A2 — `ThemeContext` y `App` hacen trabajo síncrono temprano
- Evidencia:
  - `src/context/ThemeContext.tsx` lee `localStorage` y `matchMedia` en inicialización.
  - `src/App.tsx` dispara `loadSettings()` al montar.
- No es dramático por sí solo, pero suma trabajo antes de estabilizar UI.

#### Hallazgo A3 — assets estáticos muy pesados
- Evidencia:
  - `public/` ~26.1 MB.
  - `openfootlogo.svg` ~897811 bytes.
  - múltiples fotos PNG entre ~480 KB y ~700 KB.
- Riesgo: bundle final y primer acceso a vistas con imágenes grandes más lentos de lo necesario.

### B. Tiempo hasta interactivo (TTI)

#### Hallazgo B1 — Dashboard depende de `get_active_game` completo
- Evidencia: `src/pages/Dashboard.tsx` línea 111 invoca `get_active_game` y luego `setGameState(state)`.
- En backend: `src-tauri/src/commands/game.rs` devuelve `Game` completo clonado.
- Esto significa que TTI del dashboard depende de:
  - lock de `StateManager`
  - clon profundo de `Game`
  - serialización Rust→JSON
  - IPC Tauri
  - parse JSON en frontend
  - rerender de toda la vista basada en `gameState`

#### Hallazgo B2 — TeamSelection recupera dataset amplio para fallback
- Evidencia: `src/pages/TeamSelection.tsx` usa `get_team_selection_data()` si no hay `gameState`, y ese comando devuelve `manager + teams + players` completos (`src-tauri/src/commands/game.rs`).
- En la UI luego se hacen múltiples `filter()`/`reduce()` por equipo para calcular roster y OVR.

### C. Render/UI

#### Hallazgo C1 — Dashboard recalcula demasiados derivados en cada render
- Evidencia: `src/pages/Dashboard.tsx` calcula en render:
  - `getTodayMatchFixture(gameState)`
  - `getUnreadMessagesCount(gameState)`
  - `getManagerTeamName(gameState)`
  - `getDashboardSearchResults(gameState, searchQuery)`
  - `getDashboardAlerts(gameState, hasMatchToday, t)`
  - `createDashboardTabContentModel(...)`
- `dashboardHelpers.ts` usa múltiples `.filter()`, `.find()`, `.some()` sobre `players`, `staff`, `messages`, `teams`, `fixtures`.
- Con datasets mayores, esto se volverá claramente visible.

#### Hallazgo C2 — tabs del Dashboard están en el mismo chunk lógico de página
- Evidencia: `DashboardTabContent.tsx` importa directamente `HomeTab`, `SquadTab`, `TacticsTab`, `TrainingTab`, `ScheduleTab`, `FinancesTab`, `TransfersTab`, `PlayersListTab`, `TeamsListTab`, `TournamentsTab`, `ScoutingTab`, `YouthAcademyTab`, `StaffTab`, `InboxTab`, `ManagerTab`, `NewsTab`, `EndOfSeasonScreen`.
- Aunque renderice una sola tab, el coste de parse/compile/import del árbol de Dashboard crece con todo ese grafo.

#### Hallazgo C3 — hay cómputos por render en listas y cards sin índices ni memo suficiente
- Evidencia:
  - `PlayersListTab.tsx`: filtra, ordena y pagina `gameState.players` en cada render.
  - `TeamsListTab.tsx`: por cada equipo hace `players.filter(...)`, `reduce(...)`, `findIndex(...)`, `find(...)`.
  - `TrainingTab.tsx`: recalcula medias y ordena roster inline.
  - `HomeTab.tsx`: hace sorts/slices sobre standings/news/messages.
- Esto hoy puede ser tolerable con pocos equipos; no lo será con mundos más grandes.

#### Hallazgo C4 — uso incorrecto de `useMemo` para side effect
- Evidencia: `PlayersListTab.tsx` línea 61 usa `useMemo(() => setPage(1), [filterKey])`.
- Eso es un anti-pattern. Debe ser `useEffect`. No es solo estilo: ensucia el modelo mental, puede introducir renders extra y dificulta perf debugging.

### D. Gestión de estado y rerenders

#### Hallazgo D1 — store de juego monolítico
- Evidencia: `src/store/gameStore.ts` almacena `gameState` completo y `setGameState` reemplaza todo el objeto.
- Resultado:
  - cualquier actualización de una parte del juego invalida consumidores del objeto entero.
  - hace muy difícil aislar rerenders por dominio.

#### Hallazgo D2 — muchas mutaciones frontend reciben `GameStateData` completo de vuelta
- Evidencia frontend:
  - `trainingService.ts`, `transfersService.ts`, `inboxService.ts`, `scoutingService.ts` y muchos componentes esperan `GameStateData` completo como respuesta.
- Evidencia backend:
  - muchísimos comandos devuelven `Game` completo: `set_training`, `toggle_transfer_list`, `mark_message_read`, `send_scout`, etc. (`src-tauri/src/lib.rs` + `src-tauri/src/commands/*.rs`).
- Eso acopla UX, store y transport payload a la entidad más pesada posible.

### E. IPC Tauri ↔ frontend

#### Hallazgo E1 — patrón general de clone + mutate + clone + return
- Evidencia:
  - `StateManager` expone `get_game` sobre `Mutex<Option<Game>>` (`state.rs`).
  - grep en `src-tauri/src` muestra decenas de `get_game(|g| g.clone())` y `state.set_game(game.clone())`.
  - Ejemplos directos: `commands/squad.rs`, `commands/transfers.rs`, `commands/messages.rs`, `commands/game.rs`, `application/time_advancement.rs`.
- Impacto:
  - coste CPU/memoria en Rust.
  - payloads IPC sobredimensionados.
  - GC / parse pressure en WebView.

#### Hallazgo E2 — live match hace 2 IPC por minuto simulado
- Evidencia: `src/components/match/MatchLive.tsx`
  - `step_live_match({ minutes: 1 })`
  - luego `get_match_snapshot()`
- Con velocidad normal/rápida esto genera un patrón muy caro de roundtrip continuo.

#### Hallazgo E3 — perfiles/estadísticas se resuelven por llamadas separadas y sin caché
- Evidencia:
  - `PlayerProfile.tsx` carga `get_player_match_history(limit: 500)` en `useEffect`.
  - `useTeamProfileStats.ts` hace `Promise.allSettled([fetchTeamStatsOverview, fetchTeamRecentMatches])`.
- No hay cache local, prefetch, deduplicación ni invalidación dirigida.

### F. Disco / base de datos / parsing

#### Hallazgo F1 — `save_game` reescribe el estado entero
- Evidencia:
  - `commands/game.rs::save_game`
  - `GamePersistenceWriter::write_game()` hace upsert de casi todas las tablas.
- No hay persistencia incremental.

#### Hallazgo F2 — `StatsState` se persiste con DELETE + INSERT completo
- Evidencia: `stats_repo.rs::replace_stats_state()`:
  - `DELETE FROM player_match_stats`
  - `DELETE FROM team_match_stats`
  - luego inserta fila a fila.
- Con históricos grandes, esto será un cuello de botella claro de autosave/cierre.

#### Hallazgo F3 — histórico se carga entero en memoria y luego se filtra in-memory
- Evidencia:
  - `load_stats_state()` carga todas las filas de `player_match_stats` y `team_match_stats` a `StatsState` en memoria.
  - luego `get_player_match_history_internal()` y `get_team_match_history_internal()` filtran sobre vectores completos en memoria.
- Hay índices SQL, sí, pero NO se aprovechan para estas consultas de runtime porque el acceso ya se hace sobre la copia en memoria.

#### Hallazgo F4 — `start_new_game` lee/parsing JSON world en cada inicio
- Evidencia: `src-tauri/src/commands/game.rs` lee `src-tauri/databases/lec_world.json` y llama a `load_world_from_json()`.
- Tamaño observado del JSON world: ~135861 bytes. Hoy no es enorme, pero el patrón escalará mal si el mundo crece.

### G. Bundles y assets

#### Hallazgo G1 — Dashboard y match usan JSONs de draft pesados dentro del bundle
- Evidencia:
  - `ChampionDraft.tsx` importa `teams.json`, `players.json`, `champions.json`, `ai-config.json`.
  - `PlayerProfile.tsx`, `SquadRosterView.tsx`, `NextMatchDisplay.tsx`, `HomeRosterLineupCard.tsx`, `ScheduleTab.tsx`, `DraftResultScreen.tsx` importan parte de esos JSONs.
- Esto mete datos de negocio en chunks JS, no como recursos bajo demanda.

#### Hallazgo G2 — dependencia de imágenes remotas en runtime
- Evidencia:
  - `raw.communitydragon.org` para role icons.
  - `ddragon.leagueoflegends.com` para portraits/splashes.
- Riesgos:
  - TTI condicionado por red externa en ciertas vistas.
  - comportamiento pobre offline.
  - más variabilidad de rendimiento y observabilidad más difícil.

#### Hallazgo G3 — chunking manual existe, pero es todavía superficial
- Evidencia: `vite.config.ts` separa `react-vendor`, `router`, `tauri`, `i18n`, `icons`.
- Bien, pero no separa dominios propios costosos como `dashboard`, `match`, `draft`, `profiles`, `history`.

### H. Memoria

#### Hallazgo H1 — estado duplicado en backend y frontend
- Backend mantiene `Game` y `StatsState` en memoria (`StateManager`).
- Frontend mantiene una versión serializada completa en Zustand (`gameStore.ts`).
- Cada IPC que devuelve `Game` crea nuevas copias intermedias.

#### Hallazgo H2 — live match conserva `events: Vec<MatchEvent>` y snapshot amplio
- Evidencia: `engine/src/live_match/mod.rs::MatchSnapshot` incluye eventos, benches, stats, set pieces, tarjetas, sustituciones, etc.
- Si el frontend pide snapshots completos con mucha frecuencia, la presión de memoria sube rápido.

### I. Listas/tablas grandes

#### Hallazgo I1 — sin virtualización
- Evidencia visible en:
  - `PlayersListTab.tsx`
  - `InboxTab.tsx` / panes asociados
  - `TrainingTab.tsx` listado de fitness
  - `TournamentsTab.tsx` tablas de standings/fixtures
  - `SquadRosterView.tsx`
- Hoy hay paginación en algunos sitios, pero no virtualización real.

#### Hallazgo I2 — complejidad O(n*m) repetida en vistas compuestas
- `TeamsListTab.tsx` por cada team filtra roster completo.
- `NextMatchDisplay.tsx` reconstruye lineup por rol filtrando y ordenando por rol para home y away.
- `HomeRosterLineupCard.tsx` repite lógica similar.

### J. Observabilidad / medición

#### Hallazgo J1 — no hay instrumentación de rendimiento en frontend
- Búsqueda sin resultados para `performance.mark`, `performance.measure`, `Profiler`, `console.time` en `src/`.
- Solo hay logs funcionales con `console.info/debug/warn/error`.

#### Hallazgo J2 — en backend hay logging, pero no profiling estructurado
- Evidencia: `tauri_plugin_log` configurado en `src-tauri/src/lib.rs`.
- No se ven benchmarks, spans, flamegraphs, ni métricas temporales persistentes.

### K. Tauri / Rust / WebView específico

#### Hallazgo K1 — workaround Linux correcto, pero sin tuning adicional del pipeline
- Evidencia: `WEBKIT_DISABLE_DMABUF_RENDERER=1` en Linux (`src-tauri/src/lib.rs`).
- Correcto como workaround, pero no hay estrategia adicional específica de WebView para tiempo de parse/render.

#### Hallazgo K2 — uso de `BrowserRouter` en desktop webview
- Evidencia: `src/App.tsx` usa `BrowserRouter`.
- No es el principal problema de perf, pero en Tauri suele evaluarse si `HashRouter` simplifica navegación/recarga y reduce edge cases de routing. Es más estabilidad que velocidad, pero conviene revisarlo dentro del profiling de navegación.

---

## Riesgos y anti-patrones detectados

1. **Payload máximo por defecto**: devolver `Game` entero casi siempre.
2. **Store global grueso**: todo depende de `gameState` completo.
3. **Clonado sistemático en Rust**: patrón `clone → mutate → clone → return`.
4. **Datos de negocio embebidos en bundle JS**: JSONs de draft importados en múltiples vistas.
5. **Imágenes externas en runtime**: dependencias de red evitables.
6. **Persistencia full rewrite**: save/stats no incrementales.
7. **Consultas históricas in-memory no index-aware**: se pierde el valor de SQLite para lectura selectiva.
8. **Sin perfilado base**: tocar rendimiento ahora sin medir sería ingeniería a ciegas.
9. **Lógica derivada duplicada entre componentes**: lineup, roles, OVR, lookups.
10. **Incorrect side-effect API usage**: `useMemo(() => setPage(1), ...)`.

---

## Roadmap priorizado por impacto vs esfuerzo

### Prioridad 0 — Antes de optimizar nada: baseline y profiling
**Impacto:** máximo
**Esfuerzo:** bajo-medio

1. Medir cold start real Tauri → primera pintura → dashboard interactivo.
2. Medir tamaño real de payload IPC en:
   - `get_active_game`
   - `get_team_selection_data`
   - `advance_time_with_mode`
   - `set_training`
   - `mark_message_read`
   - `toggle_transfer_list`
   - `step_live_match`
   - `get_match_snapshot`
3. Medir tiempo de serialización/deserialización de `Game`.
4. Medir rerenders por tab crítica (`Dashboard`, `HomeTab`, `PlayersListTab`, `InboxTab`, `MatchLive`).

### Prioridad 1 — Quick wins de alto impacto
**Impacto:** alto
**Esfuerzo:** bajo

1. Lazy-load real de namespaces i18n o carga diferida por idioma.
2. Mover JSONs pesados de draft a carga bajo demanda por ruta/feature.
3. Corregir anti-patterns React (`useMemo` side effects, sorts/filter inline repetidos).
4. Introducir memo/selectores en cálculos derivados del dashboard.
5. Reducir frecuencia IPC del live match unificando `step + snapshot`.
6. Optimizar assets pesados y normalizar formatos.

### Prioridad 2 — Cambios estructurales de medio plazo
**Impacto:** muy alto
**Esfuerzo:** medio-alto

1. Cambiar contrato IPC: respuestas parciales / patches / DTOs específicos.
2. Reestructurar store frontend por slices y selectores estables.
3. Indexar y cachear derivadas de dominio (`playersByTeam`, `teamsById`, `messagesUnread`, etc.).
4. Persistencia incremental o por dominios en saves/stats.
5. Queries históricas directas a SQLite en vez de filtrar `StatsState` entero.

### Prioridad 3 — Largo plazo / avanzado
**Impacto:** muy alto
**Esfuerzo:** alto

1. Motor de snapshots parciales para live match.
2. Hydration parcial del `Game` en frontend.
3. Event sourcing ligero o dirty-tracking por agregado.
4. Virtualización real en listas grandes.
5. Benchmarks automatizados Rust + budget checks de bundle en CI.

---

## Quick wins futuros

1. **Lazy i18n por idioma**
   - Objetivo: sacar ~700 KB JSON del arranque inicial.
   - Archivos: `src/i18n/index.ts`, `src/App.tsx`, `src/store/settingsStore.ts`.

2. **Separar chunks propios de producto**
   - `dashboard`, `match`, `draft`, `profiles`, `history`, `inbox`, `tournaments`.
   - Archivo: `vite.config.ts`.

3. **Corregir `PlayersListTab`**
   - Pasar reset de página a `useEffect`.
   - Memoizar filtered/sorted list y precomputar `teamNameById`.
   - Archivo: `src/components/players/PlayersListTab.tsx`.

4. **Reducir recomputación en `TeamsListTab`**
   - Construir `playersByTeamId` una vez.
   - Construir `standingsByTeamId` una vez.
   - Archivo: `src/components/teams/TeamsListTab.tsx`.

5. **Memoizar helpers del Dashboard**
   - `searchResults`, `dashboardAlerts`, `todayMatchFixture`, `unreadCount`, `myTeamName`.
   - Archivos: `src/pages/Dashboard.tsx`, `src/components/dashboard/dashboardHelpers.ts`.

6. **Unificar `step_live_match` + snapshot**
   - Nuevo comando que devuelva `{ minute_results, snapshot }`.
   - Archivos: `src/components/match/MatchLive.tsx`, `src-tauri/src/commands/live_match.rs`, `src-tauri/src/application/live_match.rs`.

7. **Optimizar assets `public/player-photos/*` y `openfootlogo.svg`**
   - Comprimir y generar variantes.
   - Directorio: `public/`.

8. **Cache local de stats/perfiles**
   - Al menos por sesión, keyed por `playerId`/`teamId` + versión de `gameState`.
   - Archivos: `PlayerProfile.tsx`, `useTeamProfileStats.ts`.

---

## Cambios estructurales de medio plazo

### 1. Rediseñar contrato IPC

#### Problema
Hoy demasiados comandos devuelven `Game` entero.

#### Cambio propuesto
Definir DTOs por caso de uso:
- `DashboardSummaryDto`
- `InboxUpdateDto`
- `RosterUpdateDto`
- `TrainingUpdateDto`
- `TransferUpdateDto`
- `LiveMatchStepDto`

#### Beneficio
- menos bytes por IPC
- menos parse JSON
- menos invalidación de store
- posibilidad de actualizaciones parciales

### 2. Particionar store frontend

#### Problema
`gameStore` concentra todo en `gameState`.

#### Cambio propuesto
Separar slices derivadas o normalizadas:
- `entities.teamsById`
- `entities.playersById`
- `entities.playersByTeamId`
- `ui.dashboard`
- `mailbox`
- `fixtures`
- `manager`
- `finances`
- `liveMatch`

#### Beneficio
Menos rerenders y selección más fina con Zustand.

### 3. Introducir normalización + memo de dominio

Centralizar selectores como:
- `selectMyTeam(gameState)`
- `selectRosterByTeamId(gameState, teamId)`
- `selectUnreadMessagesCount(gameState)`
- `selectDashboardAlerts(gameState)`
- `selectTeamStanding(gameState, teamId)`

En vez de repetir `.find/.filter/.sort` por componente.

### 4. Mover histórico a consultas dirigidas

#### Problema
`StatsState` completo en memoria + filtrado local.

#### Cambio propuesto
Para perfiles e históricos:
- consultar SQLite por `player_id`, `team_id`, `limit`, `season`.
- dejar `StatsState` en memoria solo si realmente es crítico para simulación viva.

#### Beneficio
Escala mejor con saves largos.

### 5. Persistencia incremental

#### Problema
`write_game()` y `replace_stats_state()` son demasiado gruesos.

#### Cambio propuesto
- dirty flags por dominio (`messages_dirty`, `players_dirty`, `league_dirty`, etc.)
- batch writes en transacción
- upserts selectivos
- append-only para stats históricas

---

## Mejoras avanzadas de largo plazo

1. **Snapshot diffs para live match**
   - Enviar solo cambios de score, minute, events, substitutions, possession, etc.

2. **Hydration progresiva del Dashboard**
   - cargar primero shell + resumen mínimo.
   - luego inbox, standings, scouting, history bajo demanda.

3. **Worker/web worker para derivadas frontend pesadas**
   - útil si el mundo crece mucho y ciertas agregaciones siguen del lado frontend.

4. **Perf budgets automáticos**
   - tamaño máximo por chunk.
   - tiempo máximo de `get_active_game`.
   - tiempo máximo de `save_game`.

5. **Benchmarks Rust reproducibles**
   - turn processing.
   - save/load.
   - live match stepping.
   - build summary / stats queries.

---

## Estrategia de profiling y benchmarks antes de tocar nada

### Frontend

#### Instrumentación mínima a añadir
1. `performance.mark()` / `performance.measure()` en:
   - `main.tsx` inicio bootstrap
   - `App.tsx` antes/después de `loadSettings`
   - `MainMenu.tsx` antes/después de `start_new_game` y `load_game`
   - `Dashboard.tsx` antes/después de `get_active_game`
   - `TeamSelection.tsx` antes/después de `get_team_selection_data`
   - `MatchLive.tsx` step cycle

2. React Profiler en:
   - `Dashboard`
   - `DashboardWorkspaceContent`
   - `DashboardTabContent`
   - `HomeTab`
   - `PlayersListTab`
   - `InboxTab`
   - `MatchLive`

3. Contadores de rerender por componente crítico en entorno dev.

#### Qué medir
- `app_boot_to_first_route_ms`
- `settings_load_ms`
- `dashboard_data_fetch_ms`
- `dashboard_first_interactive_ms`
- `team_selection_recovery_ms`
- `player_profile_history_fetch_ms`
- `team_profile_stats_fetch_ms`
- `match_step_cycle_ms`
- `match_snapshot_parse_ms`

### Backend Rust

#### Instrumentación mínima
Medir con logs temporales o spans en:
- `start_new_game`
- `select_team`
- `load_game`
- `get_active_game`
- `save_game`
- `advance_time_with_mode`
- `skip_to_match_day`
- `step_live_match`
- `finish_live_match`
- `get_player_match_history`
- `get_team_match_history`

#### Submedidas recomendadas
- lock acquire time
- clone time de `Game`
- serialize time
- DB open+migrations time
- write_game total time
- replace_stats_state total time
- query/read stats time

### Bundle / assets

1. generar reporte de chunks de Vite en modo análisis.
2. registrar tamaño gzip/brotli por chunk.
3. listar top 20 assets servidos más pesados.
4. registrar número de requests remotos por match/profile.

---

## Métricas objetivo y cómo medirlas

## Objetivos de producto/técnicos

### Cold start
- **Objetivo inicial**: abrir ventana y pintar shell en **< 1200 ms** en máquina dev media.
- **Objetivo deseable**: dashboard interactivo tras cargar save reciente en **< 2000 ms**.

### TTI Dashboard
- `get_active_game` end-to-end: **< 200 ms** ideal / **< 400 ms** aceptable.
- commit render principal Dashboard: **< 120 ms**.

### IPC
- comandos frecuentes de UI (`mark_message_read`, `toggle_transfer_list`, `set_training`): **payload < 50 KB**.
- `get_active_game`: reducir progresivamente hasta **< 250 KB** serializado o dividirlo.
- live match step loop: **1 roundtrip por tick, no 2**.

### Save/Load
- `save_game`: **< 300 ms** ideal / **< 700 ms** aceptable.
- `load_game`: **< 500 ms** ideal / **< 1200 ms** aceptable.

### Render
- ninguna interacción simple de dashboard/tab debe provocar más de **1 commit pesado > 50 ms**.
- búsqueda en players/inbox con dataset actual: respuesta visual **< 50 ms**.

### Memoria
- reducir duplicación de payloads grandes y evitar picos por snapshots completos.
- monitorizar heap del WebView antes/después de `get_active_game`, abrir perfil y match live.

## Cómo medir

- **Frontend**: `performance.measure`, React Profiler, devtools Performance.
- **Tauri/Rust**: logs temporales con timestamps por comando.
- **Bundle**: reporte de Rollup/Vite.
- **Assets**: inventario de pesos y formatos.
- **Persistencia**: benchmark sintético con save pequeño / mediano / grande.

---

## Checklist accionable por fases

## Fase 0 — Baseline
- [ ] Añadir marks/measure en bootstrap, dashboard, team selection, profiles, live match.
- [ ] Medir tamaño serializado de `Game` en `get_active_game` y comandos frecuentes.
- [ ] Medir tiempo de `save_game` y `load_game` con logs.
- [ ] Sacar reporte de chunks Vite y top assets.
- [ ] Registrar rerenders de `Dashboard`, `HomeTab`, `PlayersListTab`, `InboxTab`, `MatchLive`.

## Fase 1 — Quick wins
- [ ] Lazy-load de idiomas en vez de importar todos al arranque.
- [ ] Pasar JSONs de draft a carga diferida por feature.
- [ ] Corregir `useMemo(() => setPage(1))` en `PlayersListTab`.
- [ ] Memoizar `dashboardHelpers` y derivados críticos.
- [ ] Precomputar mapas `playersByTeamId`, `teamsById`, `standingsByTeamId` en tabs con tablas/listas.
- [ ] Comprimir/convertir fotos PNG pesadas y revisar `openfootlogo.svg`.
- [ ] Sustituir assets remotos críticos por assets locales o cacheados.

## Fase 2 — IPC y estado
- [ ] Diseñar DTOs parciales por dominio.
- [ ] Reducir comandos que devuelven `Game` completo.
- [ ] Introducir actualizaciones parciales/patches en store.
- [ ] Separar store de juego en slices/entidades/selectores.
- [ ] Implementar un comando combinado para live match step + snapshot.

## Fase 3 — Persistencia e histórico
- [ ] Dejar de hacer `DELETE + INSERT` completo para stats.
- [ ] Evaluar append-only para histórico de matches.
- [ ] Consultar históricos desde SQLite por filtros en vez de recorrer `StatsState` entero.
- [ ] Añadir transacciones explícitas donde falten y medir impacto.

## Fase 4 — Escalado serio
- [ ] Virtualizar listas con potencial de cientos/miles de filas.
- [ ] Hydration progresiva del dashboard.
- [ ] Snapshot diffs para live match.
- [ ] Benchmarks automáticos Rust + budgets en CI.

---

## Recomendación principal

La secuencia correcta NO es "vamos optimizando cosas". La secuencia correcta es:

1. **Medir** `get_active_game`, `save_game`, `load_game`, `step_live_match`, bundle y rerenders.
2. **Quitar peso del arranque**: i18n + JSONs + assets.
3. **Reducir payloads IPC** y eliminar respuestas `Game` completas donde no hagan falta.
4. **Reestructurar store/selectores** para cortar rerenders masivos.
5. **Replantear persistencia/histórico** para saves largos.

Ese es el orden que de verdad cambia el producto. Lo demás, sinceramente, sería maquillaje técnico.
