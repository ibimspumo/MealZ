import { useEffect, useMemo, useState } from "react";
import { formatDistanceToNowStrict } from "date-fns";
import { de } from "date-fns/locale";
import { BookOpen, Check, ChefHat, Clock3, ExternalLink as ExternalLinkIcon, Filter, Heart, Pencil, Plus, Search, Sparkles, Star, Trash2, UsersRound, X } from "lucide-react";
import { Button, EmptyState, ExternalLink, IconButton, Modal, NutritionStrip, PageHeader, SafeRecipeImage, formatAmount } from "../components/Common";
import { useAppStore } from "../store";
import type { Ingredient, MealType, Recipe } from "../types";

type RecipeFilter = "alle" | "favoriten" | "schnell" | "vegetarisch";

export function Recipes() {
  const recipes = useAppStore((state) => state.recipes);
  const saveRecipe = useAppStore((state) => state.saveRecipe);
  const deleteRecipe = useAppStore((state) => state.deleteRecipe);
  const rateRecipe = useAppStore((state) => state.rateRecipe);
  const setView = useAppStore((state) => state.setView);
  const setAgentDraft = useAppStore((state) => state.setAgentDraft);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<RecipeFilter>("alle");
  const [detail, setDetail] = useState<Recipe | null>(null);
  const [editing, setEditing] = useState<Recipe | "new" | null>(null);
  const [rating, setRating] = useState<Recipe | null>(null);
  const selectedRecipeId = useAppStore((state) => state.selectedRecipeId);
  const setSelectedRecipeId = useAppStore((state) => state.setSelectedRecipeId);
  useEffect(() => {
    if (!selectedRecipeId) return;
    const next = recipes.find((recipe) => recipe.id === selectedRecipeId);
    if (next) setDetail(next);
    setSelectedRecipeId(undefined);
  }, [recipes, selectedRecipeId, setSelectedRecipeId]);
  const filtered = useMemo(() => recipes.filter((recipe) => {
    const text = `${recipe.title} ${recipe.description} ${recipe.tags.join(" ")}`.toLocaleLowerCase("de");
    if (query && !text.includes(query.toLocaleLowerCase("de"))) return false;
    if (filter === "favoriten") return recipe.favorite;
    if (filter === "schnell") return recipe.prepMinutes + recipe.cookMinutes <= 30;
    if (filter === "vegetarisch") return recipe.tags.includes("Vegetarisch");
    return true;
  }), [filter, query, recipes]);

  const remove = async (recipe: Recipe) => {
    if (!window.confirm(`„${recipe.title}“ wirklich löschen? Geplante Einträge werden ebenfalls entfernt.`)) return;
    await deleteRecipe(recipe.id);
    setDetail(null);
  };

  const regenerate = (recipe: Recipe) => {
    setDetail(null);
    setAgentDraft(`Erstelle eine neue Variante von „${recipe.title}“. Behalte den Charakter, berücksichtige aber mein bisheriges Feedback.`);
    setView("agent");
    window.setTimeout(() => document.querySelector<HTMLTextAreaElement>("#agent-composer")?.focus(), 120);
  };

  return (
    <div className="page page--recipes">
      <PageHeader title="Rezeptkatalog" description={`${recipes.length} gespeicherte Rezepte · deine dauerhafte persönliche Sammlung`} actions={<><Button icon={<Sparkles size={16} />} onClick={() => setView("agent")}>Generieren</Button><Button tone="primary" icon={<Plus size={16} />} onClick={() => setEditing("new")}>Neues Rezept</Button></>} />
      <div className="catalog-toolbar">
        <label className="search-field"><Search size={17} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Rezepte, Zutaten oder Tags suchen …" aria-label="Rezepte suchen" />{query && <IconButton label="Suche leeren" onClick={() => setQuery("")}><X size={14} /></IconButton>}</label>
        <div className="segmented" aria-label="Rezeptfilter"><Filter size={15} aria-hidden="true" />{(["alle", "favoriten", "schnell", "vegetarisch"] as RecipeFilter[]).map((value) => <button key={value} className={filter === value ? "is-active" : ""} onClick={() => setFilter(value)}>{value === "alle" ? "Alle" : value === "favoriten" ? "Favoriten" : value === "schnell" ? "≤ 30 Min." : "Vegetarisch"}</button>)}</div>
      </div>
      {filtered.length ? (
        <div className="recipe-grid">
          {filtered.map((recipe) => <RecipeCard key={recipe.id} recipe={recipe} onOpen={() => setDetail(recipe)} onFavorite={() => saveRecipe({ ...recipe, favorite: !recipe.favorite })} />)}
        </div>
      ) : (
        <EmptyState icon={<Search size={24} />} title="Kein passendes Rezept" action={<Button onClick={() => { setQuery(""); setFilter("alle"); }}>Filter zurücksetzen</Button>}><p>Probiere einen anderen Suchbegriff oder lass Mila ein neues Rezept entwickeln.</p></EmptyState>
      )}

      <RecipeDetail recipe={detail} onClose={() => setDetail(null)} onEdit={(recipe) => { setDetail(null); setEditing(recipe); }} onDelete={remove} onRate={(recipe) => { setDetail(null); setRating(recipe); }} onRegenerate={regenerate} />
      <RecipeEditor recipe={editing} onClose={() => setEditing(null)} onSave={async (recipe) => { const saved = await saveRecipe(recipe); setEditing(null); setDetail(saved); }} />
      <RatingDialog recipe={rating} onClose={() => setRating(null)} onSave={async (stars, comment) => { if (rating) await rateRecipe(rating.id, stars, comment); setRating(null); setDetail(null); }} />
    </div>
  );
}

