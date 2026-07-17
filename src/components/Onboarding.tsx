import { forwardRef, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Bot,
  Check,
  ChefHat,
  CircleUserRound,
  Clock3,
  Heart,
  LoaderCircle,
  MessageCircleMore,
  Plus,
  Scale,
  Sparkles,
  Trash2,
  Utensils,
} from "lucide-react";
import { Button, IconButton } from "./Common";
import { useAppStore } from "../store";
import type { Profile } from "../types";
import { activityLabels, calculateEnergyTarget } from "../nutrition";

const steps = [
  { title: "Willkommen", short: "Wer du bist", icon: CircleUserRound },
  { title: "Dein Rahmen", short: "Basis & Nährwerte", icon: Scale },
  { title: "Dein Alltag", short: "Zeit & Budget", icon: Clock3 },
  { title: "Deine Küche", short: "Vorhandene Geräte", icon: Utensils },
  { title: "Dein Geschmack", short: "Favoriten & No-Gos", icon: Heart },
  { title: "Dein Agent", short: "Name & Persönlichkeit", icon: Bot },
  { title: "Alles bereit", short: "Prüfen & loslegen", icon: Check },
] as const;

export function Onboarding() {
  const profile = useAppStore((state) => state.profile);
  const completeOnboarding = useAppStore((state) => state.completeOnboarding);
  const continueInChat = useAppStore((state) => state.continueOnboardingInChat);
  const [draft, setDraft] = useState<Profile>(() => structuredClone(profile));
  const [step, setStep] = useState(0);
  const [briefing, setBriefing] = useState("");
  const [equipmentName, setEquipmentName] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const headingRef = useRef<HTMLHeadingElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  const update = <K extends keyof Profile>(key: K, value: Profile[K]) => setDraft((current) => ({ ...current, [key]: value }));
  const progress = ((step + 1) / steps.length) * 100;

  useEffect(() => { headingRef.current?.focus(); }, [step]);
  useEffect(() => {
    const shell = document.querySelector<HTMLElement>(".app-shell");
    if (shell) { shell.inert = true; shell.setAttribute("aria-hidden", "true"); }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Tab" || !dialogRef.current) return;
      const nodes = [...dialogRef.current.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled]), textarea:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])')];
      if (!nodes.length) return;
      const first = nodes[0]; const last = nodes[nodes.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => { document.removeEventListener("keydown", onKeyDown); if (shell) { shell.inert = false; shell.removeAttribute("aria-hidden"); } };
  }, []);

  const validation = useMemo(() => {
    if (step === 0 && !draft.name.trim()) return "Bitte verrate uns, wie Mila dich ansprechen soll.";
    if (step === 1 && (draft.calorieTarget <= 0 || draft.proteinTarget <= 0 || draft.fiberTarget <= 0)) return "Die drei Nährwertrahmen müssen größer als null sein.";
    if (step === 2 && (draft.weekdayMaxMinutes <= 0 || draft.weekendMaxMinutes <= 0)) return "Bitte gib realistische Zeitfenster größer als null an.";
    if (step === 5 && (!draft.agentName.trim() || !draft.agentPersonality.trim())) return "Dein Agent braucht einen Namen und eine kurze Persönlichkeit.";
    return "";
  }, [draft, step]);

  const next = () => {
    if (validation) { setError(validation); return; }
    setError("");
    setStep((current) => Math.min(steps.length - 1, current + 1));
  };
  const back = () => { setError(""); setStep((current) => Math.max(0, current - 1)); };
  const finish = async () => {
    setSaving(true); setError("");
    try { await completeOnboarding(draft, briefing); }
    catch (reason) { setError(`Onboarding konnte nicht gespeichert werden: ${String(reason)}`); }
    finally { setSaving(false); }
  };
  const finishInChat = async () => {
    setSaving(true); setError("");
    try { await continueInChat(draft, briefing); }
    catch (reason) { setError(`Chat konnte nicht gestartet werden: ${String(reason)}`); setSaving(false); }
  };
  const addEquipment = () => {
    if (!equipmentName.trim()) return;
    update("equipment", [...draft.equipment, { id: crypto.randomUUID(), name: equipmentName.trim(), enabled: true }]);
    setEquipmentName("");
  };

  return (
    <div ref={dialogRef} className="onboarding-layer" role="dialog" aria-modal="true" aria-labelledby="onboarding-heading">
      <aside className="onboarding-rail">
        <div className="onboarding-brand"><span><ChefHat size={21} /></span><div><strong>MealZ</strong><small>Deine persönliche Küche</small></div></div>
        <div className="onboarding-rail__intro"><span>Einmal persönlich einrichten</span><h2>Damit Vorschläge wirklich zu dir passen.</h2><p>Alles bleibt lokal, sichtbar und später vollständig bearbeitbar.</p></div>
        <ol aria-label="Onboarding-Fortschritt">
          {steps.map(({ title, short, icon: Icon }, index) => (
            <li key={title} className={index === step ? "is-current" : index < step ? "is-complete" : ""} aria-current={index === step ? "step" : undefined}>
              <button disabled={index > step} onClick={() => index <= step && setStep(index)}>
                <span>{index < step ? <Check size={14} /> : <Icon size={14} />}</span>
                <div><strong>{title}</strong><small>{short}</small></div>
              </button>
            </li>
          ))}
        </ol>
        <div className="onboarding-rail__privacy"><Sparkles size={16} /><p><strong>Codex-only, local-first</strong><span>Keine fremde KI-Laufzeit. Deine Daten bleiben in MealZ.</span></p></div>
      </aside>

      <main className="onboarding-main">
        <header className="onboarding-topbar">
          <div className="onboarding-progress" role="progressbar" aria-label="Onboarding-Fortschritt" aria-valuemin={1} aria-valuemax={steps.length} aria-valuenow={step + 1}><i style={{ width: `${progress}%` }} /></div>
          <span>Schritt {step + 1} von {steps.length}</span>
        </header>
        <div className="onboarding-content">
          <section className="onboarding-step">
            <StepHeading ref={headingRef} step={step} />
            {step === 0 && <WelcomeStep draft={draft} update={update} />}
            {step === 1 && <BasicsStep draft={draft} update={update} onSkip={() => { update("heightCm", undefined); update("weightKg", undefined); setError(""); setStep(2); }} />}
            {step === 2 && <RoutineStep draft={draft} update={update} />}
            {step === 3 && <EquipmentStep draft={draft} update={update} equipmentName={equipmentName} setEquipmentName={setEquipmentName} addEquipment={addEquipment} />}
            {step === 4 && <TasteStep draft={draft} update={update} />}
            {step === 5 && <AgentStep draft={draft} update={update} />}
            {step === 6 && <SummaryStep draft={draft} briefing={briefing} setBriefing={setBriefing} />}
            {error && <p className="onboarding-error" role="alert">{error}</p>}
          </section>
        </div>
        <footer className="onboarding-actions">
          <div>{step > 0 && <Button tone="quiet" icon={<ArrowLeft size={16} />} onClick={back} disabled={saving}>Zurück</Button>}</div>
          <div>
            {step < steps.length - 1 ? <Button tone="primary" icon={<ArrowRight size={16} />} onClick={next}>Weiter</Button> : <><Button icon={<MessageCircleMore size={16} />} onClick={finishInChat} disabled={saving}>Mit Mila im Chat weiter</Button><Button tone="primary" icon={saving ? <LoaderCircle className="spin" size={16} /> : <Check size={16} />} onClick={finish} disabled={saving}>{saving ? "Wird gespeichert …" : "MealZ einrichten"}</Button></>}
          </div>
        </footer>
      </main>
    </div>
  );
}

