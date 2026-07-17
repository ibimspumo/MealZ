export type ViewId =
  | "today"
  | "week"
  | "recipes"
  | "shopping"
  | "agent"
  | "memory"
  | "settings";

export type MealType = "fruehstueck" | "mittagessen" | "abendessen" | "snack" | "shake" | "dessert" | "sonstiges";

export interface Nutrition {
  calories: number;
  protein: number;
  carbs: number;
  fat: number;
  fiber: number;
}

export interface Ingredient {
  id: string;
  name: string;
  amount: number;
  unit: string;
  category: string;
  optional?: boolean;
}

export interface Recipe {
  id: string;
  title: string;
  description: string;
  imageUrl?: string;
  mealTypes: MealType[];
  tags: string[];
  prepMinutes: number;
  cookMinutes: number;
  servings: number;
  nutrition: Nutrition;
  ingredients: Ingredient[];
  steps: string[];
  rating?: number;
  ratingComment?: string;
  sourceUrl?: string;
  sourceName?: string;
  lastCookedAt?: string;
  createdAt: string;
  updatedAt: string;
  favorite?: boolean;
}

export interface PlanItem {
  id: string;
  date: string;
  mealType: MealType;
  recipeId?: string;
  recipe?: Recipe;
  servings: number;
  status: "planned" | "prepared" | "cooked" | "leftovers" | "eaten" | "skipped" | "eating_out" | "cancelled";
  note?: string;
  /** Optional bridge fields for entries without a recipe, for example eating out. */
  title?: string;
  titleOverride?: string;
}

export interface ShoppingItem {
  id: string;
  name: string;
  amount: number;
  unit: string;
  category: string;
  checked: boolean;
  manual: boolean;
  recipeIds: string[];
}

export interface Memory {
  id: string;
  kind: string;
  title: string;
  content: string;
  confidence: number;
  source: string;
  preferenceScore?: number;
  evidence?: string[];
  active: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface Equipment {
  id: string;
  name: string;
  enabled: boolean;
}

export interface Profile {
  name: string;
  birthDate?: string;
  sexForEnergy?: "male" | "female";
  activityLevel: "inactive" | "low_active" | "active" | "very_active";
  calorieTargetMode: "calculated" | "manual";
  heightCm?: number;
  weightKg?: number;
  calorieTarget: number;
  proteinTarget: number;
  fiberTarget: number;
  budgetPreference: "sparsam" | "ausgewogen" | "flexibel";
  weekdayMaxMinutes: number;
  weekendMaxMinutes: number;
  cookingStyle: string;
  dislikes: string[];
  favorites: string[];
  equipment: Equipment[];
  agentName: string;
  agentPersonality: string;
  autonomy: "vorsichtig" | "ausgewogen" | "autonom";
}

export interface AgentToolActivity {
  id: string;
  name: string;
  label: string;
  status: "running" | "success" | "error";
  detail?: string;
  startedAt?: string;
  completedAt?: string;
  recipeId?: string;
  recipeTitle?: string;
}

export interface AgentMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  createdAt: string;
  streaming?: boolean;
  tools?: AgentToolActivity[];
  recipeId?: string;
  recipeTitle?: string;
}

export interface AgentFiles {
  persona: string;
  memory: string;
}

export interface AgentCapabilities {
  webSearch: boolean | "unknown";
  imageGeneration: boolean | "unknown";
}

export interface AgentConversation {
  id: string;
  title: string;
  status: "active" | "archived";
  active: boolean;
  threadId?: string;
  messageCount: number;
  preview?: string;
  createdAt: string;
  updatedAt: string;
}

export interface AgentConversationResult {
  action: "created" | "activated";
  conversation: AgentConversation;
  bootstrap: BootstrapData;
  resumed: boolean;
  serverVersion: string;
}

export type AgentContextStage = "unknown" | "healthy" | "warning" | "compacting" | "recommend_new" | "error";

export interface AgentContextState {
  stage: AgentContextStage;
  usedTokens?: number;
  totalTokens?: number;
  contextWindow?: number;
  utilizationPercent?: number;
  remainingPercent?: number;
  automaticCompaction: boolean;
  lastCompactedAt?: string;
  detail?: string;
}

export type AgentEvent =
  | { type: "message_delta"; messageId: string; delta: string }
  | { type: "message_completed"; message: AgentMessage }
  | { type: "tool_started"; activity: AgentToolActivity }
  | { type: "tool_completed"; activity: AgentToolActivity }
  | { type: "data_changed"; areas?: string[] }
  | { type: "context_updated"; context: Partial<AgentContextState> }
  | { type: "error"; message: string }
  | { type: "status"; status: "idle" | "thinking" | "streaming" };

export interface BootstrapData {
  onboardingComplete: boolean;
  recipes: Recipe[];
  plan: PlanItem[];
  shopping: ShoppingItem[];
  memories: Memory[];
  profile: Profile;
  messages: AgentMessage[];
}

export interface ToastMessage {
  id: string;
  tone: "success" | "error" | "info";
  title: string;
  detail?: string;
}
