import { format, isSameDay, parseISO } from "date-fns";
import { de } from "date-fns/locale";
import { ArrowRight, CalendarDays, CheckCircle2, Clock3, CookingPot, MessageCircleMore, ShoppingBasket, Sparkles, Utensils } from "lucide-react";
import { Button, EmptyState, NutritionStrip, PageHeader, SafeRecipeImage } from "../components/Common";
import { useAppStore } from "../store";

export function Dashboard() {
  const plan = useAppStore((state) => state.plan);
  const shopping = useAppStore((state) => state.shopping);
  const profile = useAppStore((state) => state.profile);
  const recipes = useAppStore((state) => state.recipes);
  const setView = useAppStore((state) => state.setView);
  const setSelectedRecipeId = useAppStore((state) => state.setSelectedRecipeId);
  const today = new Date();
  const todaysMeals = plan.filter((item) => isSameDay(parseISO(item.date), today));
  const primary = todaysMeals[0];
  const checked = shopping.filter((item) => item.checked).length;
  const recommendation = [...recipes].filter((recipe) => recipe.favorite || recipe.rating && recipe.rating >= 4).sort((a, b) => new Date(a.lastCookedAt ?? 0).getTime() - new Date(b.lastCookedAt ?? 0).getTime())[0];
  const totals = todaysMeals.reduce((sum, item) => {
    const factor = item.recipe ? item.servings / item.recipe.servings : 0;
    return {
      calories: sum.calories + (item.recipe?.nutrition.calories ?? 0) * factor,
      protein: sum.protein + (item.recipe?.nutrition.protein ?? 0) * factor,
      carbs: sum.carbs + (item.recipe?.nutrition.carbs ?? 0) * factor,
      fat: sum.fat + (item.recipe?.nutrition.fat ?? 0) * factor,
      fiber: sum.fiber + (item.recipe?.nutrition.fiber ?? 0) * factor,
    };
  }, { calories: 0, protein: 0, carbs: 0, fat: 0, fiber: 0 });

  return (
    <div className="page page--dashboard">
      <PageHeader
        title={`Guten ${today.getHours() < 12 ? "Morgen" : today.getHours() < 18 ? "Tag" : "Abend"}, ${profile.name}.`}
        description={format(today, "EEEE, d. MMMM", { locale: de }) + " · Was heute in deiner Küche ansteht"}
        actions={<Button tone="primary" icon={<Sparkles size={16} />} onClick={() => setView("agent")}>Mila fragen</Button>}
      />

      <section className="dashboard-hero">
        {primary?.recipe ? (
          <>
            <div className="dashboard-hero__image">
              <SafeRecipeImage src={primary.recipe.imageUrl} alt={`Serviervorschlag für ${primary.recipe.title}`} fallback={<span><CookingPot size={32} /></span>} />
              <div className="meal-badge"><Utensils size={14} />Heute Abend</div>
            </div>
            <div className="dashboard-hero__content">
              <div className="dashboard-hero__meta"><span><Clock3 size={15} />{primary.recipe.prepMinutes + primary.recipe.cookMinutes} Min.</span><span>{primary.servings} Portionen</span></div>
              <h2>{primary.recipe.title}</h2>
              <p>{primary.recipe.description}</p>
              <NutritionStrip nutrition={primary.recipe.nutrition} compact />
              <div className="action-row">
                <Button tone="primary" icon={<CookingPot size={16} />} onClick={() => setView("recipes")}>Rezept öffnen</Button>
                <Button tone="quiet" icon={<MessageCircleMore size={16} />} onClick={() => setView("agent")}>Änderung besprechen</Button>
              </div>
            </div>
          </>
        ) : (
          <EmptyState icon={<CalendarDays size={24} />} title="Heute ist noch frei" action={<Button tone="primary" onClick={() => setView("week")}>Gericht einplanen</Button>}>
            <p>Wähle selbst ein Rezept oder lass Mila etwas Passendes vorschlagen.</p>
          </EmptyState>
        )}
      </section>

      <section className="dashboard-grid">
        <article className="metric-panel">
          <header><div><h3>Tagesrahmen</h3><p>Nur geplante Mahlzeiten</p></div><span className="metric-panel__status">{Math.round(totals.calories)} / {profile.calorieTarget} kcal</span></header>
          <div className="target-bars">
            <TargetBar label="Kalorien" value={totals.calories} target={profile.calorieTarget} unit="kcal" />
            <TargetBar label="Protein" value={totals.protein} target={profile.proteinTarget} unit="g" />
            <TargetBar label="Ballaststoffe" value={totals.fiber} target={profile.fiberTarget} unit="g" />
          </div>
          <button className="text-link" onClick={() => setView("settings")}>Ziele anpassen <ArrowRight size={14} /></button>
        </article>

        <article className="shopping-progress">
          <header><div><h3>Einkauf</h3><p>Aktuelle Wochenliste</p></div><ShoppingBasket size={20} /></header>
          <div className="progress-donut" style={{ "--progress": `${shopping.length ? (checked / shopping.length) * 360 : 0}deg` } as React.CSSProperties}>
            <span><strong>{shopping.length - checked}</strong><small>offen</small></span>
          </div>
          <p>{checked} von {shopping.length} Artikeln erledigt</p>
          <Button tone="secondary" onClick={() => setView("shopping")}>Liste öffnen</Button>
        </article>

        <article className="mila-note">
          <div className="mila-note__avatar"><Sparkles size={19} /></div>
          <div><h3>Eine Idee von {profile.agentName || "deinem Agenten"}</h3><p>{recommendation ? `${recommendation.title}${recommendation.lastCookedAt ? ` wurde zuletzt am ${format(new Date(recommendation.lastCookedAt), "d. MMMM", { locale: de })} gekocht` : " ist in deiner Sammlung"}. Das könnte gut wieder in deinen Plan passen.` : "Bewerte gespeicherte Gerichte, damit ich passende Wiederholungen vorschlagen kann."}</p></div>
          {recommendation ? <button className="text-link" onClick={() => { setSelectedRecipeId(recommendation.id); setView("recipes"); }}>Rezept ansehen <ArrowRight size={14} /></button> : <button className="text-link" onClick={() => setView("agent")}>Vorschlag besprechen <ArrowRight size={14} /></button>}
        </article>
      </section>

      <section className="upcoming">
        <header><div><h2>Als Nächstes</h2><p>Dein Plan bis zum Wochenende</p></div><button className="text-link" onClick={() => setView("week")}>Ganze Woche <ArrowRight size={14} /></button></header>
        <div className="upcoming__list">
          {plan.filter((item) => parseISO(item.date) >= today).slice(0, 4).map((item) => (
            <button key={item.id} onClick={() => setView("week")}>
              <time dateTime={item.date}><strong>{format(parseISO(item.date), "EEE", { locale: de })}</strong><span>{format(parseISO(item.date), "dd.MM.")}</span></time>
              <span className="upcoming__thumb"><SafeRecipeImage src={item.recipe?.imageUrl} alt={item.recipe ? `Serviervorschlag für ${item.recipe.title}` : ""} fallback={<CookingPot size={18} />} /></span>
              <span className="upcoming__name"><strong>{item.recipe?.title ?? "Freier Slot"}</strong><small>{item.recipe ? `${item.recipe.prepMinutes + item.recipe.cookMinutes} Min. · ${item.servings} Portionen` : "Noch nicht geplant"}</small></span>
              {item.status === "prepared" ? <CheckCircle2 size={18} className="success-icon" /> : <ArrowRight size={17} />}
            </button>
          ))}
        </div>
      </section>
    </div>
  );
}

function TargetBar({ label, value, target, unit }: { label: string; value: number; target: number; unit: string }) {
  const percentage = Math.min(100, target ? (value / target) * 100 : 0);
  return (
    <div className="target-bar">
      <div><span>{label}</span><strong>{Math.round(value)} <small>/ {target} {unit}</small></strong></div>
      <span className="target-bar__track"><i style={{ width: `${percentage}%` }} /></span>
    </div>
  );
}
