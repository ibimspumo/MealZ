# MealZ engineering guide

MealZ is a personal, local-first Tauri 2 meal-planning application for Timo. It is open source, but the product decisions optimize for one macOS user rather than a multi-tenant SaaS.

## Non-negotiable architecture

- The only AI/agent runtime is `codex app-server` over its JSONL stdio protocol.
- Do not add OpenAI Responses API, Agents SDK, another provider, or a provider abstraction.
- Adapt the proven host/protocol pattern from `/Users/timocorvinus/Desktop/Code/SwarmZ`.
- MealZ capabilities are Codex App Server dynamic tools. Domain writes are structured, validated, local SQLite operations rather than parsed chat prose.
- Data is local-first. Recipes, weekly plans, shopping, profile, ratings, memories, agent session metadata, and undo history persist in SQLite.

## Product priorities

1. A polished Monday-to-Sunday planning calendar.
2. A deterministic, range-aware shopping list aggregated from planned recipes.
3. A personal meal-planning agent with visible memory, personality, web research, and structured tool calls.
4. A durable recipe catalog with ratings, comments, history, reuse, edit, and regeneration flows.

## Stack and quality

- React 19 + TypeScript + Vite; Tauri 2 + Rust.
- Use strict types and semantic, keyboard-accessible UI.
- Visual language: calm editorial utility, warm near-white surfaces, deep ink, moss green accent, restrained coral warning; no glassmorphism or decorative gradients.
- Verify TypeScript, frontend tests/build, Rust tests/clippy, and a real native app session.
- Preserve unrelated user files. Do not commit or push unless explicitly asked.

## File ownership during parallel work

Agents must edit only the paths assigned in their task. The root orchestrator owns manifests, `src-tauri/src/lib.rs`, final integration, icon assets, testing, and release documentation.