function RecipeCard({ recipe, onOpen, onFavorite }: { recipe: Recipe; onOpen: () => void; onFavorite: () => void }) {
  return (
    <article className="recipe-card">
      <button className="recipe-card__open" onClick={onOpen} aria-label={`${recipe.title} öffnen`}>
        <span className="recipe-card__image"><SafeRecipeImage src={recipe.imageUrl} alt={`Serviervorschlag für ${recipe.title}`} fallback={<ChefHat size={30} />} />{recipe.rating && <span className="recipe-card__rating"><Star size={13} fill="currentColor" />{recipe.rating}</span>}</span>
        <span className="recipe-card__body">
          <span className="recipe-card__tags">{recipe.tags.slice(0, 2).map((tag) => <em key={tag}>{tag}</em>)}</span>
          <strong>{recipe.title}</strong>
          <small>{recipe.description}</small>
          <span className="recipe-card__footer"><span><Clock3 size={14} />{recipe.prepMinutes + recipe.cookMinutes} Min.</span><span><UsersRound size={14} />{recipe.servings}</span><span>{recipe.nutrition.protein} g Protein</span></span>
        </span>
      </button>
      <IconButton className={`favorite-button ${recipe.favorite ? "is-active" : ""}`} label={recipe.favorite ? "Aus Favoriten entfernen" : "Als Favorit speichern"} onClick={onFavorite}><Heart size={17} fill={recipe.favorite ? "currentColor" : "none"} /></IconButton>
    </article>
  );
}

