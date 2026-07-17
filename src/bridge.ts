import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { createDemoBootstrap } from "./demo-data";
import type {
  AgentEvent,
  AgentCapabilities,
  AgentConversation,
  AgentConversationResult,
  AgentFiles,
  AgentMessage,
  BootstrapData,
  Memory,
  PlanItem,
  Profile,
  Recipe,
  ShoppingItem,
} from "./types";

const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
const wait = (ms = 180) => new Promise((resolve) => window.setTimeout(resolve, ms));
let demo = createDemoBootstrap();
let demoConversations: AgentConversation[] = [{
  id: "demo-conversation-main",
  title: "MealZ Chat",
  status: "active",
  active: true,
  messageCount: demo.messages.length,
  preview: demo.messages[demo.messages.length - 1]?.content,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
}];
let demoAgentFiles: AgentFiles = {
  persona: "# PERSONA.md\n\nDu bist die persönliche Meal-Planning-Begleiterin des Nutzers. Du bist direkt, aufmerksam und pragmatisch.\n\n## Sprachregeln\n\n- Antworte auf Deutsch.\n- Verwende niemals Em-Dashes (Unicode U+2014). Nutze stattdessen Punkte, Kommas oder Doppelpunkte.\n- Erkläre Abwägungen kurz und konkret.\n",
  memory: "# MEMORY.md\n\nLangzeitkontext, der zusätzlich zu den strukturierten Erinnerungen in MealZ an jeden relevanten Turn übergeben wird.\n",
};

const makeId = (prefix: string) => `${prefix}-${crypto.randomUUID()}`;

function normalizeCapabilities(value: unknown): AgentCapabilities {
  if (!value || typeof value !== "object") return { webSearch: "unknown", imageGeneration: "unknown" };
  const root = value as Record<string, unknown>;
  const nested = root.capabilities && typeof root.capabilities === "object" ? root.capabilities as Record<string, unknown> : root;
  const flag = (keys: string[]): boolean | "unknown" => {
    for (const key of keys) if (typeof nested[key] === "boolean") return nested[key] as boolean;
    return "unknown";
  };
  return { webSearch: flag(["webSearch", "web_search"]), imageGeneration: flag(["imageGeneration", "image_generation", "imageGen"]) };
}

function aggregateShopping(plan: PlanItem[], recipes: Recipe[], startDate: string, endDate: string): ShoppingItem[] {
  const items = new Map<string, ShoppingItem>();
  plan
    .filter((entry) => entry.date >= startDate && entry.date <= endDate && entry.status !== "skipped")
    .forEach((entry) => {
      const recipe = entry.recipe ?? recipes.find((candidate) => candidate.id === entry.recipeId);
      if (!recipe) return;
      const factor = entry.servings / recipe.servings;
      recipe.ingredients.forEach((ingredient) => {
        const key = `${ingredient.name.toLocaleLowerCase("de")}::${ingredient.unit}`;
        const existing = items.get(key);
        if (existing) {
          existing.amount += ingredient.amount * factor;
          if (!existing.recipeIds.includes(recipe.id)) existing.recipeIds.push(recipe.id);
        } else {
          items.set(key, {
            id: makeId("shopping"),
            name: ingredient.name,
            amount: ingredient.amount * factor,
            unit: ingredient.unit,
            category: ingredient.category,
            checked: false,
            manual: false,
            recipeIds: [recipe.id],
          });
        }
      });
    });
  return [...items.values()];
}

async function tauriOrDemo<T>(command: string, args: Record<string, unknown>, fallback: () => Promise<T> | T): Promise<T> {
  if (isTauri()) return invoke<T>(command, args);
  await wait();
  return fallback();
}

