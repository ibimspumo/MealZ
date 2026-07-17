import { useEffect, useMemo, useState } from "react";
import { BrainCircuit, CircleAlert, MessageSquareText, Pencil, Plus, SlidersHorizontal, Sparkles, Trash2 } from "lucide-react";
import { Button, EmptyState, IconButton, Modal, PageHeader } from "../components/Common";
import { useAppStore } from "../store";
import type { Memory } from "../types";

const kindLabel: Record<string, string> = { preference: "Vorliebe", routine: "Alltag", feedback: "Feedback", constraint: "Feste Regel" };
const sourceLabel: Record<string, string> = { explicit: "Von dir angegeben", rating: "Aus Bewertung", inferred: "Vom Agenten abgeleitet", agent: "Vom Agenten", import: "Importiert" };
const labelForKind = (value: string) => kindLabel[value] ?? value.replace(/_/g, " ");
const labelForSource = (value: string) => sourceLabel[value] ?? value.replace(/_/g, " ");

export function MemoryCenter() {
  const memories = useAppStore((state) => state.memories);
  const saveMemory = useAppStore((state) => state.saveMemory);
  const deleteMemory = useAppStore((state) => state.deleteMemory);
  const [filter, setFilter] = useState<string | "all">("all");
  const [editing, setEditing] = useState<Memory | "new" | null>(null);
  const [deleteCandidate, setDeleteCandidate] = useState<Memory | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState("");
  const visible = useMemo(() => filter === "all" ? memories : memories.filter((memory) => memory.kind === filter), [filter, memories]);
  return (
    <div className="page page--memory">
      <PageHeader title="Memory" description="Was Mila über dich weiß – sichtbar, nachvollziehbar und vollständig in deiner Hand." actions={<Button tone="primary" icon={<Plus size={16} />} onClick={() => setEditing("new")}>Erinnerung hinzufügen</Button>} />
      <section className="memory-intro"><span><BrainCircuit size={23} /></span><div><h2>Persönlich, nicht mysteriös</h2><p>Erinnerungen helfen Mila, deine Wochen wirklich passend zu planen. Abgeleitete Vorlieben erkennst du an ihrer Herkunft und Confidence. Du kannst alles bearbeiten, pausieren oder löschen.</p></div><strong>{memories.filter((memory) => memory.active).length}<small>aktiv</small></strong></section>
      <div className="memory-toolbar"><SlidersHorizontal size={16} /><span>Ansicht</span><div className="segmented">{(["all", ...Array.from(new Set(memories.map((memory) => memory.kind)))]).map((value) => <button aria-pressed={filter === value} className={filter === value ? "is-active" : ""} key={value} onClick={() => setFilter(value)}>{value === "all" ? "Alle" : labelForKind(value)}</button>)}</div></div>
      {visible.length ? <div className="memory-list">{visible.map((memory) => <article className={`memory-row ${!memory.active ? "is-paused" : ""}`} key={memory.id}><span className="memory-row__icon">{memory.kind === "feedback" ? <MessageSquareText size={18} /> : memory.source === "inferred" ? <Sparkles size={18} /> : <BrainCircuit size={18} />}</span><div><div className="memory-row__meta"><span>{labelForKind(memory.kind)}</span><span>{labelForSource(memory.source)}</span><span>{Math.round(memory.confidence * 100)} % sicher</span>{memory.preferenceScore !== undefined && <span>Vorliebe {memory.preferenceScore}/10</span>}{!memory.active && <span className="paused-badge">Pausiert</span>}</div><h3>{memory.title}</h3><p>{memory.content}</p>{memory.evidence?.length ? <p className="memory-evidence">Belege: {memory.evidence.join(" · ")}</p> : null}</div><div className="memory-row__actions"><label className="switch switch--compact"><input type="checkbox" aria-label={`${memory.title} ${memory.active ? "pausieren" : "aktivieren"}`} checked={memory.active} onChange={(event) => saveMemory({ ...memory, active: event.target.checked })} /><span /></label><IconButton label="Erinnerung bearbeiten" onClick={() => setEditing(memory)}><Pencil size={15} /></IconButton><IconButton label="Erinnerung löschen" onClick={() => { setDeleteError(""); setDeleteCandidate(memory); }}><Trash2 size={15} /></IconButton></div></article>)}</div> : <EmptyState icon={<BrainCircuit size={25} />} title="Noch keine Erinnerungen"><p>Füge eine feste Regel hinzu oder erzähle deinem Agenten von deinen Vorlieben.</p></EmptyState>}
      <MemoryEditor memory={editing} onClose={() => setEditing(null)} onSave={async (memory) => { await saveMemory(memory); setEditing(null); }} />
      <Modal open={Boolean(deleteCandidate)} onClose={() => { if (!deleting) setDeleteCandidate(null); }} title="Erinnerung löschen?" description={deleteCandidate?.title} size="small" footer={<><Button disabled={deleting} onClick={() => setDeleteCandidate(null)}>Abbrechen</Button><Button tone="danger" icon={<Trash2 size={15} />} disabled={deleting} onClick={async () => { if (!deleteCandidate) return; setDeleting(true); setDeleteError(""); try { await deleteMemory(deleteCandidate.id); setDeleteCandidate(null); } catch (error) { setDeleteError(String(error)); } finally { setDeleting(false); } }}>{deleting ? "Wird gelöscht …" : "Erinnerung löschen"}</Button></>}>
        <div className="destructive-confirmation"><CircleAlert size={18} /><div><strong>Diese Information fließt danach nicht mehr in Vorschläge ein.</strong><p>Andere Erinnerungen, Rezepte und Pläne bleiben unverändert.</p>{deleteError && <p className="form-error" role="alert">{deleteError}</p>}</div></div>
      </Modal>
    </div>
  );
}