function RecipeDetail({ recipe, onClose, onEdit, onDelete, onRate, onRegenerate }: { recipe: Recipe | null; onClose: () => void; onEdit: (recipe: Recipe) => void; onDelete: (recipe: Recipe) => void; onRate: (recipe: Recipe) => void; onRegenerate: (recipe: Recipe) => void }) {
  if (!recipe) return null;
  return (
    <Modal open onClose={onClose} title={recipe.title} size="large" footer={<><Button tone="danger" icon={<Trash2 size={15} />} onClick={() => onDelete(recipe)}>Löschen</Button><span className="modal__footer-spacer" /><Button icon={<Star size={15} />} onClick={() => onRate(recipe)}>Bewerten</Button><Button icon={<Sparkles size={15} />} onClick={() => onRegenerate(recipe)}>Neu interpretieren</Button><Button tone="primary" icon={<Pencil size={15} />} onClick={() => onEdit(recipe)}>Bearbeiten</Button></>}>
      <div className="recipe-detail">
        <div className="recipe-detail__hero"><SafeRecipeImage src={recipe.imageUrl} alt={`Serviervorschlag für ${recipe.title}`} fallback={<span><ChefHat size={36} /></span>} /><div>{recipe.tags.map((tag) => <em key={tag}>{tag}</em>)}</div></div>
        <p className="recipe-detail__lead">{recipe.description}</p>
        <div className="recipe-detail__facts"><span><Clock3 size={16} /><strong>{recipe.prepMinutes + recipe.cookMinutes} Min.</strong><small>{recipe.prepMinutes} Min. Vorbereitung</small></span><span><UsersRound size={16} /><strong>{recipe.servings} Portionen</strong><small>einfach skalierbar</small></span>{recipe.lastCookedAt && <span><BookOpen size={16} /><strong>Zuletzt gekocht</strong><small>vor {formatDistanceToNowStrict(new Date(recipe.lastCookedAt), { locale: de })}</small></span>}</div>
        <NutritionStrip nutrition={recipe.nutrition} />
        <div className="recipe-detail__columns">
          <section><h3>Zutaten</h3><ul className="ingredient-list">{recipe.ingredients.map((ingredient) => <li key={ingredient.id}><span>{ingredient.name}{ingredient.optional && <small>optional</small>}</span><strong>{formatAmount(ingredient.amount)} {ingredient.unit}</strong></li>)}</ul></section>
          <section><h3>Zubereitung</h3><ol className="step-list">{recipe.steps.map((step, index) => <li key={`${index}-${step.slice(0, 8)}`}><span>{index + 1}</span><p>{step}</p></li>)}</ol></section>
        </div>
        {recipe.sourceUrl && <ExternalLink className="source-link" href={recipe.sourceUrl}><ExternalLinkIcon size={15} />Quelle: {recipe.sourceName ?? recipe.sourceUrl}</ExternalLink>}
        {recipe.ratingComment && <blockquote className="rating-quote"><Star size={17} fill="currentColor" /><div><strong>Dein Feedback</strong><p>{recipe.ratingComment}</p></div></blockquote>}
      </div>
    </Modal>
  );
}

function blankRecipe(): Recipe {
  const stamp = new Date().toISOString();
  return { id: "", title: "", description: "", mealTypes: ["abendessen"], tags: [], prepMinutes: 10, cookMinutes: 20, servings: 2, nutrition: { calories: 0, protein: 0, carbs: 0, fat: 0, fiber: 0 }, ingredients: [], steps: [""], createdAt: stamp, updatedAt: stamp };
}

