# Análisis del proyecto Tauri: OLManager

## 1. Resumen ejecutivo del proyecto

Este repositorio implementa una **aplicación de escritorio con Tauri** cuyo frontend está hecho con **React + TypeScript + Vite** y cuyo backend está hecho con **Rust**.

La idea base del producto es la de un **manager deportivo**, pero el estado actual del código muestra algo importante:

- la base histórica del proyecto sigue muy ligada a **OpenFoot Manager** y al mundo del **fútbol**;
- la implementación actual ya empuja con fuerza hacia un **manager de esports / League of Legends / LEC**;
- por eso conviven nombres, conceptos y documentos de ambas etapas.

En la práctica, hoy el repo parece ser un **producto en transición**: reutiliza estructuras del football manager clásico, pero las está adaptando a un contexto de **LoL competitivo**, draft de campeones, roles como TOP/JUNGLE/MID/ADC/SUPPORT y mundo LEC por defecto.

---

## 2. Qué es Tauri y cómo aplica en este repo

### Qué es Tauri

Tauri es un framework para construir aplicaciones de escritorio con una idea muy simple:

- la **interfaz** se hace como una app web moderna;
- el **contenedor de escritorio** y la lógica nativa se hacen en **Rust**;
- ambas partes se comunican por un puente de comandos.

Dicho de forma pedagógica:

- **React/Vite** pinta pantallas, botones, formularios y navegación;
- **Tauri** empaqueta eso como app de escritorio real;
- **Rust** ejecuta la lógica pesada, persistencia, simulación y acceso al sistema.

### Cómo aplica aquí

En este proyecto Tauri no es un detalle cosmético: es la **columna vertebral de la app desktop**.

- `src/` contiene la aplicación React.
- `src-tauri/` contiene la app Rust/Tauri.
- el frontend llama al backend con `invoke("nombre_del_comando")`.
- el backend registra esos comandos en `src-tauri/src/lib.rs`.

Ejemplo real del repo:

```ts
const state = await invoke<GameStateData>("get_active_game");
```

```rust
.invoke_handler(tauri::generate_handler![
    get_active_game,
    start_new_game,
    select_team,
    advance_time_with_mode,
    start_live_match,
    finish_live_match,
    get_settings,
    save_settings,
])
```

---

## 3. Estructura de carpetas explicada

## `src/`

Es el **frontend**.

Aquí viven:

- el arranque React (`main.tsx`);
- el router (`App.tsx`);
- páginas (`pages/`);
- componentes (`components/`);
- stores globales con Zustand (`store/`);
- utilidades (`lib/`);
- contexto de tema (`context/`);
- i18n (`i18n/`).

Qué se ve en la práctica:

- `pages/MainMenu.tsx` inicia partida y carga saves;
- `pages/TeamSelection.tsx` deja elegir equipo;
- `pages/Dashboard.tsx` es el shell principal del juego;
- `pages/MatchSimulation.tsx` orquesta el flujo de partida/matchday;
- muchos componentes ya están pensados para **LoL/LEC**.

## `src-tauri/`

Es el **backend + shell desktop**.

Aquí viven:

- configuración Tauri (`tauri.conf.json`);
- entrada Rust (`src/main.rs`, `src/lib.rs`);
- comandos Tauri (`src/commands/`);
- lógica de aplicación (`src/application/`);
- crates del dominio (`crates/domain`, `crates/engine`, `crates/ofm_core`, `crates/db`);
- base de datos de mundo por defecto (`databases/lec_world.json`);
- datos base (`data/`);
- iconos del bundle (`icons/`).

Es la parte que hace de:

- motor de simulación;
- gestor de estado global del juego;
- persistencia de partidas;
- configuración de escritorio;
- capa IPC para el frontend.

## `public/`

Contiene **assets estáticos servidos al frontend**.

Ejemplos claros:

- logos de equipos;
- fotos de jugadores;
- SVGs de marca;
- recursos visuales usados directamente en React vía rutas como `/team-logos/...`.