const StepHeading = forwardRef<HTMLHeadingElement, { step: number }>(({ step }, ref) => {
  const copy = [
    ["Willkommen bei MealZ.", "Wir starten mit dem Wichtigsten: Wie dürfen MealZ und Mila dich ansprechen?"],
    ["Ein sinnvoller Rahmen, kein Ernährungskorsett.", "Körperdaten sind freiwillig. Die Nährwerte dienen nur als editierbare Orientierung für deine Essensplanung."],
    ["So sieht dein echter Kochalltag aus.", "MealZ plant schnelle Arbeitsabende anders als entspannte Wochenenden."],
    ["Was steht in deiner Küche?", "Aktive Geräte dürfen in Rezeptvorschlägen und Zubereitungsschritten verwendet werden."],
    ["Was begeistert dich – und was eher nicht?", "Ein paar ehrliche Hinweise sind wertvoller als eine lange, starre Verbotsliste."],
    ["Gib deinem Agenten eine Persönlichkeit.", "Name, Ton und Autonomie machen aus einem Chat deinen persönlichen Meal-Planning-Begleiter."],
    ["Das ist dein persönlicher Ausgangspunkt.", "Prüfe die wichtigsten Angaben und ergänze optional alles, was Mila von Anfang an wissen sollte."],
  ][step];
  return <header className="onboarding-step__header"><span>{steps[step].title}</span><h1 id="onboarding-heading" ref={ref} tabIndex={-1}>{copy[0]}</h1><p>{copy[1]}</p></header>;
});
StepHeading.displayName = "StepHeading";