function blankMemory(): Memory { const now = new Date().toISOString(); return { id: "", kind: "preference", title: "", content: "", confidence: 1, source: "explicit", preferenceScore: 5, active: true, createdAt: now, updatedAt: now }; }

function MemoryEditor({ memory, onClose, onSave }: { memory: Memory | "new" | null; onClose: () => void; onSave: (memory: Memory) => Promise<void> }) {
  const [draft, setDraft] = useState<Memory>(blankMemory());
  const nextKey = memory === "new" ? "new" : memory?.id ?? null;
  useEffect(() => { if (nextKey) setDraft(memory && memory !== "new" ? structuredClone(memory) : blankMemory()); }, [memory, nextKey]);
  return <Modal open={memory !== null} onClose={onClose} title={memory === "new" ? "Neue Erinnerung" : "Erinnerung bearbeiten"} size="small" footer={<><Button tone="quiet" onClick={onClose}>Abbrechen</Button><Button tone="primary" disabled={!draft.title.trim() || !draft.content.trim()} onClick={() => onSave(draft)}>Speichern</Button></>}><div className="form-stack"><label className="field"><span>Art</span><select value={draft.kind} onChange={(event) => { const kind = event.target.value as Memory["kind"]; setDraft({ ...draft, kind, preferenceScore: kind === "preference" ? draft.preferenceScore ?? 5 : undefined }); }}>{Object.entries(kindLabel).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>{draft.kind === "preference" && <label className="preference-scale"><span><strong>Wie gern magst du das?</strong><small>1 bedeutet möglichst vermeiden, 10 bedeutet sehr gern und darf häufig wiederkommen.</small></span><div><input type="range" min="1" max="10" step="1" value={draft.preferenceScore ?? 5} onChange={(event) => setDraft({ ...draft, preferenceScore: Number(event.target.value) })} aria-label="Vorliebenstärke von 1 bis 10" /><output>{draft.preferenceScore ?? 5}<small>/10</small></output></div></label>}<label className="field"><span>Kurzbezeichnung</span><input value={draft.title} onChange={(event) => setDraft({ ...draft, title: event.target.value })} placeholder="z. B. Karotten eher vermeiden" /></label><label className="field"><span>Kontext für Mila</span><textarea rows={5} value={draft.content} onChange={(event) => setDraft({ ...draft, content: event.target.value })} placeholder="Beschreibe möglichst konkret, wann und wie das berücksichtigt werden soll." /></label><label className="switch-row"><span><strong>Aktiv verwenden</strong><small>Fließt in neue Vorschläge ein</small></span><span className="switch"><input type="checkbox" aria-label="Erinnerung aktiv verwenden" checked={draft.active} onChange={(event) => setDraft({ ...draft, active: event.target.checked })} /><span /></span></label></div></Modal>;
}
