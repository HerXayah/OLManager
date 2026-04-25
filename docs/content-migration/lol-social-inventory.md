# LoL Social/Media Narrative Inventory

Scope for `lol-social-media-rework`: press conference UI, backend media/fan/player narrative events, backend news/social copy, and locale strings that currently expose football-era framing in those surfaces.

## Press conference UI

| File | Football-era copy/contract | Migration target |
|---|---|---|
| `src/components/match/PressConference.tsx` | Hardcoded journalist/outlet names (`David Thomson`, `Sports Daily`, `Rachel Cooper`, `Match Day Live`, `Sarah Mitchell`, `Football Weekly`, `Supporters' Voice`, `The Athletic`) | Replace with JSON personas/outlets using real/fictional/inspired type, league scope, allowed tones, and weights. |
| `src/components/match/PressConference.tsx` | Hardcoded question categories (`result`, `player_focus`, `tactics`, `fans`, `ahead`) generated from football score/events | Replace with registry questions selected only after match-context tags satisfy required/excluded preconditions. |
| `src/components/match/PressConference.tsx` | Answer payload sends localized `response_tone` as the backend gameplay coupling | Preserve morale/player effects by sending stable `effect_id` while keeping display label/copy for UI/news. |

## Backend random events

| File | Football-era copy/contract | Migration target |
|---|---|---|
| `src-tauri/crates/ofm_core/src/random_events/message_builders.rs` | `media_story_message` uses local papers/tabloids/sports journalists/player form framing | Resolve LoL media templates from content with player/team params and stable effect IDs. |
| `src-tauri/crates/ofm_core/src/random_events/message_builders.rs` | `community_event_message` mentions open days, training sessions, PR, and team spirit | Migrate to fan/community esports activities while preserving existing morale trigger/effect behavior. |
| `src-tauri/crates/ofm_core/src/random_events/responses.rs` | Action responses preserve morale/finance outcomes for random events | Map migrated content to existing response IDs/effects until backend narrative selectors take ownership. |

## Backend player events

| File | Football-era copy/contract | Migration target |
|---|---|---|
| `src-tauri/crates/ofm_core/src/player_events/message_builders.rs` | Low morale, bench complaint, happy player, and contract concern messages reference football, pitch, game time, dressing room, and manager-office framing | Resolve LoL player conversation templates with role/scrim/stage/mental-reset framing and stable effect IDs. |
| `src-tauri/crates/ofm_core/src/player_events/responses.rs` | Existing response IDs drive morale/promise/contract effects | Preserve IDs/effects during copy migration; introduce content `effectId` aliases before removing old coupling. |

## Backend news/social articles

| File | Football-era copy/contract | Migration target |
|---|---|---|
| `src-tauri/crates/ofm_core/src/news.rs` | League roundup/standings/season preview copy uses Matchday, goals, Premier Division, Football Herald, and sports-gazette framing | Migrate scoped social/media article copy to LoL series/maps/objectives/league-play framing. |
| `src-tauri/crates/ofm_core/src/news.rs` | Match report and transfer/injury/editorial article copy may still expose football stats | Keep unrelated domain/stat fields until later slices; only social/media narrative is in this change scope. |

## Locale files

| File(s) | Football-era copy/contract | Migration target |
|---|---|---|
| `src/i18n/locales/*.json` | `match.press.*` question/response keys contain football-era press text, tone labels, and result framing | Replace or bypass with registry locale keys/copy while preserving fallback keys during rollout. |
| `src/i18n/locales/*.json` | `match.pressReport.*` uses press-conference and post-match football wording | Migrate report copy to LoL post-series media scrum framing. |
| `src/i18n/locales/*.json` | `be.msg.mediaPositive`, `be.msg.mediaNegative`, fan petition, player-event strings, and `be.source.footballHerald` contain scoped media/player football terms | Migrate keys used by backend social/media slices after JSON registry loaders/selectors exist. |

## Initial allowlist / out-of-scope terms

- Domain fields and legacy stat names such as `goals`, `Goalkeeper`, `Matchday`, and `GD` may remain outside scoped narrative surfaces until their owning systems migrate.
- Comments that document current legacy behavior may remain during rollout if they are not user-visible copy.