function WelcomeStep({ draft, update }: StepProps) {
  return <div className="onboarding-form onboarding-welcome"><label className="field field--large"><span>Dein Name</span><input autoFocus value={draft.name} onChange={(event) => update("name", event.target.value)} placeholder="Wie dürfen wir dich ansprechen?" autoComplete="name" /></label><div className="onboarding-principles"><article><span><ChefHat size={18} /></span><div><strong>Rezepte, die bleiben</strong><p>Generieren, recherchieren, bewerten und wiederverwenden.</p></div></article><article><span><Sparkles size={18} /></span><div><strong>Ein Agent, der lernt</strong><p>Vorlieben und Feedback bleiben transparent editierbar.</p></div></article></div></div>;
}

function BasicsStep({ draft, update, onSkip }: StepProps & { onSkip: () => void }) {
  const calculated = calculateEnergyTarget(draft);
  const useCalculated = () => { if (calculated) { update("calorieTargetMode", "calculated"); update("calorieTarget", calculated.calories); } };
  return <div className="onboarding-form"><div className="form-grid form-grid--2"><OnboardingNumber label="Größe" value={draft.heightCm} suffix="cm" optional onChange={(value) => update("heightCm", value)} /><OnboardingNumber label="Gewicht" value={draft.weightKg} suffix="kg" optional onChange={(value) => update("weightKg", value)} /></div><div className="form-grid form-grid--3"><label className="field"><span>Geburtsdatum <small>optional</small></span><input type="date" value={draft.birthDate ?? ""} onChange={(event) => update("birthDate", event.target.value || undefined)} /></label><label className="field"><span>Geschlecht für Formel <small>optional</small></span><select value={draft.sexForEnergy ?? ""} onChange={(event) => update("sexForEnergy", (event.target.value || undefined) as Profile["sexForEnergy"])}><option value="">Bitte wählen</option><option value="male">Männlich</option><option value="female">Weiblich</option></select></label><label className="field"><span>Aktivitätsniveau</span><select value={draft.activityLevel} onChange={(event) => update("activityLevel", event.target.value as Profile["activityLevel"])}>{Object.entries(activityLabels).map(([value, label]) => <option key={value} value={value}>{label.title}</option>)}</select></label></div><div className="onboarding-divider"><span>Dein täglicher Orientierungsrahmen</span></div><div className="energy-mode" role="group" aria-label="Kalorienziel wählen"><button type="button" className={draft.calorieTargetMode === "calculated" ? "is-selected" : ""} aria-pressed={draft.calorieTargetMode === "calculated"} disabled={!calculated} onClick={useCalculated}><strong>Berechnet</strong><small>{calculated ? `${calculated.calories.toLocaleString("de-DE")} kcal pro Tag` : "Daten ergänzen"}</small></button><button type="button" className={draft.calorieTargetMode === "manual" ? "is-selected" : ""} aria-pressed={draft.calorieTargetMode === "manual"} onClick={() => update("calorieTargetMode", "manual")}><strong>Manuell</strong><small>Eigenen Zielwert festlegen</small></button></div><div className="onboarding-targets"><OnboardingNumber label="Kalorien" value={draft.calorieTarget} suffix="kcal" onChange={(value) => { update("calorieTargetMode", "manual"); update("calorieTarget", value ?? 0); }} /><OnboardingNumber label="Protein" value={draft.proteinTarget} suffix="g" onChange={(value) => update("proteinTarget", value ?? 0)} /><OnboardingNumber label="Ballaststoffe" value={draft.fiberTarget} suffix="g" onChange={(value) => update("fiberTarget", value ?? 0)} /></div><p className="onboarding-note">{calculated ? `NASEM DRI 2023, Table S-3: geschätzter Erhaltungsbedarf für ${calculated.age} Jahre. Das ist eine Orientierung und kann individuell um etwa 190 bis 340 kcal pro Tag abweichen.` : "Körperdaten sind freiwillig. Die Berechnung nutzt bei vollständigen Angaben die NASEM-DRI-EER-Formel 2023 für Erwachsene. Sie ist keine medizinische Empfehlung."}</p><button className="onboarding-skip" onClick={onSkip}>Körperdaten überspringen und weiter</button></div>;
}