Regla mental útil:

- si algo es solo visual y el frontend lo referencia por URL, suele vivir aquí.

## `data/`

Contiene **datos del mundo y seeds de contenido**, especialmente de la parte LEC/LoL.

Ejemplos detectados:

- `data/lec/seed.teams-players.json`
- `data/lec/player-overrides.json`
- `data/lec/draft/champions.json`
- `data/lec/draft/teams.json`
- `data/lec/draft/players.json`
- imágenes asociadas al dataset

Es una carpeta de **contenido de negocio / seed data**, no de UI.

## `scripts/`

Contiene scripts auxiliares para **generar o enriquecer datos**.

Ejemplos:

- `generate-lec-world.mjs`: genera el mundo LEC que acaba en `src-tauri/databases/lec_world.json`;
- `fetch-leaguepedia-dobs.mjs`: extrae fechas de nacimiento desde fuentes externas para enriquecer datos.

Esta carpeta sirve para el trabajo de preparación del contenido, no para la ejecución normal del usuario final.

## `docs/`

Contiene documentación funcional y técnica.

PERO aquí hay una observación clave:

- gran parte de la documentación sigue describiendo el proyecto como **football manager / OpenFoot Manager**;
- el código actual ya muestra una transición fuerte a **Open League Manager / LEC / esports**.

O sea: la carpeta sigue siendo valiosa, pero **hay desalineación entre docs y realidad actual**.

---

## 4. Arquitectura general frontend/backend

La arquitectura se puede entender así:

```text
React + TypeScript (src/)
        │
        │ invoke()
        ▼
Tauri commands (src-tauri/src/commands)
        │
        ▼
Application layer (src-tauri/src/application)
        │
        ▼
Core/domain/engine/db crates (Rust)
        │
        ▼
Estado, simulación, guardado, settings, logs
```

### Frontend

Responsabilidades:

- renderizar pantallas;
- manejar navegación;
- recoger interacción del usuario;
- mostrar estado actual del juego;
- lanzar comandos al backend.

### Backend

Responsabilidades:

- crear y mantener el estado activo del juego;
- simular avance temporal y partidos;
- guardar/cargar partidas;
- gestionar settings;
- exponer comandos seguros al frontend.

### Estado

Hay dos estados bien distintos:

1. **Estado UI/frontend**
   - Zustand (`gameStore`, `settingsStore`)
   - cachea y propaga datos para React

2. **Estado real del juego**
   - Rust `StateManager`
   - vive en el backend y es la fuente de verdad

Esto es importante: el frontend **no es el dueño final del juego**, solo consume y refleja el estado que controla Rust.

---

## 5. Flujo de arranque de la app

## Arranque técnico

### Frontend

`src/main.tsx` hace el bootstrap:

```tsx
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <App />
    </ThemeProvider>
  </React.StrictMode>,
);
```

Luego `src/App.tsx`:

- carga settings si hace falta;
- aplica idioma, escala UI y alto contraste;
- monta `BrowserRouter`;
- expone rutas principales:
  - `/`
  - `/select-team`
  - `/dashboard`
  - `/match`
  - `/settings`

### Backend

`src-tauri/src/main.rs` delega en:

```rust
openfootmanager_lib::run()
```

Y `src-tauri/src/lib.rs`:

- crea el `tauri::Builder`;
- registra plugins (`opener`, `log`);
- crea y registra `StateManager`;
- inicializa el sistema de saves en `setup()`;
- migra saves legacy si encuentra una base antigua;
- registra todos los comandos disponibles para `invoke()`.

## Arranque funcional

Desde la perspectiva del usuario:

1. se abre el menú principal;
2. se puede iniciar nueva partida o cargar save;
3. al crear partida, `start_new_game` genera/carga el mundo;
4. después se elige equipo con `select_team`;
5. al entrar a dashboard, el frontend pide `get_active_game` y sincroniza el store.

---

