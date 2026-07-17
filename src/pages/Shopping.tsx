import { useEffect, useMemo, useState } from "react";
import { format, parseISO } from "date-fns";
import { de } from "date-fns/locale";
import { CheckCheck, ListRestart, Plus, ShoppingBasket, Trash2 } from "lucide-react";
import { Button, EmptyState, IconButton, PageHeader, Skeleton, formatAmount } from "../components/Common";
import { DateRangePicker, nextMondayRange } from "../components/DateRangePicker";
import { useAppStore } from "../store";
import type { ShoppingItem } from "../types";

const categoryOrder = ["Obst & Gemüse", "Fleisch & Fisch", "Kühlregal", "Konserven", "Vorrat", "Getränke", "Sonstiges"];

export function Shopping() {
  const shopping = useAppStore((state) => state.shopping);
  const recipes = useAppStore((state) => state.recipes);
  const rebuild = useAppStore((state) => state.rebuildShopping);
  const loadShoppingRange = useAppStore((state) => state.loadShoppingRange);
  const toggle = useAppStore((state) => state.toggleShopping);
  const add = useAppStore((state) => state.addShopping);
  const remove = useAppStore((state) => state.deleteShopping);
  const [showDone, setShowDone] = useState(true);
  const [name, setName] = useState("");
  const [amount, setAmount] = useState(1);
  const [unit, setUnit] = useState("Stk.");
  const initialRange = nextMondayRange();
  const [startDate, setStartDate] = useState(initialRange.start);
  const [endDate, setEndDate] = useState(initialRange.end);
  const [rangeLoading, setRangeLoading] = useState(true);
  useEffect(() => {
    let active = true;
    setRangeLoading(true);
    void loadShoppingRange(startDate, endDate).catch(() => undefined).finally(() => { if (active) setRangeLoading(false); });
    return () => { active = false; };
  }, [endDate, loadShoppingRange, startDate]);
  const visible = showDone ? shopping : shopping.filter((item) => !item.checked);
  const grouped = useMemo(() => {
    const entries = new Map<string, ShoppingItem[]>();
    visible.forEach((item) => entries.set(item.category, [...(entries.get(item.category) ?? []), item]));
    return [...entries.entries()].sort(([a], [b]) => {
      const ai = categoryOrder.indexOf(a);
      const bi = categoryOrder.indexOf(b);
      return (ai < 0 ? 99 : ai) - (bi < 0 ? 99 : bi);
    });
  }, [visible]);
  const done = shopping.filter((item) => item.checked).length;
  const addManual = async () => {
    if (!name.trim()) return;
    if (!startDate || !endDate || endDate < startDate || !Number.isFinite(amount) || amount <= 0) return;
    await add({ id: "", name: name.trim(), amount, unit, category: "Sonstiges", checked: false, manual: true, recipeIds: [] }, startDate, endDate);
    setName(""); setAmount(1);
  };

  return (
    <div className="page page--shopping">
      <PageHeader title="Einkaufsliste" description="Aus deinem Plan berechnet, manuell ergänzbar und im Laden schnell abhackbar." actions={<Button tone="primary" icon={<ListRestart size={16} />} disabled={!startDate || !endDate || endDate < startDate} onClick={() => rebuild(startDate, endDate)}>Neu berechnen</Button>} />
      <section className="shopping-summary">
        <DateRangePicker start={startDate} end={endDate} onChange={(range) => { setStartDate(range.start); setEndDate(range.end); }} label="Einkaufszeitraum" />
        <div className="shopping-summary__progress"><span className="shopping-summary__icon"><ShoppingBasket size={19} /></span><div><strong>{shopping.length - done} Artikel offen</strong><small>{startDate && endDate ? `${format(parseISO(startDate), "d. MMM", { locale: de })} bis ${format(parseISO(endDate), "d. MMM", { locale: de })}` : "Zeitraum auswählen"}</small></div><span>{done}/{shopping.length}</span></div>
        <label className="switch"><input type="checkbox" checked={showDone} onChange={(event) => setShowDone(event.target.checked)} /><span />Erledigtes zeigen</label>
      </section>

      <section className="manual-item-form">
        <Plus size={18} />
        <input aria-label="Artikelname" value={name} onChange={(event) => setName(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") addManual(); }} placeholder="Weiteren Artikel hinzufügen …" />
        <input aria-label="Menge" type="number" min="0.1" step="0.1" value={amount} onChange={(event) => setAmount(Number(event.target.value))} />
        <select aria-label="Einheit" value={unit} onChange={(event) => setUnit(event.target.value)}><option>Stk.</option><option>g</option><option>kg</option><option>ml</option><option>l</option><option>Packung</option></select>
        <Button tone="secondary" onClick={addManual} disabled={!name.trim()}>Hinzufügen</Button>
      </section>

      {rangeLoading ? <div className="shopping-range-loading" aria-live="polite"><span>Liste für den gewählten Zeitraum wird geladen …</span><Skeleton lines={4} /></div> : grouped.length ? (
        <div className="shopping-groups">
          {grouped.map(([category, items]) => (
            <section key={category} className="shopping-group">
              <header><h2>{category}</h2><span>{items.filter((item) => !item.checked).length} offen</span></header>
              <div className="shopping-group__items">
                {items.map((item) => {
                  const recipeNames = item.recipeIds.map((id) => recipes.find((recipe) => recipe.id === id)?.title).filter(Boolean);
                  return (
                    <div className={`shopping-row ${item.checked ? "is-checked" : ""}`} key={item.id}>
                      <label><input type="checkbox" aria-label={`${item.name} als ${item.checked ? "offen" : "erledigt"} markieren`} checked={item.checked} onChange={(event) => toggle(item.id, event.target.checked)} /><span className="checkmark" /></label>
                      <div><strong>{item.name}</strong><small>{item.manual ? "Manuell hinzugefügt" : recipeNames.join(" · ") || "Aus dem Wochenplan"}</small></div>
                      <span className="shopping-row__amount">{formatAmount(item.amount)} {item.unit}</span>
                      <IconButton label={`${item.name} löschen`} onClick={() => remove(item.id)}><Trash2 size={15} /></IconButton>
                    </div>
                  );
                })}
              </div>
            </section>
          ))}
        </div>
      ) : (
        <EmptyState icon={<CheckCheck size={25} />} title={shopping.length ? "Alles erledigt" : "Noch nichts auf der Liste"} action={!shopping.length ? <Button tone="primary" onClick={() => rebuild(startDate, endDate)}>Aus Wochenplan erstellen</Button> : <Button onClick={() => setShowDone(true)}>Erledigtes anzeigen</Button>}><p>{shopping.length ? "Du bist startklar. Erledigte Artikel bleiben bis zur nächsten Neuberechnung erhalten." : "Plane Gerichte ein und MealZ fasst die Zutaten automatisch zusammen."}</p></EmptyState>
      )}
    </div>
  );
}