function RoutineStep({ draft, update }: StepProps) {
  return <div className="onboarding-form"><div className="onboarding-time-cards"><TimeChoice title="Montag bis Freitag" copy="Meistens erst abends zu Hause" value={draft.weekdayMaxMinutes} onChange={(value) => update("weekdayMaxMinutes", value)} options={[25, 35, 45, 60]} /><TimeChoice title="Am Wochenende" copy="Mehr Zeit für besondere Gerichte" value={draft.weekendMaxMinutes} onChange={(value) => update("weekendMaxMinutes", value)} options={[45, 60, 90, 120]} /></div><fieldset className="onboarding-choice-group"><legend>Budgetgefühl</legend>{(["sparsam", "ausgewogen", "flexibel"] as const).map((value) => <button key={value} type="button" className={draft.budgetPreference === value ? "is-selected" : ""} onClick={() => update("budgetPreference", value)}><span className="radio-dot" /><strong>{value === "sparsam" ? "Preisbewusst" : value === "ausgewogen" ? "Ausgewogen" : "Flexibel"}</strong><small>{value === "sparsam" ? "Günstige Zutaten bevorzugen" : value === "ausgewogen" ? "Preis, Qualität und Abwechslung balancieren" : "Auch besondere Zutaten sind willkommen"}</small></button>)}</fieldset><label className="field"><span>So koche ich gern <small>optional</small></span><textarea rows={4} value={draft.cookingStyle} onChange={(event) => update("cookingStyle", event.target.value)} placeholder="Zum Beispiel: Unter der Woche einfach, am Wochenende gerne aufwendiger …" /></label></div>;
}

function EquipmentStep({ draft, update, equipmentName, setEquipmentName, addEquipment }: StepProps & { equipmentName: string; setEquipmentName: (value: string) => void; addEquipment: () => void }) {
  return <div className="onboarding-form"><div className="onboarding-equipment">{draft.equipment.map((equipment) => <div key={equipment.id} className={equipment.enabled ? "is-active" : ""}><label><input type="checkbox" checked={equipment.enabled} onChange={(event) => update("equipment", draft.equipment.map((item) => item.id === equipment.id ? { ...item, enabled: event.target.checked } : item))} /><span><Utensils size={16} /></span><strong>{equipment.name}</strong></label><IconButton label={`${equipment.name} entfernen`} onClick={() => update("equipment", draft.equipment.filter((item) => item.id !== equipment.id))}><Trash2 size={14} /></IconButton></div>)}</div><div className="onboarding-add-row"><input value={equipmentName} onChange={(event) => setEquipmentName(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); addEquipment(); } }} placeholder="Weiteres Gerät hinzufügen …" aria-label="Weiteres Küchengerät" /><Button icon={<Plus size={15} />} onClick={addEquipment} disabled={!equipmentName.trim()}>Hinzufügen</Button></div></div>;
}

function TasteStep({ draft, update }: StepProps) {
  return <div className="onboarding-form onboarding-taste"><TasteEditor title="Das esse ich besonders gern" copy="Favoriten dürfen häufiger wiederkommen." values={draft.favorites} onChange={(values) => update("favorites", values)} placeholder="z. B. Lasagne, Hähnchen, Lachs" tone="positive" /><TasteEditor title="Das esse ich eher ungern" copy="Kein hartes Verbot – Mila bevorzugt Alternativen." values={draft.dislikes} onChange={(values) => update("dislikes", values)} placeholder="z. B. Karotten" tone="caution" /><p className="onboarding-note">Später kannst du im Memory-Center genauer festhalten: „esse ich, aber nicht besonders gern“ oder eine Vorliebe mit Kontext bewerten.</p></div>;
}

function AgentStep({ draft, update }: StepProps) {
  return <div className="onboarding-form"><div className="onboarding-agent-preview"><span><Sparkles size={24} /></span><div><strong>{draft.agentName || "Dein Agent"}</strong><p>{draft.agentPersonality || "Gib deinem persönlichen Meal-Planning-Agenten einen Charakter."}</p></div></div><label className="field field--large"><span>Name deines Agenten</span><input value={draft.agentName} onChange={(event) => update("agentName", event.target.value)} placeholder="z. B. Mila" /></label><label className="field"><span>Persönlichkeit & Ton</span><textarea rows={4} value={draft.agentPersonality} onChange={(event) => update("agentPersonality", event.target.value)} placeholder="Direkt, aufmerksam und pragmatisch …" /></label><fieldset className="onboarding-choice-group onboarding-choice-group--vertical"><legend>Autonomie</legend>{(["vorsichtig", "ausgewogen", "autonom"] as const).map((value) => <button type="button" key={value} className={draft.autonomy === value ? "is-selected" : ""} onClick={() => update("autonomy", value)}><span className="radio-dot" /><strong>{value === "vorsichtig" ? "Erst fragen" : value === "ausgewogen" ? "Ausgewogen" : "Sehr selbstständig"}</strong><small>{value === "vorsichtig" ? "Jede Änderung wird zuerst bestätigt." : value === "ausgewogen" ? "Sichere Anpassungen direkt, größere Änderungen als Vorschlag." : "Pläne und Listen eigenständig optimieren – nachvollziehbar und rückgängig machbar."}</small></button>)}</fieldset></div>;
}