function RecipeEditor({ recipe, onClose, onSave }: { recipe: Recipe | "new" | null; onClose: () => void; onSave: (recipe: Recipe) => Promise<void> }) {
  const [draft, setDraft] = useState<Recipe>(blankRecipe());
  const key = recipe === "new" ? "new" : recipe?.id ?? null;
  useEffect(() => { if (key) setDraft(recipe && recipe !== "new" ? structuredClone(recipe) : blankRecipe()); }, [key, recipe]);
  const patch = <K extends keyof Recipe>(field: K, value: Recipe[K]) => setDraft((current) => ({ ...current, [field]: value }));
  const addIngredient = () => patch("ingredients", [...draft.ingredients, { id: crypto.randomUUID(), name: "", amount: 1, unit: "g", category: "Sonstiges" }]);
  const updateIngredient = (index: number, next: Ingredient) => patch("ingredients", draft.ingredients.map((item, itemIndex) => itemIndex === index ? next : item));
  const invalidIngredients = draft.ingredients.some((ingredient) => !ingredient.name.trim() || !ingredient.unit.trim() || !ingredient.category.trim() || !Number.isFinite(ingredient.amount) || ingredient.amount <= 0);
  const canSave = Boolean(draft.title.trim() && draft.description.trim() && draft.steps.some((step) => step.trim()) && !invalidIngredients);
  return (
    <Modal open={recipe !== null} onClose={onClose} title={recipe === "new" ? "Neues Rezept" : "Rezept bearbeiten"} description="Alle Daten bleiben lokal und können jederzeit geändert werden." size="large" footer={<><Button tone="quiet" onClick={onClose}>Abbrechen</Button><Button tone="primary" icon={<Check size={15} />} disabled={!canSave} onClick={() => onSave({ ...draft, title: draft.title.trim(), description: draft.description.trim(), tags: draft.tags.filter(Boolean), steps: draft.steps.filter((step) => step.trim()) })}>Rezept speichern</Button></>}>
      <div className="recipe-form">
        <div className="form-grid form-grid--2"><label className="field"><span>Titel</span><input value={draft.title} onChange={(event) => patch("title", event.target.value)} placeholder="z. B. Knuspriges Chili-Hähnchen" /></label><label className="field"><span>Bild-URL <small>optional</small></span><input value={draft.imageUrl ?? ""} onChange={(event) => patch("imageUrl", event.target.value)} placeholder="https://…" /></label></div>
        <label className="field"><span>Beschreibung</span><textarea rows={3} value={draft.description} onChange={(event) => patch("description", event.target.value)} placeholder="Was macht dieses Rezept besonders?" /></label>
        <div className="form-grid form-grid--4"><NumberField label="Vorbereitung" value={draft.prepMinutes} onChange={(value) => patch("prepMinutes", value)} suffix="Min." /><NumberField label="Kochen" value={draft.cookMinutes} onChange={(value) => patch("cookMinutes", value)} suffix="Min." /><NumberField label="Portionen" value={draft.servings} onChange={(value) => patch("servings", value)} /><label className="field"><span>Mahlzeit</span><select value={draft.mealTypes[0]} onChange={(event) => patch("mealTypes", [event.target.value as MealType])}><option value="fruehstueck">Frühstück</option><option value="mittagessen">Mittagessen</option><option value="abendessen">Abendessen</option><option value="snack">Snack</option><option value="shake">Shake</option><option value="dessert">Dessert</option><option value="sonstiges">Sonstiges</option></select></label></div>
        <label className="field"><span>Tags <small>durch Komma trennen</small></span><input value={draft.tags.join(", ")} onChange={(event) => patch("tags", event.target.value.split(",").map((tag) => tag.trim()))} placeholder="Proteinreich, Airfryer, Schnell" /></label>
        <fieldset className="form-section"><legend>Nährwerte pro Portion</legend><div className="form-grid form-grid--5"><NumberField label="Kalorien" value={draft.nutrition.calories} onChange={(value) => patch("nutrition", { ...draft.nutrition, calories: value })} /><NumberField label="Protein (g)" value={draft.nutrition.protein} onChange={(value) => patch("nutrition", { ...draft.nutrition, protein: value })} /><NumberField label="Kohlenhydrate (g)" value={draft.nutrition.carbs} onChange={(value) => patch("nutrition", { ...draft.nutrition, carbs: value })} /><NumberField label="Fett (g)" value={draft.nutrition.fat} onChange={(value) => patch("nutrition", { ...draft.nutrition, fat: value })} /><NumberField label="Ballaststoffe (g)" value={draft.nutrition.fiber} onChange={(value) => patch("nutrition", { ...draft.nutrition, fiber: value })} /></div></fieldset>
        <fieldset className="form-section"><legend>Zutaten</legend><div className="ingredient-editor">{draft.ingredients.map((ingredient, index) => <div key={ingredient.id}><input aria-label={`Zutat ${index + 1}`} aria-invalid={!ingredient.name.trim()} value={ingredient.name} onChange={(event) => updateIngredient(index, { ...ingredient, name: event.target.value })} placeholder="Zutat" /><input aria-label={`Menge ${index + 1}`} type="number" min="0.1" value={ingredient.amount} onChange={(event) => updateIngredient(index, { ...ingredient, amount: Number(event.target.value) })} /><input aria-label={`Einheit ${index + 1}`} aria-invalid={!ingredient.unit.trim()} value={ingredient.unit} onChange={(event) => updateIngredient(index, { ...ingredient, unit: event.target.value })} /><input aria-label={`Kategorie ${index + 1}`} aria-invalid={!ingredient.category.trim()} value={ingredient.category} onChange={(event) => updateIngredient(index, { ...ingredient, category: event.target.value })} /><IconButton label="Zutat entfernen" onClick={() => patch("ingredients", draft.ingredients.filter((_, itemIndex) => itemIndex !== index))}><Trash2 size={15} /></IconButton></div>)}</div>{invalidIngredients && <p className="form-error" role="alert">Jede Zutat braucht Name, Menge größer als null, Einheit und Kategorie.</p>}<Button tone="quiet" icon={<Plus size={15} />} onClick={addIngredient}>Zutat hinzufügen</Button></fieldset>
        <fieldset className="form-section"><legend>Zubereitung</legend><div className="step-editor">{draft.steps.map((step, index) => <div key={index}><span>{index + 1}</span><textarea rows={2} value={step} onChange={(event) => patch("steps", draft.steps.map((item, itemIndex) => itemIndex === index ? event.target.value : item))} placeholder="Arbeitsschritt beschreiben …" /><IconButton label="Schritt entfernen" onClick={() => patch("steps", draft.steps.filter((_, itemIndex) => itemIndex !== index))}><Trash2 size={15} /></IconButton></div>)}</div><Button tone="quiet" icon={<Plus size={15} />} onClick={() => patch("steps", [...draft.steps, ""])}>Schritt hinzufügen</Button></fieldset>
      </div>
    </Modal>
  );
}

