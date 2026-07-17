import { addDays, format, startOfWeek } from "date-fns";
import type {
  AgentMessage,
  BootstrapData,
  Memory,
  PlanItem,
  Profile,
  Recipe,
  ShoppingItem,
} from "./types";

const now = new Date();
const weekStart = startOfWeek(now, { weekStartsOn: 1 });
const day = (offset: number) => format(addDays(weekStart, offset), "yyyy-MM-dd");
const iso = now.toISOString();

export const demoRecipes: Recipe[] = [
  {
    id: "r-chicken",
    title: "Zitronen-Hähnchen mit Ofenkartoffeln",
    description: "Saftiges Hähnchen, knusprige Kartoffeln und grünes Gemüse aus einem Blech – werktagstauglich und proteinreich.",
    imageUrl: "https://images.unsplash.com/photo-1532550907401-a500c9a57435?auto=format&fit=crop&w=1200&q=82",
    mealTypes: ["mittagessen", "abendessen"],
    tags: ["Proteinreich", "Blechgericht", "45 Minuten"],
    prepMinutes: 12,
    cookMinutes: 33,
    servings: 2,
    nutrition: { calories: 684, protein: 57, carbs: 61, fat: 23, fiber: 10 },
    ingredients: [
      { id: "i1", name: "Hähnchenbrust", amount: 400, unit: "g", category: "Fleisch & Fisch" },
      { id: "i2", name: "Kartoffeln", amount: 600, unit: "g", category: "Obst & Gemüse" },
      { id: "i3", name: "Brokkoli", amount: 300, unit: "g", category: "Obst & Gemüse" },
      { id: "i4", name: "Zitrone", amount: 1, unit: "Stk.", category: "Obst & Gemüse" },
      { id: "i5", name: "Olivenöl", amount: 2, unit: "EL", category: "Vorrat" },
    ],
    steps: [
      "Ofen auf 210 °C Ober-/Unterhitze vorheizen. Kartoffeln vierteln und mit einem Esslöffel Öl, Salz und Pfeffer mischen.",
      "Kartoffeln 15 Minuten vorbacken. Hähnchen würzen, Brokkoli teilen und beides mit auf das Blech geben.",
      "Weitere 18 Minuten garen. Mit Zitronenabrieb und -saft abschließen und kurz ruhen lassen.",
    ],
    rating: 5,
    ratingComment: "Sehr gut – beim nächsten Mal noch etwas mehr Zitrone.",
    lastCookedAt: day(-10),
    favorite: true,
    createdAt: iso,
    updatedAt: iso,
  },
  {
    id: "r-orzo",
    title: "Cremige Tomaten-Orzo-Pfanne",
    description: "Orzo, Spinat und Feta in einer würzigen Tomatensauce. Ein Topf, wenig Abwasch, richtig viel Geschmack.",
    imageUrl: "https://images.unsplash.com/photo-1621996346565-e3dbc646d9a9?auto=format&fit=crop&w=1200&q=82",
    mealTypes: ["abendessen"],
    tags: ["Vegetarisch", "One Pot", "Schnell"],
    prepMinutes: 8,
    cookMinutes: 22,
    servings: 2,
    nutrition: { calories: 628, protein: 25, carbs: 88, fat: 19, fiber: 11 },
    ingredients: [
      { id: "i6", name: "Orzo", amount: 220, unit: "g", category: "Vorrat" },
      { id: "i7", name: "Passierte Tomaten", amount: 400, unit: "ml", category: "Konserven" },
      { id: "i8", name: "Babyspinat", amount: 150, unit: "g", category: "Obst & Gemüse" },
      { id: "i9", name: "Feta", amount: 120, unit: "g", category: "Kühlregal" },
    ],
    steps: [
      "Knoblauch kurz in Olivenöl anschwitzen, Orzo zugeben und eine Minute rösten.",
      "Passierte Tomaten und 300 ml Wasser angießen. 15 Minuten sanft köcheln und regelmäßig rühren.",
      "Spinat unterheben, Feta darüberbröseln und mit Pfeffer, Chili und Zitronensaft abschmecken.",
    ],
    lastCookedAt: day(-18),
    createdAt: iso,
    updatedAt: iso,
  },
  {
    id: "r-salmon",
    title: "Miso-Lachs mit Sesamreis",
    description: "Glasierter Lachs, duftender Reis und knackige Gurke – ausgewogen, frisch und in einer halben Stunde fertig.",
    imageUrl: "https://images.unsplash.com/photo-1467003909585-2f8a72700288?auto=format&fit=crop&w=1200&q=82",
    mealTypes: ["mittagessen", "abendessen"],
    tags: ["Omega-3", "30 Minuten", "Airfryer"],
    prepMinutes: 10,
    cookMinutes: 20,
    servings: 2,
    nutrition: { calories: 712, protein: 45, carbs: 76, fat: 25, fiber: 6 },
    ingredients: [
      { id: "i10", name: "Lachsfilet", amount: 360, unit: "g", category: "Fleisch & Fisch" },
      { id: "i11", name: "Jasminreis", amount: 180, unit: "g", category: "Vorrat" },
      { id: "i12", name: "Gurke", amount: 1, unit: "Stk.", category: "Obst & Gemüse" },
      { id: "i13", name: "Misopaste", amount: 2, unit: "EL", category: "Vorrat" },
    ],
    steps: [
      "Reis nach Packungsangabe garen. Misopaste mit Sojasauce und einem Teelöffel Honig verrühren.",
      "Lachs glasieren und im Airfryer bei 190 °C etwa 9 Minuten garen.",
      "Gurke leicht salzen, mit Reisessig marinieren und alles mit Sesam anrichten.",
    ],
    rating: 4,
    lastCookedAt: day(-7),
    createdAt: iso,
    updatedAt: iso,
  },
  {
    id: "r-lasagna",
    title: "Timos Lasagne",
    description: "Die bewährte Lasagne mit kräftiger Tomatensauce, cremiger Béchamel und goldener Käsekruste.",
    imageUrl: "https://images.unsplash.com/photo-1574894709920-11b28e7367e3?auto=format&fit=crop&w=1200&q=82",
    mealTypes: ["abendessen"],
    tags: ["Favorit", "Wochenende", "Mealprep"],
    prepMinutes: 25,
    cookMinutes: 50,
    servings: 4,
    nutrition: { calories: 795, protein: 48, carbs: 73, fat: 34, fiber: 9 },
    ingredients: [
      { id: "i14", name: "Rinderhack", amount: 500, unit: "g", category: "Fleisch & Fisch" },
      { id: "i15", name: "Lasagneplatten", amount: 250, unit: "g", category: "Vorrat" },
      { id: "i16", name: "Dosentomaten", amount: 800, unit: "g", category: "Konserven" },
      { id: "i17", name: "Mozzarella", amount: 250, unit: "g", category: "Kühlregal" },
    ],
    steps: [
      "Hack kräftig anbraten, Tomaten zugeben und die Sauce mindestens 20 Minuten einkochen.",
      "Béchamel rühren. Sauce, Platten und Béchamel abwechselnd in die Form schichten.",
      "Mit Mozzarella abschließen und bei 190 °C 35 Minuten backen. Vor dem Anschneiden 10 Minuten ruhen lassen.",
    ],
    rating: 5,
    ratingComment: "Genau so speichern. Das ist die Referenz.",
    lastCookedAt: day(-23),
    favorite: true,
    createdAt: iso,
    updatedAt: iso,
  },
];