function SummaryStep({ draft, briefing, setBriefing }: { draft: Profile; briefing: string; setBriefing: (value: string) => void }) {
  const activeEquipment = draft.equipment.filter((item) => item.enabled).length;
  return <div className="onboarding-form"><div className="onboarding-summary"><SummaryItem label="Profil" value={`${draft.name}${draft.heightCm ? ` · ${draft.heightCm} cm` : ""}${draft.weightKg ? ` · ${draft.weightKg} kg` : ""}`} /><SummaryItem label="Nährwertrahmen" value={`${draft.calorieTarget} kcal · ${draft.proteinTarget} g Protein · ${draft.fiberTarget} g Ballaststoffe`} /><SummaryItem label="Zeit" value={`${draft.weekdayMaxMinutes} Min. werktags · ${draft.weekendMaxMinutes} Min. am Wochenende`} /><SummaryItem label="Küche" value={`${activeEquipment} aktive Geräte · Budget ${draft.budgetPreference}`} /><SummaryItem label="Geschmack" value={`${draft.favorites.length} Favoriten · ${draft.dislikes.length} eher ungern`} /><SummaryItem label="Agent" value={`${draft.agentName} · ${draft.autonomy}`} /></div><label className="field"><span>Was sollte {draft.agentName || "dein Agent"} noch über dich wissen? <small>optional</small></span><textarea rows={5} value={briefing} onChange={(event) => setBriefing(event.target.value)} placeholder="Zum Beispiel besondere Routinen, Zutaten, die gerade aufgebraucht werden sollen, oder was ein perfektes Abendessen für dich ausmacht …" /><small className="field-help">Wird beim Direktabschluss als transparente Erinnerung gespeichert.</small></label></div>;
}

interface StepProps { draft: Profile; update: <K extends keyof Profile>(key: K, value: Profile[K]) => void; }

function OnboardingNumber({ label, value, suffix, optional, onChange }: { label: string; value?: number; suffix: string; optional?: boolean; onChange: (value?: number) => void }) { return <label className="field"><span>{label} {optional && <small>optional</small>}</span><span className="number-input"><input type="number" min="0" value={value ?? ""} onChange={(event) => onChange(event.target.value === "" ? undefined : Number(event.target.value))} placeholder="–" /><small>{suffix}</small></span></label>; }
function TimeChoice({ title, copy, value, onChange, options }: { title: string; copy: string; value: number; onChange: (value: number) => void; options: number[] }) { return <section><div><strong>{title}</strong><small>{copy}</small></div><div>{options.map((option) => <button type="button" key={option} className={value === option ? "is-selected" : ""} onClick={() => onChange(option)}>{option}<small>Min.</small></button>)}</div></section>; }
function TasteEditor({ title, copy, values, onChange, placeholder, tone }: { title: string; copy: string; values: string[]; onChange: (values: string[]) => void; placeholder: string; tone: string }) { const [text, setText] = useState(""); const add = () => { const value = text.trim(); if (value && !values.some((item) => item.toLocaleLowerCase("de") === value.toLocaleLowerCase("de"))) onChange([...values, value]); setText(""); }; return <section className={`taste-editor taste-editor--${tone}`}><header><span><Heart size={16} /></span><div><strong>{title}</strong><small>{copy}</small></div></header><div>{values.map((value) => <span key={value}>{value}<button onClick={() => onChange(values.filter((item) => item !== value))} aria-label={`${value} entfernen`}>×</button></span>)}<input aria-label={title} value={text} onChange={(event) => setText(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); add(); } }} onBlur={add} placeholder={placeholder} /></div></section>; }
function SummaryItem({ label, value }: { label: string; value: string }) { return <div><span><Check size={13} /></span><div><small>{label}</small><strong>{value}</strong></div></div>; }
