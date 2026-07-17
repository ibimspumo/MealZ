import { useMemo, useState } from "react";
import { addDays, differenceInCalendarDays, format, isSameDay, parseISO, startOfWeek } from "date-fns";
import { de } from "date-fns/locale";
import { ArrowRight, CalendarPlus, ChevronLeft, ChevronRight, Clock3, CookingPot, MapPin, Minus, Plus, Sparkles, Trash2, UsersRound } from "lucide-react";
import { Button, EmptyState, IconButton, Modal, PageHeader, SafeRecipeImage } from "../components/Common";
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
  const start = parseISO(weekStart);
  const end = parseISO(weekEnd);
  const days = useMemo(() => Array.from({ length: differenceInCalendarDays(end, start) + 1 }, (_, index) => addDays(start, index)), [end, start]);
  const rangeDays = days.length;

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
                  <PlanCard item={item} onOpen={() => setView("recipes")} onRemove={() => removePlanItem(item.id)} key={item.id} />
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
        return <article className={`agenda-day ${today ? "agenda-day--today" : ""}`} key={dateKey}><header><div><span>{format(date, "EEEE", { locale: de })}</span><h2>{format(date, "d. MMMM", { locale: de })}</h2></div>{today && <strong>Heute</strong>}<Button tone="quiet" icon={<Plus size={15} />} onClick={() => beginEditing(dateKey)}>Eintrag</Button></header><div className="agenda-day__items">{items.length ? items.map((item) => <PlanCard item={item} onOpen={() => setView("recipes")} onRemove={() => removePlanItem(item.id)} key={item.id} />) : <button className="agenda-empty" onClick={() => beginEditing(dateKey)}><Plus size={16} />Gericht oder Termin hinzufügen</button>}</div></article>;
      })}</section>}

      {!plan.length && <EmptyState icon={<CookingPot size={25} />} title="Die Woche wartet auf Ideen" action={<Button tone="primary" onClick={askForWeek}>Mit Mila planen</Button>}><p>Lass sieben passende Hauptgerichte erstellen oder trage selbst deine Favoriten ein.</p></EmptyState>}

      <Modal open={Boolean(editingDay)} onClose={() => setEditingDay(null)} title="Kalendereintrag planen" description={editingDay ? format(parseISO(editingDay), "EEEE, d. MMMM", { locale: de }) : undefined} footer={<><Button tone="quiet" onClick={() => setEditingDay(null)}>Abbrechen</Button><Button tone="primary" onClick={addMeal} disabled={entryMode === "recipe" ? !selectedRecipe : !customTitle.trim()}>Einplanen</Button></>}>
        <div className="form-stack">
          <div className="segmented plan-entry-mode" role="group" aria-label="Art des Kalendereintrags"><button type="button" aria-pressed={entryMode === "recipe"} className={entryMode === "recipe" ? "is-active" : ""} onClick={() => setEntryMode("recipe")}>Rezept</button><button type="button" aria-pressed={entryMode === "eating_out"} className={entryMode === "eating_out" ? "is-active" : ""} onClick={() => setEntryMode("eating_out")}>Auswärts</button></div>
          <label className="field"><span>Mahlzeit</span><select value={mealType} onChange={(event) => setMealType(event.target.value as MealType)}>{Object.entries(mealLabels).map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label>
          {entryMode === "recipe" ? <><fieldset className="recipe-picker"><legend>Rezept auswählen</legend>{recipes.map((recipe) => <RecipeChoice key={recipe.id} recipe={recipe} selected={recipe.id === selectedRecipe} onSelect={() => setSelectedRecipe(recipe.id)} />)}</fieldset><div className="serving-stepper"><span>Portionen</span><div><IconButton label="Eine Portion weniger" onClick={() => setServings(Math.max(1, servings - 1))}><Minus size={15} /></IconButton><strong>{servings}</strong><IconButton label="Eine Portion mehr" onClick={() => setServings(servings + 1)}><Plus size={15} /></IconButton></div></div></> : <label className="field"><span>Was steht an?</span><input autoFocus value={customTitle} onChange={(event) => setCustomTitle(event.target.value)} placeholder="z. B. Abendessen im Restaurant" /></label>}
        </div>
      </Modal>
    </div>
  );
}

function PlanCard({ item, onOpen, onRemove }: { item: PlanItem; onOpen: () => void; onRemove: () => void }) {
  const title = item.recipe?.title ?? item.titleOverride ?? item.title ?? "Ohne Rezept";
  return <div className="plan-card">
    <div className="plan-card__image"><SafeRecipeImage src={item.recipe?.imageUrl} alt={item.recipe ? `Serviervorschlag für ${item.recipe.title}` : ""} fallback={item.status === "eating_out" ? <MapPin size={22} /> : <CookingPot size={22} />} /></div>
    <div className="plan-card__content"><span className="plan-card__type">{item.status === "eating_out" ? "Auswärts" : mealLabels[item.mealType]}</span><h3>{title}</h3>{item.recipe && <div className="plan-card__meta"><span><Clock3 size={13} />{item.recipe.prepMinutes + item.recipe.cookMinutes} Min.</span><span><UsersRound size={13} />{item.servings}</span></div>}{item.note && <p className="plan-card__note">{item.note}</p>}<div className="plan-card__actions">{item.recipe && <button onClick={onOpen}>Öffnen <ArrowRight size={13} /></button>}<IconButton label={`${title} entfernen`} onClick={onRemove}><Trash2 size={14} /></IconButton></div></div>
  </div>;
}

function RecipeChoice({ recipe, selected, onSelect }: { recipe: Recipe; selected: boolean; onSelect: () => void }) {
  return <button type="button" className={selected ? "is-selected" : ""} onClick={onSelect}><SafeRecipeImage src={recipe.imageUrl} alt={`Serviervorschlag für ${recipe.title}`} fallback={<CookingPot size={19} />} /><span><strong>{recipe.title}</strong><small>{recipe.prepMinutes + recipe.cookMinutes} Min. · {recipe.nutrition.protein} g Protein</small></span>{selected && <span className="selection-dot" />}</button>;
}