export const demoPlan: PlanItem[] = [
  { id: "p1", date: day(0), mealType: "abendessen", recipeId: "r-orzo", recipe: demoRecipes[1], servings: 2, status: "planned" },
  { id: "p2", date: day(1), mealType: "abendessen", recipeId: "r-chicken", recipe: demoRecipes[0], servings: 2, status: "planned" },
  { id: "p3", date: day(2), mealType: "abendessen", recipeId: "r-salmon", recipe: demoRecipes[2], servings: 2, status: "planned" },
  { id: "p4", date: day(4), mealType: "abendessen", recipeId: "r-chicken", recipe: demoRecipes[0], servings: 1, status: "planned", note: "Rest vom Dienstag" },
  { id: "p5", date: day(6), mealType: "abendessen", recipeId: "r-lasagna", recipe: demoRecipes[3], servings: 4, status: "planned" },
];

export const demoShopping: ShoppingItem[] = [
  { id: "s1", name: "Hähnchenbrust", amount: 600, unit: "g", category: "Fleisch & Fisch", checked: false, manual: false, recipeIds: ["r-chicken"] },
  { id: "s2", name: "Lachsfilet", amount: 360, unit: "g", category: "Fleisch & Fisch", checked: true, manual: false, recipeIds: ["r-salmon"] },
  { id: "s3", name: "Kartoffeln", amount: 900, unit: "g", category: "Obst & Gemüse", checked: false, manual: false, recipeIds: ["r-chicken"] },
  { id: "s4", name: "Brokkoli", amount: 450, unit: "g", category: "Obst & Gemüse", checked: false, manual: false, recipeIds: ["r-chicken"] },
  { id: "s5", name: "Babyspinat", amount: 150, unit: "g", category: "Obst & Gemüse", checked: false, manual: false, recipeIds: ["r-orzo"] },
  { id: "s6", name: "Feta", amount: 120, unit: "g", category: "Kühlregal", checked: false, manual: false, recipeIds: ["r-orzo"] },
  { id: "s7", name: "Mozzarella", amount: 250, unit: "g", category: "Kühlregal", checked: false, manual: false, recipeIds: ["r-lasagna"] },
  { id: "s8", name: "Orzo", amount: 220, unit: "g", category: "Vorrat", checked: true, manual: false, recipeIds: ["r-orzo"] },
];

