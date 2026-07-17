# MealZ product context

MealZ is Timo's personal, local-first nutrition and meal-planning desktop application. It will be open source, but every product decision optimizes for one primary macOS user.

## Core outcome

Timo can ask a personal agent for the next Monday-to-Sunday plan, edit or regenerate individual meals, save good recipes permanently, see exact cooking instructions and nutrition, and receive one deterministic, aggregated shopping list. Over time the app learns what he likes, dislikes, avoids, owns, cooks, repeats and rates.

## Primary product pillars

1. **Weekly calendar:** Monday through Sunday, usually one different main meal per day, daily cooking by default, with optional meal-prep reuse when ingredients and methods overlap. Weekdays should favor quicker evening cooking; weekends may be more involved. Eating out or taking food away is an editable calendar state.
2. **Shopping:** derived from a selected plan date range, unit-aware aggregation, pantry exclusions, manual items, categories, check-off state, and automatic rebuilds after plan changes.
3. **Personal agent:** a warm, opinionated meal-planning companion with a stable personality, visible reasoning progress/tool activity, Markdown, web recipe research, and a transparent editable memory system.
4. **Recipe catalog:** generated, web-researched, or manually edited recipes; ingredients, structured steps, equipment, timing, servings, sources/images, calories/macros/fiber, ratings/comments, last-cooked history, favorites and reuse suggestions.
5. **Personal onboarding:** a guided first run collects the user's name, optional body context, nutrition frame, weekly cooking reality, equipment, first preferences and the agent's name/personality/autonomy. It can be restarted from Settings without deleting recipes or history. A conversational Codex briefing may finish by calling the structured `onboarding_complete` tool.

## Personal defaults

- Focus on lunch, dinner and snacks/protein shakes; Timo normally does not eat breakfast.
- Plan the coming week, Monday through Sunday.
- Primarily home cooking. Workdays are usually time-constrained evenings; weekends have more time.
- Available equipment is editable and initially includes four-zone stove, oven, dual-basket air fryer, blender, contact grill, toaster, microwave with oven/keep-warm function, vegetable chopper and Lidl Monsieur Cuisine/Thermomix-style cooker.
- Preference strength needs more nuance than like/dislike: 1–10 plus free-text context. Example: carrots are acceptable but not preferred.
- Nutrition supports calories, protein, carbohydrates, fat and fiber. Defaults should be sensible and editable, not presented as medical advice. Protein can target roughly 2 g/kg and fiber 30–40 g/day when selected.
- The app is meal planning, not a workout or body-weight logging application.

## Agent architecture — fixed

- The runtime is **exclusively `codex app-server`**. No Responses API, Agents SDK, alternate provider, fallback runtime or provider switch.
- Use one long-lived app-server process over JSONL stdio, adapted from `/Users/timocorvinus/Desktop/Code/SwarmZ`.
- Initialize with experimental APIs, persist and resume the main thread, stream events, and answer server-side `item/tool/call` requests.
- MealZ actions are dynamic tools supplied with `thread/start.dynamicTools`. The agent proposes and performs validated structured changes through tools rather than returning prose that the UI parses.
- Built-in Codex web research is available for recipe discovery and source verification.
- Developer instructions plus curated memory blocks provide personality and user context. Memory can be proposed autonomously, but remains inspectable, editable, dismissible and traceable.
- `PERSONA.md` and `MEMORY.md` live in the local application data directory and are directly editable in Settings. The structured SQLite memory remains authoritative for individual facts and ratings; `MEMORY.md` complements it with narrative context.
- Mila's default persona contains a hard language rule: never emit Em-Dashes or Unicode U+2014.
- The future web/mobile surface remains Codex-only by connecting to a persistent app-server host service; it must not silently replace the runtime with serverless model calls.

## Experience principles

- Calm editorial utility: warm near-white, deep ink, moss-green action color and restrained coral highlights.
- Fast native desktop interactions, clear hierarchy, keyboard support, no glassmorphism or decorative gradient noise.
- AI never overwrites silently. Show what changed, allow undo, and distinguish saved facts from inferred preferences.
- Structured data and deterministic calculations are the source of truth for plans, shopping quantities and nutrition totals.