export const api = {
  isNative: isTauri,
  getBootstrap: () => tauriOrDemo<BootstrapData>("get_bootstrap", {}, () => structuredClone(demo)),
  agentCapabilities: () => tauriOrDemo<unknown>("agent_capabilities", {}, () => ({ webSearch: "unknown", imageGeneration: "unknown" })).then(normalizeCapabilities),
  completeOnboarding: (profile: Profile, briefing?: string) =>
    tauriOrDemo<BootstrapData | null>("complete_onboarding", { profile, briefing }, () => {
      demo.profile = structuredClone(profile);
      demo.onboardingComplete = true;
      if (briefing?.trim()) {
        const timestamp = new Date().toISOString();
        demo.memories.unshift({
          id: makeId("memory"), kind: "preference", title: "Erstes persönliches Briefing",
          content: briefing.trim(), confidence: 1, source: "explicit", active: true,
          createdAt: timestamp, updatedAt: timestamp,
        });
      }
      return structuredClone(demo);
    }),
  restartOnboarding: () =>
    tauriOrDemo<void>("restart_onboarding", {}, () => {
      demo.onboardingComplete = false;
    }),
  getAgentFiles: () => tauriOrDemo<AgentFiles>("get_agent_files", {}, () => structuredClone(demoAgentFiles)),
  saveAgentFiles: (files: AgentFiles) =>
    tauriOrDemo<AgentFiles>("save_agent_files", { files }, () => {
      demoAgentFiles = structuredClone(files);
      return structuredClone(demoAgentFiles);
    }),
  listRecipes: (query = "", tags: string[] = []) =>
    tauriOrDemo<Recipe[]>("list_recipes", { query, tags }, () => {
      const needle = query.toLocaleLowerCase("de").trim();
      return structuredClone(
        demo.recipes.filter((recipe) =>
          (!needle || `${recipe.title} ${recipe.description} ${recipe.tags.join(" ")}`.toLocaleLowerCase("de").includes(needle)) &&
          (!tags.length || tags.every((tag) => recipe.tags.includes(tag))),
        ),
      );
    }),
  saveRecipe: (recipe: Recipe) =>
    tauriOrDemo<Recipe>("save_recipe", { recipe }, () => {
      const next = { ...recipe, id: recipe.id || makeId("recipe"), updatedAt: new Date().toISOString() };
      const index = demo.recipes.findIndex((entry) => entry.id === next.id);
      if (index >= 0) demo.recipes[index] = next;
      else demo.recipes.unshift(next);
      return structuredClone(next);
    }),
  deleteRecipe: (recipeId: string) =>
    tauriOrDemo<void>("delete_recipe", { recipeId }, () => {
      demo.recipes = demo.recipes.filter((recipe) => recipe.id !== recipeId);
      demo.plan = demo.plan.filter((entry) => entry.recipeId !== recipeId);
    }),
  rateRecipe: (recipeId: string, rating: number, comment: string) =>
    tauriOrDemo<Recipe>("rate_recipe", { recipeId, rating, comment }, () => {
      const recipe = demo.recipes.find((entry) => entry.id === recipeId);
      if (!recipe) throw new Error("Rezept nicht gefunden");
      recipe.rating = rating;
      recipe.ratingComment = comment;
      return structuredClone(recipe);
    }),
  getWeekPlan: (startDate: string, endDate?: string) => tauriOrDemo<PlanItem[]>("get_week_plan", { startDate, endDate }, () => structuredClone(demo.plan.filter((item) => item.date >= startDate && (!endDate || item.date <= endDate)))),
  savePlanItem: (item: PlanItem) =>
    tauriOrDemo<PlanItem>("save_plan_item", { item }, () => {
      const next = { ...item, id: item.id || makeId("plan") };
      const index = demo.plan.findIndex((entry) => entry.id === next.id);
      if (index >= 0) demo.plan[index] = next;
      else demo.plan.push(next);
      return structuredClone(next);
    }),
  removePlanItem: (planItemId: string) =>
    tauriOrDemo<void>("remove_plan_item", { planItemId }, () => {
      demo.plan = demo.plan.filter((entry) => entry.id !== planItemId);
    }),
  rebuildShoppingList: (startDate: string, endDate: string) =>
    tauriOrDemo<ShoppingItem[]>("rebuild_shopping_list", { startDate, endDate }, () => {
      const generated = aggregateShopping(demo.plan, demo.recipes, startDate, endDate);
      const manual = demo.shopping.filter((item) => item.manual);
      demo.shopping = [...generated, ...manual];
      return structuredClone(demo.shopping);
    }),
  getShoppingList: (startDate: string, endDate: string) =>
    tauriOrDemo<ShoppingItem[]>("get_shopping_list", { startDate, endDate }, () => structuredClone(demo.shopping)),
  toggleShoppingItem: (itemId: string, checked: boolean) =>
    tauriOrDemo<ShoppingItem>("toggle_shopping_item", { itemId, checked }, () => {
      const item = demo.shopping.find((entry) => entry.id === itemId);
      if (!item) throw new Error("Einkaufsartikel nicht gefunden");
      item.checked = checked;
      return structuredClone(item);
    }),
  addShoppingItem: (item: ShoppingItem, startDate: string, endDate: string) =>
    tauriOrDemo<ShoppingItem>("add_shopping_item", { item, startDate, endDate }, () => {
      const next = { ...item, id: item.id || makeId("shopping"), manual: true };
      demo.shopping.push(next);
      return structuredClone(next);
    }),
  deleteShoppingItem: (itemId: string) =>
    tauriOrDemo<void>("delete_shopping_item", { itemId }, () => {
      demo.shopping = demo.shopping.filter((item) => item.id !== itemId);
    }),
  getProfile: () => tauriOrDemo<Profile>("get_profile", {}, () => structuredClone(demo.profile)),
  saveProfile: (profile: Profile) =>
    tauriOrDemo<Profile>("save_profile", { profile }, () => {
      demo.profile = profile;
      return structuredClone(profile);
    }),
  listMemories: () => tauriOrDemo<Memory[]>("list_memories", {}, () => structuredClone(demo.memories)),
  saveMemory: (memory: Memory) =>
    tauriOrDemo<Memory>("save_memory", { memory }, () => {
      const next = { ...memory, id: memory.id || makeId("memory"), updatedAt: new Date().toISOString() };
      const index = demo.memories.findIndex((entry) => entry.id === next.id);
      if (index >= 0) demo.memories[index] = next;
      else demo.memories.unshift(next);
      return structuredClone(next);
    }),
  deleteMemory: (memoryId: string) =>
    tauriOrDemo<void>("delete_memory", { memoryId }, () => {
      demo.memories = demo.memories.filter((memory) => memory.id !== memoryId);
    }),
  listAgentMessages: () => tauriOrDemo<AgentMessage[]>("list_agent_messages", {}, () => structuredClone(demo.messages)),
  agentSend: (message: string) =>
    tauriOrDemo<AgentMessage | null>("agent_send", { message }, async () => {
      const userMessage: AgentMessage = { id: makeId("message"), role: "user", content: message, createdAt: new Date().toISOString() };
      demo.messages.push(userMessage);
      await wait(450);
      const lower = message.toLocaleLowerCase("de");
      const content = lower.includes("woche")
        ? "Ich habe deine **kommende Woche** geprüft. Unter der Woche halte ich die Gerichte unter 45 Minuten; am Wochenende plane ich etwas mit mehr Ruhe ein. Ich kann die Vorschläge jetzt strukturiert in den Kalender schreiben und danach die Einkaufsliste neu berechnen."
        : lower.includes("lasagne")
          ? "Gute Idee. Deine Lasagne wurde zuletzt vor über drei Wochen gekocht und mit **5 Sternen** bewertet. Ich würde sie für Sonntag einplanen – dann hast du genug Zeit und direkt Reste für Montag."
          : "Verstanden. Ich berücksichtige dabei deine gespeicherten Vorlieben, den aktuellen Wochenplan und deine Nährwertziele. Soll ich den Vorschlag direkt einplanen oder erst als Entwurf zeigen?";
      const assistant: AgentMessage = {
        id: makeId("message"),
        role: "assistant",
        content,
        createdAt: new Date().toISOString(),
        tools: lower.includes("woche")
          ? [
              { id: makeId("tool"), name: "memory_recall", label: "Vorlieben gelesen", status: "success", detail: "4 relevante Erinnerungen" },
              { id: makeId("tool"), name: "plan_get_week", label: "Wochenplan geprüft", status: "success", detail: "5 bestehende Einträge" },
            ]
          : undefined,
      };
      demo.messages.push(assistant);
      return structuredClone(assistant);
    }),
  agentNewThread: () =>
    tauriOrDemo<BootstrapData>("agent_new_thread", {}, () => {
      demo.messages = [];
      return structuredClone(demo);
    }),
  listAgentConversations: () =>
    tauriOrDemo<AgentConversation[]>("list_agent_conversations", {}, () => structuredClone(demoConversations)),
  agentCreateConversation: () =>
    tauriOrDemo<AgentConversationResult>("agent_create_conversation", {}, () => {
      demoConversations = demoConversations.map((conversation) => ({ ...conversation, status: "archived" as const, active: false }));
      const now = new Date().toISOString();
      const conversation: AgentConversation = {
        id: makeId("conversation"), title: "Neues MealZ-Gespräch", status: "active", active: true,
        messageCount: 0, createdAt: now, updatedAt: now,
      };
      demoConversations.unshift(conversation);
      demo.messages = [];
      return { action: "created", conversation: structuredClone(conversation), bootstrap: structuredClone(demo), resumed: false, serverVersion: "demo" };
    }),
  agentActivateConversation: (sessionId: string) =>
    tauriOrDemo<AgentConversationResult>("agent_activate_conversation", { sessionId }, () => {
      const conversation = demoConversations.find((entry) => entry.id === sessionId);
      if (!conversation) throw new Error("Gespräch nicht gefunden");
      demoConversations = demoConversations.map((entry) => ({ ...entry, status: entry.id === sessionId ? "active" as const : "archived" as const, active: entry.id === sessionId }));
      return { action: "activated", conversation: structuredClone({ ...conversation, status: "active" as const, active: true }), bootstrap: structuredClone(demo), resumed: true, serverVersion: "demo" };
    }),
  agentStop: () => tauriOrDemo<void>("agent_stop", {}, () => undefined),
  agentCompactContext: () => tauriOrDemo<void>("agent_compact_context", {}, async () => { await wait(350); }),
  onAgentEvent: async (handler: (event: AgentEvent) => void): Promise<UnlistenFn> => {
    if (!isTauri()) return () => undefined;
    return listen<AgentEvent>("agent:event", ({ payload }) => handler(payload));
  },
};

export const resetDemo = (onboardingComplete = false) => {
  demo = createDemoBootstrap(onboardingComplete);
  const now = new Date().toISOString();
  demoConversations = [{
    id: "demo-conversation-main", title: "MealZ Chat", status: "active", active: true,
    messageCount: demo.messages.length, preview: demo.messages[demo.messages.length - 1]?.content,
    createdAt: now, updatedAt: now,
  }];
  demoAgentFiles = {
    persona: "# PERSONA.md\n\nDu bist die persönliche Meal-Planning-Begleiterin des Nutzers. Du bist direkt, aufmerksam und pragmatisch.\n\n## Sprachregeln\n\n- Antworte auf Deutsch.\n- Verwende niemals Em-Dashes (Unicode U+2014). Nutze stattdessen Punkte, Kommas oder Doppelpunkte.\n- Erkläre Abwägungen kurz und konkret.\n",
    memory: "# MEMORY.md\n\nLangzeitkontext, der zusätzlich zu den strukturierten Erinnerungen in MealZ an jeden relevanten Turn übergeben wird.\n",
  };
};
