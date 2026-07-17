import { useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { addDays, addMonths, endOfMonth, endOfWeek, format, isSameDay, isWithinInterval, parseISO, startOfMonth, startOfWeek } from "date-fns";
import { de } from "date-fns/locale";
import { CalendarDays, ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "./Common";

type Range = { start: string; end: string };

export function nextMondayRange(today = new Date()): Range {
  const next = startOfWeek(addDays(today, 7), { weekStartsOn: 1 });
  return { start: format(next, "yyyy-MM-dd"), end: format(addDays(next, 6), "yyyy-MM-dd") };
}

export function DateRangePicker({ start, end, onChange, label = "Zeitraum" }: { start: string; end: string; onChange: (range: Range) => void; label?: string }) {
  const [open, setOpen] = useState(false);
  const [cursor, setCursor] = useState(() => start ? parseISO(start) : new Date());
  const [pendingStart, setPendingStart] = useState<string | null>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const close = () => { setPendingStart(null); setOpen(false); buttonRef.current?.focus(); };
  useEffect(() => {
    if (!open) return;
    setCursor(start ? parseISO(start) : new Date());
    const outside = (event: MouseEvent) => { if (popoverRef.current && !popoverRef.current.contains(event.target as Node) && !buttonRef.current?.contains(event.target as Node)) close(); };
    const key = (event: globalThis.KeyboardEvent) => { if (event.key === "Escape") { event.preventDefault(); close(); } };
    const focusTimer = window.setTimeout(() => popoverRef.current?.querySelector<HTMLButtonElement>(".is-selected, .is-today, .date-range-picker__calendar button")?.focus(), 0);
    document.addEventListener("mousedown", outside); document.addEventListener("keydown", key);
    return () => { window.clearTimeout(focusTimer); document.removeEventListener("mousedown", outside); document.removeEventListener("keydown", key); };
  }, [open]);
  const select = (day: Date) => {
    const value = format(day, "yyyy-MM-dd");
    if (!pendingStart) { setPendingStart(value); return; }
    const next = value < pendingStart ? { start: value, end: pendingStart } : { start: pendingStart, end: value };
    onChange(next); close();
  };
  const quick = (range: Range) => { onChange(range); close(); };
  const currentWeek = (): Range => { const monday = startOfWeek(new Date(), { weekStartsOn: 1 }); return { start: format(monday, "yyyy-MM-dd"), end: format(addDays(monday, 6), "yyyy-MM-dd") }; };
  const nextSeven = (): Range => { const today = new Date(); return { start: format(today, "yyyy-MM-dd"), end: format(addDays(today, 6), "yyyy-MM-dd") }; };
  const labelText = start && end ? `${format(parseISO(start), "d. MMMM", { locale: de })} bis ${format(parseISO(end), "d. MMMM yyyy", { locale: de })}` : "Zeitraum wählen";
  return <div className="date-range-picker"><button ref={buttonRef} type="button" className="date-range-picker__button" aria-haspopup="dialog" aria-expanded={open} onClick={() => { if (open) close(); else { setPendingStart(null); setOpen(true); } }}><CalendarDays size={16} /><span><small>{label}</small><strong>{labelText}</strong></span></button>{open && <div ref={popoverRef} className="date-range-picker__popover" role="dialog" aria-modal="false" aria-label="Datumsbereich auswählen"><header><Button tone="quiet" aria-label="Vorheriger Monat" onClick={() => setCursor((value) => addMonths(value, -1))}><ChevronLeft size={16} /></Button><strong aria-live="polite">{format(cursor, "MMMM yyyy", { locale: de })}</strong><Button tone="quiet" aria-label="Nächster Monat" onClick={() => setCursor((value) => addMonths(value, 1))}><ChevronRight size={16} /></Button></header><CalendarMonth month={cursor} start={start} end={end} pendingStart={pendingStart} onSelect={select} /><div className="date-range-picker__quick"><button type="button" onClick={() => quick(currentWeek())}>Aktuelle Woche</button><button type="button" onClick={() => quick(nextMondayRange())}>Nächste Woche</button><button type="button" onClick={() => quick(nextSeven())}>Nächste 7 Tage</button></div><p aria-live="polite">{pendingStart ? "Jetzt das Enddatum auswählen." : "Erster Klick: Start. Zweiter Klick: Ende."}</p></div>}</div>;
}

function CalendarMonth({ month, start, end, pendingStart, onSelect }: { month: Date; start: string; end: string; pendingStart: string | null; onSelect: (day: Date) => void }) {
  const days = useMemo(() => {
    const first = startOfWeek(startOfMonth(month), { weekStartsOn: 1 }); const last = endOfWeek(endOfMonth(month), { weekStartsOn: 1 });
    const output: Date[] = []; for (let day = first; day <= last; day = addDays(day, 1)) output.push(day); return output;
  }, [month]);
  const from = pendingStart ?? start; const to = pendingStart ? pendingStart : end;
  const moveFocus = (event: ReactKeyboardEvent<HTMLButtonElement>, day: Date) => {
    const offsets: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7 };
    let target = offsets[event.key] !== undefined ? addDays(day, offsets[event.key]) : event.key === "Home" ? startOfWeek(day, { weekStartsOn: 1 }) : event.key === "End" ? endOfWeek(day, { weekStartsOn: 1 }) : null;
    if (event.key === "PageUp") target = addMonths(day, -1);
    if (event.key === "PageDown") target = addMonths(day, 1);
    if (!target) return;
    event.preventDefault();
    const key = format(target, "yyyy-MM-dd");
    document.querySelector<HTMLButtonElement>(`[data-calendar-date="${key}"]`)?.focus();
  };
  return <div className="date-range-picker__calendar" role="group" aria-label={format(month, "MMMM yyyy", { locale: de })}>{["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"].map((day) => <span aria-hidden="true" key={day}>{day}</span>)}{days.map((day) => { const value = format(day, "yyyy-MM-dd"); const selected = value === from || value === to; const inRange = from && to && isWithinInterval(day, { start: parseISO(from), end: parseISO(to) }); const today = isSameDay(day, new Date()); return <button key={value} data-calendar-date={value} type="button" aria-label={format(day, "EEEE, d. MMMM yyyy", { locale: de })} aria-pressed={selected} aria-current={today ? "date" : undefined} className={`${day.getMonth() !== month.getMonth() ? "is-outside" : ""} ${selected ? "is-selected" : ""} ${inRange ? "is-in-range" : ""} ${today ? "is-today" : ""}`} onKeyDown={(event) => moveFocus(event, day)} onClick={() => onSelect(day)}>{format(day, "d")}</button>; })}</div>;
}
