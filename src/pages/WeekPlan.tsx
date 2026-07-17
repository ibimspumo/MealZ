import { useEffect, useMemo, useState, type KeyboardEvent } from "react";
import { addDays, differenceInCalendarDays, format, isSameDay, parseISO, startOfWeek } from "date-fns";
import { de } from "date-fns/locale";
import { CalendarPlus, ChevronLeft, ChevronRight, Clock3, CookingPot, ExternalLink as ExternalLinkIcon, MapPin, Minus, Plus, Sparkles, Trash2, UsersRound } from "lucide-react";
import { Button, EmptyState, ExternalLink, IconButton, Modal, NutritionStrip, PageHeader, SafeRecipeImage, formatAmount } from "../components/Common";
import { DateRangePicker } from "../components/DateRangePicker";
import { useAppStore } from "../store";
import type { MealType, PlanItem, Recipe } from "../types";

const mealLabels: Record<MealType, string> = { fruehstueck: "Frühstück", mittagessen: "Mittagessen", abendessen: "Abendessen", snack: "Snack", shake: "Shake", dessert: "Dessert", sonstiges: "Sonstiges" };

export function WeekPlan() {
  const weekStart = useAppStore((state) => state.weekStart);
  const weekEnd = useAppStore((state) => state.weekEnd);
  const setCalendarRange = useAppStore((state) => state.setCalendarRange);
  const plan = useAppStore((state) => state.plan);
  const recipes = useAppStore((state) => state.recipes);
  const setView = useAppStore((state) => state.setView);
  const setAgentDraft = useAppStore((state) => state.setAgentDraft);
  const savePlanItem = useAppStore((state) => state.savePlanItem);
  const removePlanItem = useAppStore((state) => state.removePlanItem);
  const rebuildShopping = useAppStore((state) => state.rebuildShopping);
  const [editingDay, setEditingDay] = useState<string | null>(null);
  const [selectedRecipe, setSelectedRecipe] = useState<string>(recipes[0]?.id ?? "");
  const [mealType, setMealType] = useState<MealType>("abendessen");
  const [servings, setServings] = useState(2);
  const [entryMode, setEntryMode] = useState<"recipe" | "eating_out">("recipe");
  const [customTitle, setCustomTitle] = useState("Auswärts essen");
  const [layout, setLayout] = useState<"calendar" | "agenda">("calendar");
  const [detailRecipe, setDetailRecipe] = useState<Recipe | null>(null);
  const start = parseISO(weekStart);
  const end = parseISO(weekEnd);
  const days = useMemo(() => Array.from({ length: differenceInCalendarDays(end, start) + 1 }, (_, index) => addDays(start, index)), [end, start]);
  const rangeDays = days.length;

  useEffect(() => { void setCalendarRange(weekStart, weekEnd); }, []);

  const navigate = (direction: number) => setCalendarRange(format(addDays(start, direction * rangeDays), "yyyy-MM-dd"), format(addDays(end, direction * rangeDays), "yyyy-MM-dd"));
  const beginEditing = (date: string) => { setEntryMode("recipe"); setCustomTitle("Auswärts essen"); setEditingDay(date); };
  const addMeal = async () => {
    const recipe = recipes.find((entry) => entry.id === selectedRecipe);
    if (!editingDay || (entryMode === "recipe" && !recipe) || (entryMode === "eating_out" && !customTitle.trim())) return;
    const item: PlanItem = entryMode === "eating_out"
      ? { id: "", date: editingDay, mealType, servings: 1, status: "eating_out", titleOverride: customTitle.trim() }
      : { id: "", date: editingDay, mealType, recipeId: recipe!.id, recipe, servings, status: "planned" };
    await savePlanItem(item);
    setEditingDay(null);
  };
  const askForWeek = () => {
    setAgentDraft("Plane meine kommende Woche von Montag bis Sonntag mit sieben abwechslungsreichen Hauptgerichten. Unter der Woche bitte unter 45 Minuten, am Wochenende darf es aufwendiger sein. Prüfe Favoriten, Bewertungen und meine aktiven Erinnerungen.");
    setView("agent");
    window.setTimeout(() => document.querySelector<HTMLTextAreaElement>("#agent-composer")?.focus(), 100);
  };

  return (
    <div className="page page--week">
      <PageHeader
        title="Wochenplan"
        description={`${rangeDays} ${rangeDays === 1 ? "Tag" : "Tage"}, klar geplant und jederzeit flexibel.`}
        actions={<><Button icon={<CalendarPlus size={16} />} onClick={() => rebuildShopping(weekStart, weekEnd)}>Einkauf aktualisieren</Button><Button tone="primary" icon={<Sparkles size={16} />} onClick={askForWeek}>Mit Agent planen</Button></>}
      />
      <div className="week-toolbar">
        <div className="week-navigation">
          <IconButton label="Vorherigen Zeitraum" onClick={() => navigate(-1)}><ChevronLeft size={18} /></IconButton>
          <DateRangePicker start={weekStart} end={weekEnd} label="Kalenderzeitraum" onChange={(range) => void setCalendarRange(range.start, range.end)} />
          <IconButton label="Nächsten Zeitraum" onClick={() => navigate(1)}><ChevronRight size={18} /></IconButton>
        </div>
        <div className="week-toolbar__actions"><div className="segmented" role="group" aria-label="Darstellung des Wochenplans"><button type="button" aria-pressed={layout === "calendar"} className={layout === "calendar" ? "is-active" : ""} onClick={() => setLayout("calendar")}>Kalender</button><button type="button" aria-pressed={layout === "agenda"} className={layout === "agenda" ? "is-active" : ""} onClick={() => setLayout("agenda")}>Agenda</button></div><Button tone="quiet" onClick={() => { const monday = startOfWeek(new Date(), { weekStartsOn: 1 }); void setCalendarRange(format(monday, "yyyy-MM-dd"), format(addDays(monday, 6), "yyyy-MM-dd")); }}>Heute</Button></div>
      </div>

      {layout === "calendar" ? <div className="week-grid-scroll"><div className="week-grid" style={{ gridTemplateColumns: `repeat(${rangeDays}, minmax(156px, 1fr))`, minWidth: `${Math.max(7, rangeDays) * 156}px` }} role="list" aria-label="Kalenderplan">
        {days.map((date) => {
          const dateKey = format(date, "yyyy-MM-dd");
          const items = plan.filter((item) => item.date === dateKey);
          const today = isSameDay(date, new Date());
          return (
            <article className={`day-column ${today ? "day-column--today" : ""}`} key={dateKey} role="listitem">
              <header>
                <span>{format(date, "EEEE", { locale: de })}</span>
                <strong>{format(date, "d")}</strong>
              </header>
              <div className="day-column__meals">
                {items.map((item) => (
                  <PlanCard variant="calendar" item={item} onOpen={setDetailRecipe} onRemove={() => removePlanItem(item.id)} key={item.id} />
                ))}
                {!items.length && (
                  <button className="empty-slot" onClick={() => beginEditing(dateKey)}>
                    <Plus size={17} /><span>Gericht einplanen</span>
                  </button>
                )}
                {!!items.length && <button className="add-slot" onClick={() => beginEditing(dateKey)}><Plus size={14} /> Weiteres</button>}
              </div>
            </article>
          );
        })}
      </div></div> : <section className="week-agenda" aria-label="Agenda des ausgewählten Zeitraums">{days.map((date) => {
        const dateKey = format(date, "yyyy-MM-dd"); const items = plan.filter((item) => item.date === dateKey); const today = isSameDay(date, new Date());
        return <article className={`agenda-day ${today ? "agenda-day--today" : ""}`} key={dateKey}><header><div><span>{format(date, "EEEE", { locale: de })}</span><h2>{format(date, "d. MMMM", { locale: de })}</h2></div>{today && <strong>Heute</strong>}<Button tone="quiet" icon={<Plus size={15} />} onClick={() => beginEditing(dateKey)}>Eintrag</Button></header><div className="agenda-day__items">{items.length ? items.map((item) => <PlanCard variant="agenda" item={item} onOpen={setDetailRecipe} onRemove={() => removePlanItem(item.id)} key={item.id} />) : <button className="agenda-empty" onClick={() => beginEditing(dateKey)}><Plus size={16} />Gericht oder Termin hinzufügen</button>}</div></article>;
      })}</section>}

      {!plan.length && <EmptyState icon={<CookingPot size={25} />} title="Die Woche wartet auf Ideen" action={<Button tone="primary" onClick={askForWeek}>Mit Mila planen</Button>}><p>Lass sieben passende Hauptgerichte erstellen oder trage selbst deine Favoriten ein.</p></EmptyState>}

      <Modal open={Boolean(editingDay)} onClose={() => setEditingDay(null)} title="Kalendereintrag planen" description={editingDay ? format(parseISO(editingDay), "EEEE, d. MMMM", { locale: de }) : undefined} footer={<><Button tone="quiet" onClick={() => setEditingDay(null)}>Abbrechen</Button><Button tone="primary" onClick={addMeal} disabled={entryMode === "recipe" ? !selectedRecipe : !customTitle.trim()}>Einplanen</Button></>}>
        <div className="form-stack">
          <div className="segmented plan-entry-mode" role="group" aria-label="Art des Kalendereintrags"><button type="button" aria-pressed={entryMode === "recipe"} className={entryMode === "recipe" ? "is-active" : ""} onClick={() => setEntryMode("recipe")}>Rezept</button><button type="button" aria-pressed={entryMode === "eating_out"} className={entryMode === "eating_out" ? "is-active" : ""} onClick={() => setEntryMode("eating_out")}>Auswärts</button></div>
          <label className="field"><span>Mahlzeit</span><select value={mealType} onChange={(event) => setMealType(event.target.value as MealType)}>{Object.entries(mealLabels).map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label>
          {entryMode === "recipe" ? <><fieldset className="recipe-picker"><legend>Rezept auswählen</legend>{recipes.map((recipe) => <RecipeChoice key={recipe.id} recipe={recipe} selected={recipe.id === selectedRecipe} onSelect={() => setSelectedRecipe(recipe.id)} />)}</fieldset><div className="serving-stepper"><span>Portionen</span><div><IconButton label="Eine Portion weniger" onClick={() => setServings(Math.max(1, servings - 1))}><Minus size={15} /></IconButton><strong>{servings}</strong><IconButton label="Eine Portion mehr" onClick={() => setServings(servings + 1)}><Plus size={15} /></IconButton></div></div></> : <label className="field"><span>Was steht an?</span><input autoFocus value={customTitle} onChange={(event) => setCustomTitle(event.target.value)} placeholder="z. B. Abendessen im Restaurant" /></label>}
        </div>
      </Modal>
      <CalendarRecipeDetail recipe={detailRecipe} onClose={() => setDetailRecipe(null)} />
    </div>
  );
}

function PlanCard({ variant, item, onOpen, onRemove }: { variant: "calendar" | "agenda"; item: PlanItem; onOpen: (recipe: Recipe) => void; onRemove: () => void }) {
  const title = item.recipe?.title ?? item.titleOverride ?? item.title ?? "Ohne Rezept";
  const open = () => { if (item.recipe) onOpen(item.recipe); };
  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => { if (!item.recipe || (event.key !== "Enter" && event.key !== " ")) return; event.preventDefault(); open(); };
  return <article className={`plan-card plan-card--${variant} ${item.recipe ? "plan-card--interactive" : ""}`} role={item.recipe ? "button" : undefined} tabIndex={item.recipe ? 0 : undefined} aria-label={item.recipe ? `${title} öffnen` : undefined} onClick={item.recipe ? open : undefined} onKeyDown={onKeyDown}>
    <div className="plan-card__image"><SafeRecipeImage src={item.recipe?.imageUrl} alt={item.recipe ? `Serviervorschlag für ${item.recipe.title}` : ""} fallback={item.status === "eating_out" ? <MapPin size={22} /> : <CookingPot size={22} />} /></div>
    <div className="plan-card__content"><span className="plan-card__type">{item.status === "eating_out" ? "Auswärts" : mealLabels[item.mealType]}</span><h3>{title}</h3>{item.recipe && <div className="plan-card__meta"><span><Clock3 size={13} />{item.recipe.prepMinutes + item.recipe.cookMinutes} Min.</span><span><UsersRound size={13} />{item.servings}</span></div>}{variant === "agenda" && item.recipe?.description && <p className="plan-card__description">{item.recipe.description}</p>}{variant === "agenda" && item.recipe && <NutritionStrip nutrition={item.recipe.nutrition} compact />}{item.note && <p className="plan-card__note">{item.note}</p>}<div className="plan-card__actions"><span>{item.recipe ? "Details öffnen" : ""}</span><IconButton label={`${title} entfernen`} onClick={(event) => { event.stopPropagation(); onRemove(); }}><Trash2 size={14} /></IconButton></div></div>
  </article>;
}

function CalendarRecipeDetail({ recipe, onClose }: { recipe: Recipe | null; onClose: () => void }) {
  if (!recipe) return null;
  return <Modal open onClose={onClose} title={recipe.title} description="Rezeptdetails aus deinem Kalender" size="large" footer={<Button tone="primary" onClick={onClose}>Fertig</Button>}><div className="recipe-detail"><div className="recipe-detail__hero"><SafeRecipeImage src={recipe.imageUrl} alt={`Serviervorschlag für ${recipe.title}`} fallback={<span><CookingPot size={36} /></span>} /><div>{recipe.tags.map((tag) => <em key={tag}>{tag}</em>)}</div></div><p className="recipe-detail__lead">{recipe.description}</p><div className="recipe-detail__facts"><span><Clock3 size={16} /><strong>{recipe.prepMinutes + recipe.cookMinutes} Min.</strong><small>{recipe.prepMinutes} Min. Vorbereitung</small></span><span><UsersRound size={16} /><strong>{recipe.servings} Portionen</strong><small>einfach skalierbar</small></span></div><NutritionStrip nutrition={recipe.nutrition} /><div className="recipe-detail__columns"><section><h3>Zutaten</h3><ul className="ingredient-list">{recipe.ingredients.map((ingredient) => <li key={ingredient.id}><span>{ingredient.name}{ingredient.optional && <small>optional</small>}</span><strong>{formatAmount(ingredient.amount)} {ingredient.unit}</strong></li>)}</ul></section><section><h3>Zubereitung</h3><ol className="step-list">{recipe.steps.map((step, index) => <li key={`${index}-${step.slice(0, 8)}`}><span>{index + 1}</span><p>{step}</p></li>)}</ol></section></div>{recipe.sourceUrl && <ExternalLink className="source-link" href={recipe.sourceUrl}><ExternalLinkIcon size={15} />Quelle: {recipe.sourceName ?? recipe.sourceUrl}</ExternalLink>}</div></Modal>;
}

function RecipeChoice({ recipe, selected, onSelect }: { recipe: Recipe; selected: boolean; onSelect: () => void }) {
  return <button type="button" className={selected ? "is-selected" : ""} onClick={onSelect}><SafeRecipeImage src={recipe.imageUrl} alt={`Serviervorschlag für ${recipe.title}`} fallback={<CookingPot size={19} />} /><span><strong>{recipe.title}</strong><small>{recipe.prepMinutes + recipe.cookMinutes} Min. · {recipe.nutrition.protein} g Protein</small></span>{selected && <span className="selection-dot" />}</button>;
}