function NumberField({ label, value, onChange, suffix }: { label: string; value: number; onChange: (value: number) => void; suffix?: string }) {
  return <label className="field"><span>{label}</span><span className="number-input"><input type="number" min="0" value={value} onChange={(event) => onChange(Number(event.target.value))} />{suffix && <small>{suffix}</small>}</span></label>;
}

function RatingDialog({ recipe, onClose, onSave }: { recipe: Recipe | null; onClose: () => void; onSave: (rating: number, comment: string) => Promise<void> }) {
  const [stars, setStars] = useState(0);
  const [comment, setComment] = useState("");
  useEffect(() => { setStars(recipe?.rating ?? 0); setComment(recipe?.ratingComment ?? ""); }, [recipe?.id]);
  return <Modal open={Boolean(recipe)} onClose={onClose} title="Wie war es?" description={recipe?.title} size="small" footer={<><Button tone="quiet" onClick={onClose}>Später</Button><Button tone="primary" disabled={!stars} onClick={() => onSave(stars, comment)}>Feedback speichern</Button></>}><div className="rating-form"><div className="star-picker" aria-label="Bewertung">{[1, 2, 3, 4, 5].map((value) => <button key={value} aria-label={`${value} Sterne`} onClick={() => setStars(value)} className={value <= stars ? "is-active" : ""}><Star size={27} fill={value <= stars ? "currentColor" : "none"} /></button>)}</div><label className="field"><span>Kommentar <small>hilft Mila beim Lernen</small></span><textarea rows={4} value={comment} onChange={(event) => setComment(event.target.value)} placeholder="Was soll genau so bleiben, was beim nächsten Mal anders sein?" /></label></div></Modal>;
}
