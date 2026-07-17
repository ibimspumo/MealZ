import { create } from "zustand";
import { addDays, differenceInCalendarDays, format, startOfWeek } from "date-fns";
import { api } from "./bridge";
import type {
  AgentMessage,
  AgentCapabilities,
  BootstrapData,
  Memory,
  PlanItem,
  Profile,
  Recipe,
  ShoppingItem,
  ToastMessage,
  ViewId,
} from "./types";

interface AppStore extends BootstrapData {
  view: ViewId;
  loading: boolean;
  agentStatus: "idle" | "thinking" | "streaming";
  activeAgentMessageId?: string;
  agentDraft: string;
  agentCapabilities: AgentCapabilities;
  selectedRecipeId?: string;
  onboardingSessionDismissed: boolean;
  toasts: ToastMessage[];
  weekStart: string;
  weekEnd: string;
  shoppingStart: string;
  shoppingEnd: string;
  setView: (view: ViewId) => void;
  setAgentDraft: (draft: string) => void;
  setSelectedRecipeId: (id?: string) => void;
  setWeekStart: (date: string) => Promise<void>;
  setCalendarRange: (startDate: string, endDate: string) => Promise<void>;
  loadShoppingRange: (startDate: string, endDate: string) => Promise<void>;
  initialize: () => Promise<void>;
  completeOnboarding: (profile: Profile, briefing?: string) => Promise<void>;
  restartOnboarding: () => Promise<void>;
  continueOnboardingInChat: (profile: Profile, briefing?: string) => Promise<void>;
  saveRecipe: (recipe: Recipe) => Promise<Recipe>;
  deleteRecipe: (id: string) => Promise<void>;
  rateRecipe: (id: string, rating: number, comment: string) => Promise<void>;
  savePlanItem: (item: PlanItem) => Promise<void>;
  removePlanItem: (id: string) => Promise<void>;
  rebuildShopping: (startDate?: string, endDate?: string) => Promise<void>;
  toggleShopping: (id: string, checked: boolean) => Promise<void>;
  addShopping: (item: ShoppingItem, startDate?: string, endDate?: string) => Promise<void>;
  deleteShopping: (id: string) => Promise<void>;
  saveProfile: (profile: Profile) => Promise<void>;
  saveMemory: (memory: Memory) => Promise<void>;
  deleteMemory: (id: string) => Promise<void>;
  sendMessage: (content: string) => Promise<void>;
  newThread: () => Promise<void>;
  stopAgent: () => Promise<void>;
  toast: (tone: ToastMessage["tone"], title: string, detail?: string) => void;
  dismissToast: (id: string) => void;
}

const monday = format(startOfWeek(new Date(), { weekStartsOn: 1 }), "yyyy-MM-dd");
const sunday = format(addDays(new Date(`${monday}T12:00:00`), 6), "yyyy-MM-dd");
let bootstrapRefreshSequence = 0;
let shoppingRequestSequence = 0;
const empty: BootstrapData = {
  onboardingComplete: false,
  recipes: [], plan: [], shopping: [], memories: [], messages: [],
  profile: {
    name: "", activityLevel: "low_active", calorieTargetMode: "calculated", calorieTarget: 2400, proteinTarget: 170, fiberTarget: 35,
    budgetPreference: "ausgewogen", weekdayMaxMinutes: 45, weekendMaxMinutes: 90,
    cookingStyle: "", dislikes: [], favorites: [], equipment: [], agentName: "Mila",
    agentPersonality: "Direkt, aufmerksam und pragmatisch.", autonomy: "ausgewogen",
  },
};