## 6. Comunicación frontend ↔ backend con `invoke()` y comandos Tauri

Este es EL concepto central para entender Tauri en este repo.

## En frontend

Se usa:

```ts
import { invoke } from "@tauri-apps/api/core";
```

Ejemplos reales:

```ts
await invoke("start_new_game", {
  nickname,
  firstName,
  lastName,
  dob,
  nationality,
  worldSource,
});
```

```ts
const updatedGame = await invoke<GameStateData>("select_team", { teamId });
```

```ts
const snap = await invoke<MatchSnapshot>("get_match_snapshot");
```

## En backend

Cada comando se expone con `#[tauri::command]`:

```rust
#[tauri::command]
pub async fn select_team(
    state: State<'_, StateManager>,
    sm_state: State<'_, SaveManagerState>,
    team_id: String,
) -> Result<Game, String> {
    ...
}
```

Y luego se registra en el builder.

## Patrón mental correcto

Piensa `invoke()` como una **llamada RPC local**:

- React pide una acción;
- Rust la ejecuta;
- Rust devuelve datos serializados;
- React actualiza la UI.

## Dónde se ve más claro en este proyecto

- `MainMenu.tsx`: nueva partida, cargar saves
- `TeamSelection.tsx`: selección de equipo
- `Dashboard.tsx`: obtener estado, guardar, salir
- `settingsStore.ts`: cargar/guardar settings
- `services/*.ts`: wrappers para comandos Tauri
- `MatchSimulation.tsx`: flujo de partido en vivo

---

## 7. Funcionalidades principales detectadas

Por lo que se ve en código y comandos, las capacidades principales son:

- **crear nueva partida**;
- **cargar y borrar saves**;
- **seleccionar equipo**;
- **gestionar dashboard principal**;
- **avanzar el tiempo** con distintos modos;
- **simulación de partido en vivo**;
- **modo espectador / delegado**;
- **draft de campeones** para contexto LoL;
- **gestión táctica**;
- **gestión de training**;
- **gestión de staff**;
- **mensajes/inbox**;
- **finanzas**;
- **mercado de fichajes/negociaciones**;
- **scouting**;
- **estadísticas de jugador y equipo**;
- **configuración persistente**;
- **exportación/importación de mundos**.

Señales claras del giro a esports/LoL/LEC:

- mundo por defecto `lec-default`;
- base `src-tauri/databases/lec_world.json`;
- roles `TOP/JUNGLE/MID/ADC/SUPPORT`;
- `ChampionDraft.tsx`;
- `record_fixture_champion_picks` en Rust;
- recursos visuales y datasets centrados en LEC.

---

## 8. Dependencias y tecnologías clave

## Frontend

- **React 19**
- **TypeScript**
- **Vite**
- **React Router**
- **Zustand**
- **i18next + react-i18next**
- **Tailwind CSS v4**
- **lucide-react**
- **Vitest + Testing Library + jsdom**

## Desktop / backend

- **Tauri v2**
- **@tauri-apps/api**
- **@tauri-apps/cli**
- **Rust**
- **serde / serde_json**
- **chrono**
- **log**
- **tauri-plugin-log**
- **tauri-plugin-opener**

## Organización Rust interna

- `domain`: tipos de dominio
- `engine`: simulación
- `ofm_core`: lógica principal del juego
- `db`: persistencia

---

## 9. Patrones/convenciones del proyecto

## a) Separación clara UI vs lógica de negocio

El frontend no mete la simulación compleja dentro de React. Eso vive en Rust. BIEN.

## b) `invoke()` como frontera formal

La frontera frontend/backend está bastante clara:

- React pide;
- Rust decide;
- React representa.

## c) Stores pequeños con Zustand

Se usan stores concretos:

- `gameStore`
- `settingsStore`

No parece haber una sobreingeniería rara aquí.

## d) Servicios/gateways en frontend

Hay wrappers como:

- `advanceTimeService.ts`
- `staffService.ts`
- `jobService.ts`
- `TeamProfile.gateway.ts`

