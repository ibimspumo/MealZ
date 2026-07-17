# MealZ architecture

## Runtime overview

```text
React 19 UI
   │ invoke / Tauri events
   ▼
Tauri 2 command layer
   ├── MealzStore ── rusqlite ── ~/Library/Application Support/de.agentz.mealz/mealz.sqlite3
   └── MealzAgent
          │ JSONL over ordered stdio
          ▼
      codex app-server
          ├── persistent main thread
          ├── streamed server notifications
          ├── built-in web research
          └── item/tool/call ── validated dynamic tools ── MealzStore
```

Codex App Server is the only agent runtime. MealZ intentionally does not expose a provider interface or fallback model path.

## Source boundaries

- `src/`: React product UI, typed bridge, in-memory browser development adapter and frontend state.
- `src-tauri/src/domain/`: durable MealZ models, schema/migrations, deterministic calculations and structured tool implementations.
- `src-tauri/src/codex/`: generic App Server protocol, process host, streaming runtime and dynamic-tool dispatch.
- `src-tauri/src/lib.rs`: integration boundary, Tauri commands/events and lifecycle.
- `~/Library/Application Support/de.agentz.mealz/PERSONA.md`: user-editable agent identity, tone and language rules.
- `~/Library/Application Support/de.agentz.mealz/MEMORY.md`: user- and agent-editable narrative long-term context alongside structured memories.
- `docs/codex-protocol/schema/`: frozen schemas generated from the locally installed Codex CLI with experimental APIs enabled.

## Source-of-truth rules

- The database owns recipes, preferences, memories, planned meals, shopping state, ratings and agent transcript metadata.
- Shopping totals are rebuilt from structured recipe ingredients and the selected plan range. The final chat message is never parsed to obtain quantities.
- Nutrition totals are sums of structured ingredient or recipe nutrition values. Unverified data carries source/confidence metadata.
- The agent may propose or perform actions only through registered dynamic tools. Each mutating tool validates its payload and records a change set suitable for undo.
- User-authored hard constraints outrank inferred memories. Memory entries retain kind, confidence, evidence and status and remain editable in the UI.
- The first-run state is stored in `app_meta.onboarding_complete`. The deterministic wizard or the agent's `onboarding_complete` dynamic tool may set it only after the profile has been persisted.
- Mila never changes `PERSONA.md` autonomously. Dedicated tools may read it and may read or update `MEMORY.md`; all structured preference learning still uses the validated memory tools.

## Codex session lifecycle

1. Resolve the Codex executable, preferring the configured absolute path and then the user's normal installation.
2. Spawn `codex app-server` and send `initialize` with `experimentalApi: true`, followed by `initialized`.
3. Resume the persisted thread when possible. If the rollout is unavailable, create a new persistent thread with MealZ developer instructions and dynamic tool definitions.
4. Start a turn with a structured user input and current curated profile/memory context.
5. Forward message deltas, item state, tool activity, errors and turn completion to React as `agent:event`.
6. Execute `item/tool/call` through the domain dispatcher and return a `DynamicToolCallResponse` containing structured JSON as `inputText`.
7. Shut down gracefully with the application; lazily respawn after an unexpected process exit.

## Security posture

MealZ is a personal local application. The Codex thread receives no general-purpose MealZ database credentials or raw SQL capability. Domain mutation occurs only through the explicit tool registry. The runtime working directory is the application data directory, and the product instructions constrain the agent to food and meal planning.

## Updating the protocol snapshot

```sh
rm -rf docs/codex-protocol/schema
mkdir -p docs/codex-protocol/schema
codex app-server generate-json-schema --out docs/codex-protocol/schema --experimental
```

After regenerating, run Rust protocol tests and a real initialize/thread/turn smoke test before accepting a Codex CLI update.