export const useAppStore = create<AppStore>((set, get) => ({
  ...empty,
  view: "today",
  loading: true,
  agentStatus: "idle",
  activeAgentMessageId: undefined,
  agentDraft: "",
  agentCapabilities: { webSearch: "unknown", imageGeneration: "unknown" },
  selectedRecipeId: undefined,
  onboardingSessionDismissed: false,
  toasts: [],
  weekStart: monday,
  weekEnd: sunday,
  shoppingStart: monday,
  shoppingEnd: sunday,
  setView: (view) => set({ view }),
  setAgentDraft: (agentDraft) => set({ agentDraft }),
  setSelectedRecipeId: (selectedRecipeId) => set({ selectedRecipeId }),
  setWeekStart: async (weekStart) => get().setCalendarRange(weekStart, format(addDays(new Date(`${weekStart}T12:00:00`), 6), "yyyy-MM-dd")),
  setCalendarRange: async (weekStart, weekEnd) => {
    const length = differenceInCalendarDays(new Date(`${weekEnd}T12:00:00`), new Date(`${weekStart}T12:00:00`));
    if (!weekStart || !weekEnd || length < 0 || length > 30) { get().toast("error", "Ungültiger Zeitraum", "Wähle einen Bereich von höchstens 31 Tagen."); return; }
    // Calendar range refreshes are in-place. A global skeleton would unmount the
    // calendar (and immediately trigger another range refresh) on every revisit.
    set({ weekStart, weekEnd });
    try {
      const plan = await api.getWeekPlan(weekStart, weekEnd);
      set({ plan });
    } catch (error) {
      get().toast("error", "Woche konnte nicht geladen werden", String(error));
    }
  },
  loadShoppingRange: async (shoppingStart, shoppingEnd) => {
    const request = ++shoppingRequestSequence;
    set({ shoppingStart, shoppingEnd });
    try {
      const shopping = await api.getShoppingList(shoppingStart, shoppingEnd);
      if (request === shoppingRequestSequence && get().shoppingStart === shoppingStart && get().shoppingEnd === shoppingEnd) set({ shopping });
    } catch (error) {
      get().toast("error", "Einkaufsliste konnte nicht geladen werden", String(error));
      throw error;
    }
  },
  initialize: async () => {
    set({ loading: true });
    try {
      const data = await api.getBootstrap();
      set({ ...data, onboardingSessionDismissed: false });
      void api.agentCapabilities().then((agentCapabilities) => set({ agentCapabilities })).catch(() => set({ agentCapabilities: { webSearch: "unknown", imageGeneration: "unknown" } }));
      await api.onAgentEvent((event) => {
        if (event.type === "status") set({ agentStatus: event.status });
        if (event.type === "message_delta") {
          set((state) => {
            const exists = state.messages.some((message) => message.id === event.messageId);
            if (exists) return { activeAgentMessageId: event.messageId, agentStatus: "streaming", messages: state.messages.map((message) => message.id === event.messageId ? { ...message, content: message.content + event.delta, streaming: true } : message) };
            const pending = state.activeAgentMessageId ? state.messages.find((message) => message.id === state.activeAgentMessageId) : undefined;
            if (pending) return { activeAgentMessageId: event.messageId, agentStatus: "streaming", messages: state.messages.map((message) => message.id === pending.id ? { ...message, id: event.messageId, content: `${message.content}${event.delta}`, streaming: true } : message) };
            return { activeAgentMessageId: event.messageId, agentStatus: "streaming", messages: [...state.messages, { id: event.messageId, role: "assistant", content: event.delta, createdAt: new Date().toISOString(), streaming: true }] };
          });
        }
        if (event.type === "message_completed") {
          set((state) => {
            const pending = state.messages.find((message) => message.id === event.message.id) ?? (state.activeAgentMessageId ? state.messages.find((message) => message.id === state.activeAgentMessageId) : undefined);
            const finalMessage = { ...pending, ...event.message, streaming: false, tools: event.message.tools?.length ? event.message.tools : pending?.tools, recipeId: event.message.recipeId ?? pending?.recipeId, recipeTitle: event.message.recipeTitle ?? pending?.recipeTitle };
            return {
            messages: [...state.messages.filter((message) => message.id !== event.message.id && !(message.role === "assistant" && message.streaming)), finalMessage],
            // Codex may emit an assistant message, continue with tools, and only
            // finish the turn later. Keep the global busy state until the
            // explicit turn/completed status event arrives.
            agentStatus: state.agentStatus === "idle" ? "idle" : "thinking",
            activeAgentMessageId: undefined,
          }; });
        }
        if (event.type === "tool_started") {
          set((state) => {
            const pending = state.activeAgentMessageId ? state.messages.find((message) => message.id === state.activeAgentMessageId) : undefined;
            if (pending) {
              return { agentStatus: "thinking", messages: state.messages.map((message) => message.id === pending.id ? { ...message, tools: [...(message.tools ?? []).filter((tool) => tool.id !== event.activity.id), event.activity] } : message) };
            }
            const id = `stream-${crypto.randomUUID()}`;
            return { activeAgentMessageId: id, agentStatus: "thinking", messages: [...state.messages, { id, role: "assistant", content: "", createdAt: new Date().toISOString(), streaming: true, tools: [event.activity] }] };
          });
        }
        if (event.type === "tool_completed") {
          set((state) => ({ messages: state.messages.map((message) => message.id === state.activeAgentMessageId || message.tools?.some((tool) => tool.id === event.activity.id) ? { ...message, tools: [...(message.tools ?? []).filter((tool) => tool.id !== event.activity.id), event.activity], recipeId: event.activity.recipeId ?? message.recipeId, recipeTitle: event.activity.recipeTitle ?? message.recipeTitle } : message) }));
        }
        if (event.type === "data_changed") {
          const refresh = ++bootstrapRefreshSequence;
          const { shoppingStart, shoppingEnd } = get();
          void Promise.all([api.getBootstrap(), api.getShoppingList(shoppingStart, shoppingEnd)]).then(([data, shopping]) => {
            if (refresh !== bootstrapRefreshSequence) return;
            set((state) => ({
              ...data,
              shopping: state.shoppingStart === shoppingStart && state.shoppingEnd === shoppingEnd ? shopping : state.shopping,
              shoppingStart: state.shoppingStart,
              shoppingEnd: state.shoppingEnd,
              weekStart: state.weekStart,
              weekEnd: state.weekEnd,
              messages: state.messages.length ? state.messages : data.messages,
            }));
          }).catch((error: unknown) => get().toast("error", "Änderungen konnten nicht synchronisiert werden", String(error)));
        }
        if (event.type === "error") get().toast("error", "Agentenfehler", event.message);
      });
    } catch (error) {
      get().toast("error", "MealZ konnte nicht geladen werden", String(error));
    } finally {
      set({ loading: false });
    }
  },
  completeOnboarding: async (profile, briefing) => {
    const result = await api.completeOnboarding(profile, briefing?.trim() || undefined);
    const data = result ?? await api.getBootstrap();
    set({ ...data, onboardingComplete: true, onboardingSessionDismissed: false });
    get().toast("success", "MealZ ist bereit", `${get().profile.agentName || "Dein Agent"} kennt jetzt deinen persönlichen Rahmen.`);
  },
  restartOnboarding: async () => {
    await api.restartOnboarding();
    set({ onboardingComplete: false, onboardingSessionDismissed: false });
  },
  continueOnboardingInChat: async (profile, briefing) => {
    const savedProfile = await api.saveProfile(profile);
    set({ profile: savedProfile, onboardingSessionDismissed: true, view: "agent" });
    const context = [
      `Ich möchte mein MealZ-Onboarding gemeinsam mit dir abschließen. Ich heiße ${savedProfile.name}.`,
      `Meine Leitplanken: etwa ${savedProfile.calorieTarget} kcal, ${savedProfile.proteinTarget} g Protein und ${savedProfile.fiberTarget} g Ballaststoffe pro Tag.`,
      `Werktags habe ich maximal ${savedProfile.weekdayMaxMinutes} Minuten, am Wochenende ${savedProfile.weekendMaxMinutes} Minuten. Budget: ${savedProfile.budgetPreference}.`,
      savedProfile.favorites.length ? `Besonders gern esse ich: ${savedProfile.favorites.join(", ")}.` : "",
      savedProfile.dislikes.length ? `Eher vermeiden möchte ich: ${savedProfile.dislikes.join(", ")}.` : "",
      briefing?.trim() ? `Mein zusätzliches Briefing: ${briefing.trim()}` : "",
      "Frag gezielt nach fehlendem Kontext und schließe das Onboarding danach mit dem strukturierten Tool ab.",
    ].filter(Boolean).join("\n\n");
    await get().sendMessage(context);
  },
  saveRecipe: async (recipe) => {
    const saved = await api.saveRecipe(recipe);
    set((state) => ({ recipes: [saved, ...state.recipes.filter((entry) => entry.id !== saved.id)] }));
    get().toast("success", recipe.id ? "Rezept gespeichert" : "Rezept angelegt");
    return saved;
  },
  deleteRecipe: async (id) => {
    await api.deleteRecipe(id);
    set((state) => ({ recipes: state.recipes.filter((recipe) => recipe.id !== id), plan: state.plan.filter((entry) => entry.recipeId !== id) }));
    get().toast("success", "Rezept gelöscht");
  },
  rateRecipe: async (id, rating, comment) => {
    const updated = await api.rateRecipe(id, rating, comment);
    set((state) => ({ recipes: state.recipes.map((recipe) => recipe.id === id ? updated : recipe) }));
    get().toast("success", "Bewertung gespeichert", "Der Agent berücksichtigt dein Feedback künftig.");
  },
  savePlanItem: async (item) => {
    const saved = await api.savePlanItem(item);
    set((state) => ({ plan: [...state.plan.filter((entry) => entry.id !== saved.id), saved] }));
    get().toast("success", "Wochenplan aktualisiert");
  },
  removePlanItem: async (id) => {
    await api.removePlanItem(id);
    set((state) => ({ plan: state.plan.filter((entry) => entry.id !== id) }));
    get().toast("success", "Eintrag entfernt");
  },
  rebuildShopping: async (startDate, endDate) => {
    const start = startDate ?? get().shoppingStart;
    const end = endDate ?? get().shoppingEnd;
    const shopping = await api.rebuildShoppingList(start, end);
    set({ shopping, shoppingStart: start, shoppingEnd: end });
    get().toast("success", "Einkaufsliste neu berechnet", `${shopping.length} Positionen aus deinem Plan`);
  },
  toggleShopping: async (id, checked) => {
    const updated = await api.toggleShoppingItem(id, checked);
    set((state) => ({ shopping: state.shopping.map((item) => item.id === id ? updated : item) }));
  },
  addShopping: async (item, startDate, endDate) => {
    const start = startDate ?? get().shoppingStart;
    const end = endDate ?? get().shoppingEnd;
    const saved = await api.addShoppingItem(item, start, end);
    set((state) => ({ shopping: [...state.shopping, saved] }));
    get().toast("success", "Artikel ergänzt");
  },
  deleteShopping: async (id) => {
    await api.deleteShoppingItem(id);
    set((state) => ({ shopping: state.shopping.filter((item) => item.id !== id) }));
  },
  saveProfile: async (profile) => {
    const saved = await api.saveProfile(profile);
    set({ profile: saved });
    get().toast("success", "Profil gespeichert", "Deine nächsten Vorschläge nutzen die neuen Angaben.");
  },
  saveMemory: async (memory) => {
    const saved = await api.saveMemory(memory);
    set((state) => ({ memories: [saved, ...state.memories.filter((entry) => entry.id !== saved.id)] }));
    get().toast("success", "Erinnerung gespeichert");
  },
  deleteMemory: async (id) => {
    await api.deleteMemory(id);
    set((state) => ({ memories: state.memories.filter((memory) => memory.id !== id) }));
    get().toast("success", "Erinnerung gelöscht");
  },
  sendMessage: async (content) => {
    const optimistic: AgentMessage = { id: `user-${crypto.randomUUID()}`, role: "user", content, createdAt: new Date().toISOString() };
    set((state) => ({ messages: [...state.messages, optimistic], agentStatus: "thinking", activeAgentMessageId: undefined }));
    try {
      const reply = await api.agentSend(content);
      if (reply) set((state) => ({ messages: [...state.messages, reply], agentStatus: "idle", activeAgentMessageId: undefined }));
    } catch (error) {
      set({ agentStatus: "idle", activeAgentMessageId: undefined });
      get().toast("error", "Nachricht nicht gesendet", String(error));
      throw error;
    }
  },
  newThread: async () => {
    const data = await api.agentNewThread();
    set({ ...data, activeAgentMessageId: undefined, agentStatus: "idle" });
    get().toast("success", "Neues Gespräch gestartet");
  },
  stopAgent: async () => {
    await api.agentStop();
    set({ agentStatus: "idle" });
    get().toast("info", "Agent gestoppt");
  },
  toast: (tone, title, detail) => {
    const id = crypto.randomUUID();
    set((state) => ({ toasts: [...state.toasts, { id, tone, title, detail }] }));
    window.setTimeout(() => get().dismissToast(id), 4200);
  },
  dismissToast: (id) => set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),
}));