export const demoMemories: Memory[] = [
  { id: "m1", kind: "routine", title: "Unter der Woche schnell", content: "Montag bis Freitag abends möglichst Gerichte unter 45 Minuten planen.", confidence: 1, source: "explicit", active: true, createdAt: iso, updatedAt: iso },
  { id: "m2", kind: "preference", title: "Kein Frühstück", content: "Frühstück nicht automatisch einplanen. Fokus auf Mittagessen, Abendessen und Snacks.", confidence: 1, source: "explicit", active: true, createdAt: iso, updatedAt: iso },
  { id: "m3", kind: "preference", title: "Karotten eher vermeiden", content: "Karotten werden gegessen, aber nur ungern. Wenn möglich eine Alternative wählen.", confidence: 0.92, source: "explicit", active: true, createdAt: iso, updatedAt: iso },
  { id: "m4", kind: "feedback", title: "Zitrone darf kräftiger sein", content: "Bei Zitronengerichten die Säure etwas deutlicher ausarbeiten.", confidence: 0.85, source: "rating", active: true, createdAt: iso, updatedAt: iso },
];

export const demoProfile: Profile = {
  name: "Timo",
  birthDate: "1995-01-01",
  sexForEnergy: "male",
  activityLevel: "low_active",
  calorieTargetMode: "calculated",
  heightCm: 184,
  weightKg: 86,
  calorieTarget: 2450,
  proteinTarget: 172,
  fiberTarget: 36,
  budgetPreference: "ausgewogen",
  weekdayMaxMinutes: 45,
  weekendMaxMinutes: 90,
  cookingStyle: "Alltagstauglich, proteinreich und abwechslungsreich. Unter der Woche effizient, am Wochenende gern aufwendiger.",
  dislikes: ["Karotten"],
  favorites: ["Lasagne", "Hähnchen", "Lachs", "Ofenkartoffeln"],
  equipment: [
    { id: "e1", name: "Herd (4 Kochfelder)", enabled: true },
    { id: "e2", name: "Backofen", enabled: true },
    { id: "e3", name: "Dual-Heißluftfritteuse", enabled: true },
    { id: "e4", name: "Mixer", enabled: true },
    { id: "e5", name: "Kontaktgrill", enabled: true },
    { id: "e6", name: "Mikrowelle", enabled: true },
    { id: "e7", name: "Monsieur Cuisine", enabled: true },
  ],
  agentName: "Mila",
  agentPersonality: "Direkt, aufmerksam und pragmatisch. Macht klare Vorschläge, erklärt Abwägungen kurz und merkt sich Feedback.",
  autonomy: "ausgewogen",
};

export const demoMessages: AgentMessage[] = [
  {
    id: "a1",
    role: "assistant",
    content: "Guten Morgen, Timo. Für diese Woche stehen **fünf Gerichte** im Plan. Mittwoch ist der Miso-Lachs dran, Sonntag endlich wieder deine Lasagne – die gab es seit über drei Wochen nicht.\n\nSoll ich die beiden offenen Tage passend zu deinen Werktagen ergänzen?",
    createdAt: iso,
  },
];

export const createDemoBootstrap = (onboardingComplete = false): BootstrapData => ({
  onboardingComplete,
  recipes: structuredClone(demoRecipes),
  plan: structuredClone(demoPlan),
  shopping: structuredClone(demoShopping),
  memories: structuredClone(demoMemories),
  profile: structuredClone(demoProfile),
  messages: structuredClone(demoMessages),
});
