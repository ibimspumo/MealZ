import { useEffect, useState, type PropsWithChildren, type ReactNode } from "react";
import { Bot, Check, ChefHat, CircleUserRound, Download, FileText, LoaderCircle, Plus, RotateCcw, Save, Settings2, Trash2, Utensils } from "lucide-react";
import { api } from "../bridge";
import { Button, IconButton, PageHeader } from "../components/Common";
import { UpdateCenter } from "../components/UpdateCenter";
import { useAppStore } from "../store";
import type { AgentFiles, Profile } from "../types";
import { activityLabels, calculateEnergyTarget } from "../nutrition";

type SettingsTab = "profile" | "equipment" | "agent" | "updates";

export function Settings() {
  const profile = useAppStore((state) => state.profile);
  const saveProfile = useAppStore((state) => state.saveProfile);
  const restartOnboarding = useAppStore((state) => state.restartOnboarding);
  const [tab, setTab] = useState<SettingsTab>("profile");
  const [draft, setDraft] = useState<Profile>(() => structuredClone(profile));
  const [equipmentName, setEquipmentName] = useState("");
  const [agentFiles, setAgentFiles] = useState<AgentFiles | null>(null);
  const [savedAgentFiles, setSavedAgentFiles] = useState<AgentFiles | null>(null);
  const [agentFilesLoading, setAgentFilesLoading] = useState(false);
  const [agentFilesSaving, setAgentFilesSaving] = useState(false);
  const [agentFilesError, setAgentFilesError] = useState("");
  const update = <K extends keyof Profile>(key: K, value: Profile[K]) => setDraft((current) => ({ ...current, [key]: value }));
  const addEquipment = () => { if (equipmentName.trim()) { update("equipment", [...draft.equipment, { id: crypto.randomUUID(), name: equipmentName.trim(), enabled: true }]); setEquipmentName(""); } };
  const agentFilesDirty = Boolean(agentFiles && savedAgentFiles && (agentFiles.persona !== savedAgentFiles.persona || agentFiles.memory !== savedAgentFiles.memory));
  useEffect(() => {
    if (tab !== "agent" || agentFiles) return;
    setAgentFilesLoading(true);
    void api.getAgentFiles().then((files) => { setAgentFiles(files); setSavedAgentFiles(structuredClone(files)); setAgentFilesError(""); }).catch((reason: unknown) => setAgentFilesError(`Agent-Dateien konnten nicht geladen werden: ${String(reason)}`)).finally(() => setAgentFilesLoading(false));
  }, [agentFiles, tab]);
  useEffect(() => { setDraft(structuredClone(profile)); }, [profile]);
  const saveFiles = async () => {
    if (!agentFiles) return;
    setAgentFilesSaving(true);
    try { const saved = await api.saveAgentFiles(agentFiles); setAgentFiles(saved); setSavedAgentFiles(structuredClone(saved)); setAgentFilesError(""); }
    catch (reason) { setAgentFilesError(`Agent-Dateien konnten nicht gespeichert werden: ${String(reason)}`); }
    finally { setAgentFilesSaving(false); }
  };

  return (
    <div className="page page--settings">
      <PageHeader title="Einstellungen" description="Dein Alltag, deine Küche und die Persönlichkeit deines Agenten." actions={<Button tone="primary" icon={<Check size={16} />} onClick={() => saveProfile(draft)}>Änderungen speichern</Button>} />
      <div className="settings-layout">
        <nav className="settings-nav" aria-label="Einstellungsbereiche"><button className={tab === "profile" ? "is-active" : ""} onClick={() => setTab("profile")}><CircleUserRound size={18} /><span><strong>Profil & Ziele</strong><small>Alltag und Nährwertrahmen</small></span></button><button className={tab === "equipment" ? "is-active" : ""} onClick={() => setTab("equipment")}><Utensils size={18} /><span><strong>Küchenausstattung</strong><small>Was Mila einplanen darf</small></span></button><button className={tab === "agent" ? "is-active" : ""} onClick={() => setTab("agent")}><Bot size={18} /><span><strong>Agent & Autonomie</strong><small>Persönlichkeit und Verhalten</small></span></button><button className={tab === "updates" ? "is-active" : ""} onClick={() => setTab("updates")}><Download size={18} /><span><strong>Updates & Releases</strong><small>Versionen sicher installieren</small></span></button></nav>
        <div className="settings-content">
          {tab === "profile" && <>
            <SettingsSection icon={<CircleUserRound size={19} />} title="Über dich" description="Diese Angaben helfen bei Portionsgrößen und alltagstauglichen Vorschlägen."><div className="form-grid form-grid--3"><label className="field"><span>Name</span><input value={draft.name} onChange={(event) => update("name", event.target.value)} /></label><NumberSetting label="Größe" value={draft.heightCm} suffix="cm" onChange={(value) => update("heightCm", value)} /><NumberSetting label="Gewicht" value={draft.weightKg} suffix="kg" onChange={(value) => update("weightKg", value)} /></div><label className="field"><span>Dein Kochalltag</span><textarea rows={4} value={draft.cookingStyle} onChange={(event) => update("cookingStyle", event.target.value)} /></label></SettingsSection>
            <EnergySettings draft={draft} update={update} />
            <SettingsSection icon={<ChefHat size={19} />} title="Zeit & Vorlieben" description="Schneller unter der Woche, entspannter am Wochenende."><div className="form-grid form-grid--3"><NumberSetting label="Werktags maximal" value={draft.weekdayMaxMinutes} suffix="Min." onChange={(value) => update("weekdayMaxMinutes", value ?? 0)} /><NumberSetting label="Wochenende maximal" value={draft.weekendMaxMinutes} suffix="Min." onChange={(value) => update("weekendMaxMinutes", value ?? 0)} /><label className="field"><span>Budget</span><select value={draft.budgetPreference} onChange={(event) => update("budgetPreference", event.target.value as Profile["budgetPreference"])}><option value="sparsam">Sparsam</option><option value="ausgewogen">Ausgewogen</option><option value="flexibel">Flexibel</option></select></label></div><TagEditor label="Besonders gern" values={draft.favorites} onChange={(values) => update("favorites", values)} placeholder="z. B. Lasagne" /><TagEditor label="Eher vermeiden" values={draft.dislikes} onChange={(values) => update("dislikes", values)} placeholder="z. B. Karotten" /></SettingsSection>
          </>}
          {tab === "equipment" && <SettingsSection icon={<Utensils size={19} />} title="Deine Küchenausstattung" description="Mila schlägt Zubereitungsarten nur dann vor, wenn das passende Gerät aktiv ist."><div className="equipment-list">{draft.equipment.map((equipment) => <div key={equipment.id}><label className="switch"><input type="checkbox" aria-label={`${equipment.name} ${equipment.enabled ? "deaktivieren" : "aktivieren"}`} checked={equipment.enabled} onChange={(event) => update("equipment", draft.equipment.map((item) => item.id === equipment.id ? { ...item, enabled: event.target.checked } : item))} /><span /></label><strong>{equipment.name}</strong><IconButton label={`${equipment.name} entfernen`} onClick={() => update("equipment", draft.equipment.filter((item) => item.id !== equipment.id))}><Trash2 size={15} /></IconButton></div>)}</div><div className="equipment-add"><input aria-label="Weiteres Küchengerät" value={equipmentName} onChange={(event) => setEquipmentName(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") addEquipment(); }} placeholder="Weiteres Gerät …" /><Button icon={<Plus size={15} />} onClick={addEquipment} disabled={!equipmentName.trim()}>Hinzufügen</Button></div></SettingsSection>}
          {tab === "agent" && <>
            <SettingsSection icon={<Bot size={19} />} title="Persönlichkeit" description={`${draft.agentName || "Dein Agent"} bleibt fokussiert auf Ernährung, Mealprep und deine persönliche Essensplanung.`}><label className="field"><span>Name deines Agenten</span><input value={draft.agentName} onChange={(event) => update("agentName", event.target.value)} /></label><label className="field"><span>Charakter & Ton</span><textarea rows={5} value={draft.agentPersonality} onChange={(event) => update("agentPersonality", event.target.value)} /></label></SettingsSection>
            <SettingsSection icon={<Settings2 size={19} />} title="Autonomie" description={`Wie selbstständig ${draft.agentName || "dein Agent"} mit deinen Plänen und Erinnerungen umgehen darf.`}><div className="autonomy-options">{(["vorsichtig", "ausgewogen", "autonom"] as const).map((value) => <button key={value} aria-pressed={draft.autonomy === value} className={draft.autonomy === value ? "is-selected" : ""} onClick={() => update("autonomy", value)}><span className="radio-dot" /><strong>{value === "vorsichtig" ? "Erst fragen" : value === "ausgewogen" ? "Ausgewogen" : "Sehr selbstständig"}</strong><small>{value === "vorsichtig" ? "Änderungen immer vorab bestätigen" : value === "ausgewogen" ? "Sichere Anpassungen direkt, größere Änderungen als Vorschlag" : "Pläne und Listen eigenständig optimieren, alles bleibt rückgängig machbar"}</small></button>)}</div></SettingsSection>
            <SettingsSection icon={<FileText size={19} />} title="Agent-Dateien" description="Freie Markdown-Anweisungen zusätzlich zum strukturierten Profil und Memory-Center.">
              <div className="agent-files-toolbar"><p>Beide Dateien werden lokal gespeichert und als persönlicher Kontext an relevante Codex-Turns übergeben.</p><span className={agentFilesDirty ? "is-dirty" : ""}>{agentFilesDirty ? "Ungespeicherte Änderungen" : "Gespeichert"}</span></div>
              {agentFilesLoading || !agentFiles ? <div className="agent-files-loading"><LoaderCircle size={17} />Agent-Dateien werden geladen …</div> : <div className="agent-file-editors"><label className="field"><span>PERSONA.md</span><small>Ton, Charakter, Sprachregeln und unveränderliche Verhaltensleitplanken.</small><textarea aria-label="PERSONA.md" className="markdown-editor" rows={14} value={agentFiles.persona} onChange={(event) => setAgentFiles({ ...agentFiles, persona: event.target.value })} /></label><label className="field"><span>MEMORY.md</span><small>Frei formulierter Langzeitkontext zusätzlich zu den einzelnen strukturierten Erinnerungen.</small><textarea aria-label="MEMORY.md" className="markdown-editor" rows={14} value={agentFiles.memory} onChange={(event) => setAgentFiles({ ...agentFiles, memory: event.target.value })} /></label></div>}
              {agentFilesError && <p className="form-error" role="alert">{agentFilesError}</p>}
              <div className="agent-files-actions"><Button icon={<RotateCcw size={15} />} disabled={!agentFilesDirty || agentFilesSaving} onClick={() => savedAgentFiles && setAgentFiles(structuredClone(savedAgentFiles))}>Änderungen verwerfen</Button><Button tone="primary" icon={agentFilesSaving ? <LoaderCircle className="spin" size={15} /> : <Save size={15} />} disabled={!agentFilesDirty || agentFilesSaving} onClick={saveFiles}>{agentFilesSaving ? "Wird gespeichert …" : "Agent-Dateien speichern"}</Button></div>
            </SettingsSection>
            <SettingsSection icon={<RotateCcw size={19} />} title="Onboarding" description="Gehe die persönliche Einrichtung erneut durch, ohne Rezepte, Pläne oder Erinnerungen zu löschen."><div className="settings-onboarding-row"><div><strong>Persönlichen Rahmen neu prüfen</strong><p>Der Wizard startet mit deinen aktuellen Profilwerten.</p></div><Button icon={<RotateCcw size={15} />} onClick={restartOnboarding}>Onboarding erneut starten</Button></div></SettingsSection>
          </>}
          {tab === "updates" && <UpdateCenter />}
        </div>
      </div>
    </div>
  );
}

function SettingsSection({ icon, title, description, children }: PropsWithChildren<{ icon: ReactNode; title: string; description: string }>) { return <section className="settings-section"><header><span>{icon}</span><div><h2>{title}</h2><p>{description}</p></div></header><div className="settings-section__body">{children}</div></section>; }
function NumberSetting({ label, value, suffix, onChange }: { label: string; value?: number; suffix: string; onChange: (value?: number) => void }) { return <label className="field"><span>{label}</span><span className="number-input"><input type="number" min="0" value={value ?? ""} onChange={(event) => onChange(event.target.value === "" ? undefined : Number(event.target.value))} /><small>{suffix}</small></span></label>; }

function EnergySettings({ draft, update }: { draft: Profile; update: <K extends keyof Profile>(key: K, value: Profile[K]) => void }) {
  const calculation = calculateEnergyTarget(draft);
  const chooseCalculated = () => { if (calculation) { update("calorieTargetMode", "calculated"); update("calorieTarget", calculation.calories); } };
  return <SettingsSection icon={<Settings2 size={19} />} title="Nährwertrahmen" description="Orientierung für die Essensplanung, kein medizinisches Tracking.">
    <div className="form-grid form-grid--3"><label className="field"><span>Geburtsdatum <small>für die EER-Berechnung</small></span><input type="date" value={draft.birthDate ?? ""} onChange={(event) => update("birthDate", event.target.value || undefined)} /></label><label className="field"><span>Geschlecht für Formel</span><select value={draft.sexForEnergy ?? ""} onChange={(event) => update("sexForEnergy", (event.target.value || undefined) as Profile["sexForEnergy"])}><option value="">Bitte wählen</option><option value="male">Männlich</option><option value="female">Weiblich</option></select></label><label className="field"><span>Aktivitätsniveau</span><select value={draft.activityLevel} onChange={(event) => update("activityLevel", event.target.value as Profile["activityLevel"])}>{Object.entries(activityLabels).map(([value, label]) => <option key={value} value={value}>{label.title}</option>)}</select></label></div>
    <div className="energy-mode" role="group" aria-label="Kalorienziel wählen"><button type="button" aria-pressed={draft.calorieTargetMode === "calculated"} className={draft.calorieTargetMode === "calculated" ? "is-selected" : ""} onClick={chooseCalculated} disabled={!calculation}><strong>Berechnet</strong><small>{calculation ? `${calculation.calories.toLocaleString("de-DE")} kcal pro Tag` : "Körperdaten ergänzen"}</small></button><button type="button" aria-pressed={draft.calorieTargetMode === "manual"} className={draft.calorieTargetMode === "manual" ? "is-selected" : ""} onClick={() => update("calorieTargetMode", "manual")}><strong>Manuell</strong><small>Eigenen Zielwert verwenden</small></button></div>
    <div className="form-grid form-grid--3"><NumberSetting label="Kalorien" value={draft.calorieTarget} suffix="kcal / Tag" onChange={(value) => update("calorieTarget", value ?? 0)} /><NumberSetting label="Protein" value={draft.proteinTarget} suffix="g / Tag" onChange={(value) => update("proteinTarget", value ?? 0)} /><NumberSetting label="Ballaststoffe" value={draft.fiberTarget} suffix="g / Tag" onChange={(value) => update("fiberTarget", value ?? 0)} /></div>
    <p className="setting-hint">{calculation ? `NASEM DRI 2023, Table S-3: geschätzter Erhaltungsbedarf für ${calculation.age} Jahre. Die Formel ist eine Orientierung, keine Diagnose. Individuelle Abweichungen von etwa 190 bis 340 kcal pro Tag sind möglich.` : "Für die Berechnung braucht MealZ Geburtsdatum, Geschlecht für die Formel, Größe und Gewicht. Ohne diese Angaben bleibt dein manueller Wert aktiv."}</p>
  </SettingsSection>;
}
function TagEditor({ label, values, onChange, placeholder }: { label: string; values: string[]; onChange: (values: string[]) => void; placeholder: string }) { const [text, setText] = useState(""); const add = () => { if (text.trim() && !values.includes(text.trim())) onChange([...values, text.trim()]); setText(""); }; return <div className="tag-editor"><span>{label}</span><div>{values.map((value) => <span key={value}>{value}<button aria-label={`${value} entfernen`} onClick={() => onChange(values.filter((item) => item !== value))}>×</button></span>)}<input aria-label={`${label} ergänzen`} value={text} onChange={(event) => setText(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); add(); } }} onBlur={add} placeholder={placeholder} /></div></div>; }