Eso ayuda a no llenar todos los componentes con `invoke()` directo.

## e) Comandos agrupados por dominio en Rust

`src-tauri/src/commands/` está separado por áreas:

- `game.rs`
- `time.rs`
- `live_match.rs`
- `settings.rs`
- `staff.rs`
- `transfers.rs`
- `stats/`

Buena señal: la API de escritorio está modularizada.

## f) Lazy loading de páginas

`App.tsx` hace `lazy()` para páginas grandes, lo que reduce coste inicial del bundle frontend.

## g) Mezcla de naming heredado y naming nuevo

Aquí hay una convención de hecho, pero también un problema:

- nombre histórico: `openfootmanager`, `OpenFoot Manager`
- nombre visible nuevo: `Open League Manager`
- dominio funcional actual: LEC / LoL / esports

No es solo branding; afecta comprensión arquitectónica.

---

## 10. Riesgos / deuda técnica

## 1. Desalineación entre documentación e implementación

Es probablemente la deuda más visible.

Ejemplos:

- docs hablan de football manager clásico;
- el código ya usa LEC/LoL/draft/champions;
- en `docs/ARCHITECTURE.md` aparece ruta `/team-selection`, pero en `App.tsx` la ruta real es `/select-team`.

## 2. Naming inconsistente

Conviven:

- `openfootmanager`
- `OpenFoot Manager`
- `Open League Manager`
- assets `openfootball.svg`
- ventana Tauri con título `Open League Manager pre-alpha version`

Esto aumenta la fricción para cualquiera nuevo en el repo.

## 3. Modelo de dominio todavía arrastra semántica de fútbol

Se ven traducciones como:

- roles de LoL mapeados sobre posiciones heredadas de fútbol;
- conceptos como stadium/country/formation/documentación futbolera siguen presentes.

Eso puede ser una estrategia temporal válida, PERO introduce deuda conceptual.

## 4. Datos y pipelines todavía algo artesanales

Ejemplo llamativo:

- `scripts/generate-lec-world.mjs` referencia `Nueva carpeta/players.json`.

Eso huele a pipeline de datos todavía no estabilizado.

## 5. Duplicación o dispersión de fuentes de datos

Hay datos en:

- `data/`
- `public/`
- `src-tauri/data/`
- `src-tauri/databases/`

No necesariamente está mal, pero exige entender muy bien qué es:

- seed de generación,
- asset visual,
- recurso empaquetado,
- base final consumida por backend.

## 6. Complejidad creciente en pantallas grandes

Hay componentes muy voluminosos, por ejemplo `ChampionDraft.tsx`. Eso puede volver más difícil mantenerlos a medio plazo.

---

## 11. Diferencia entre partes “web normales” y partes “propias de Tauri”

## Partes “web normales”

Estas podrían existir casi igual en una SPA convencional:

- React components
- routing con React Router
- Zustand
- i18n
- Tailwind
- assets servidos desde `public/`
- tests con Vitest y Testing Library

Ejemplos de archivos “web”:

- `src/App.tsx`
- `src/pages/*`
- `src/components/*`
- `src/store/*`
- `src/i18n/*`

## Partes propias de Tauri

Estas son las que convierten la SPA en aplicación desktop real:

- `src-tauri/tauri.conf.json`
- `src-tauri/src/lib.rs`
- `src-tauri/src/main.rs`
- `#[tauri::command]`
- `invoke()` desde frontend
- plugins Tauri
- acceso a `app_data_dir`, logs, saves locales
- bundle nativo, iconos, ventana desktop

Ejemplos claros:

```rust
app.path().app_data_dir()
```

```ts
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
```

Si quitas Tauri, te quedas con una web. Si quitas React, te quedas con un backend nativo sin interfaz usable. Ambos se necesitan.

---

## 12. Guía práctica: “si quieres tocar X, mira Y”

## Si quieres tocar el arranque de la app

Mira:

- `src/main.tsx`
- `src/App.tsx`
- `src-tauri/src/main.rs`
- `src-tauri/src/lib.rs`

## Si quieres tocar navegación o pantallas principales

Mira:

- `src/App.tsx`
- `src/pages/MainMenu.tsx`
- `src/pages/TeamSelection.tsx`
- `src/pages/Dashboard.tsx`
- `src/pages/MatchSimulation.tsx`

## Si quieres tocar settings

Mira:

- `src/store/settingsStore.ts`
- `src/pages/Settings.tsx`
- `src-tauri/src/commands/settings.rs`

## Si quieres tocar creación de partida / selección de equipo

Mira:

- `src/pages/MainMenu.tsx`
- `src/pages/TeamSelection.tsx`
- `src-tauri/src/commands/game.rs`
- `src-tauri/src/commands/world.rs`

## Si quieres tocar avance temporal y flujo del dashboard

Mira:

- `src/pages/Dashboard.tsx`
- `src/hooks/useAdvanceTime.ts`
- `src/services/advanceTimeService.ts`
- `src-tauri/src/commands/time.rs`
- `src-tauri/src/application/time_advancement.rs`

## Si quieres tocar partido en vivo

Mira:

- `src/pages/MatchSimulation.tsx`
- `src/components/match/PreMatchSetup.tsx`
- `src/components/match/MatchLive.tsx`
- `src/components/match/HalfTimeBreak.tsx`
- `src/components/match/PostMatchScreen.tsx`
- `src/components/match/PressConference.tsx`
- `src-tauri/src/commands/live_match.rs`

## Si quieres tocar draft / LoL / picks de campeones

Mira:

- `src/components/match/ChampionDraft.tsx`
- `data/lec/draft/*`
- `src-tauri/src/commands/live_match.rs` (`record_fixture_champion_picks`)
- `scripts/generate-lec-world.mjs`

## Si quieres tocar datos del mundo LEC

Mira:

- `data/lec/*`
- `scripts/generate-lec-world.mjs`
- `src-tauri/databases/lec_world.json`
- `src-tauri/src/commands/game.rs`

## Si quieres tocar guardado/carga de partidas

Mira:

- `src/components/menu/SavesList.tsx`
- `src/pages/MainMenu.tsx`
- `src-tauri/src/commands/game.rs`
- `src-tauri/crates/db/*`

## Si quieres tocar stats o vistas de perfil

Mira:

- `src/components/teamProfile/*`
- `src/components/playerProfile/*`
- `src/components/teamProfile/TeamProfile.gateway.ts`
- `src-tauri/src/commands/stats/*`

---

## Observaciones importantes

## El proyecto está en transición

La evidencia del repo apunta a que el proyecto está migrando desde una base conceptual y técnica de **football manager** hacia un **manager de esports / LoL / LEC**.

No es una hipótesis gratuita; se ve en:

- documentación todavía futbolera;
- package/product naming histórico `openfootmanager`;
- strings visibles de UI como `Open League Manager`;
- datasets y mundo por defecto centrados en LEC;
- draft de campeones y roles propios de LoL.

## Documentación y naming están desalineados

Esto también es un hallazgo central.

Ejemplos concretos:

- docs técnicas describen fútbol donde el código ya implementa LoL/LEC;
- naming interno y visible no está unificado;
- algunas rutas y conceptos documentados no coinciden exactamente con el código actual.

Para alguien que ve Tauri por primera vez, esto importa mucho porque puede confundir sobre qué parte es arquitectura real y qué parte es herencia histórica.

---

## Conclusión

Si tuviera que explicarlo en una frase:

> Este repo es una app desktop con Tauri donde **React pinta la experiencia** y **Rust gobierna el juego**, pero además está atravesando una **migración de identidad y dominio** desde football manager hacia un manager de League of Legends / LEC.

Y ese detalle NO es menor, porque condiciona casi todo lo que ves: estructura, naming, deuda técnica y cómo deberías leer el código.
