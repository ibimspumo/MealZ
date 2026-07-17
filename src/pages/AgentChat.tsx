import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { BrainCircuit, CheckCircle2, CircleAlert, CircleStop, Globe2, LoaderCircle, MessageSquarePlus, Send, Sparkles } from "lucide-react";
import { Button, ExternalLink, IconButton, NutritionStrip, PageHeader, SafeRecipeImage } from "../components/Common";
import { useAppStore } from "../store";

const suggestions = [
  "Plane meine kommende Woche mit sieben abwechslungsreichen Hauptgerichten.",
  "Was passt heute zu meinen offenen Nährwertzielen?",
  "Finde ein neues, schnelles Airfryer-Rezept mit Hähnchen.",
  "Welche gespeicherten Favoriten hatte ich länger nicht?",
];

export function AgentChat() {
  const messages = useAppStore((state) => state.messages);
  const profile = useAppStore((state) => state.profile);
  const memories = useAppStore((state) => state.memories);
  const agentStatus = useAppStore((state) => state.agentStatus);
  const agentDraft = useAppStore((state) => state.agentDraft);
  const setAgentDraft = useAppStore((state) => state.setAgentDraft);
  const sendMessage = useAppStore((state) => state.sendMessage);
  const newThread = useAppStore((state) => state.newThread);
  const stopAgent = useAppStore((state) => state.stopAgent);
  const setView = useAppStore((state) => state.setView);
  const recipes = useAppStore((state) => state.recipes);
  const setSelectedRecipeId = useAppStore((state) => state.setSelectedRecipeId);
  const agentCapabilities = useAppStore((state) => state.agentCapabilities);
  const toast = useAppStore((state) => state.toast);
  const [text, setText] = useState("");
  const [startingNewThread, setStartingNewThread] = useState(false);
  const messagesRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const container = messagesRef.current;
    if (!container) return;
    if (typeof container.scrollTo === "function") container.scrollTo({ top: container.scrollHeight, behavior: "smooth" });
    else container.scrollTop = container.scrollHeight;
  }, [messages, agentStatus]);
  useEffect(() => { if (agentDraft) { setText(agentDraft); setAgentDraft(""); } }, [agentDraft, setAgentDraft]);
  const submit = async () => {
    const content = text.trim();
    if (!content || agentStatus !== "idle") return;
    setText("");
    try { await sendMessage(content); } catch { setText(content); }
  };
  const startNewThread = async () => {
    if (messages.length && !window.confirm("Neues Gespräch starten? Der bisherige Verlauf bleibt lokal gespeichert.")) return;
    setStartingNewThread(true);
    try { await newThread(); }
    catch (error) { toast("error", "Neues Gespräch konnte nicht gestartet werden", String(error)); }
    finally { setStartingNewThread(false); }
  };

  return (
    <div className="page page--agent">
      <PageHeader title={profile.agentName || "Mila"} description="Dein persönlicher Meal-Planning-Agent · Codex App Server" actions={<Button icon={<MessageSquarePlus size={16} />} disabled={startingNewThread} onClick={startNewThread}>{startingNewThread ? "Starte Gespräch …" : "Neues Gespräch"}</Button>} />
      <div className="chat-layout">
        <section className="chat-panel">
          <header className="chat-panel__status"><span className={`agent-avatar agent-avatar--large ${agentStatus !== "idle" ? "is-busy" : ""}`}><Sparkles size={20} /></span><div><strong>{profile.agentName || "Mila"}</strong><small>{agentStatus === "idle" ? "Bereit · kennt deinen Plan und deine Vorlieben" : agentStatus === "thinking" ? "Denkt nach und prüft deinen Kontext …" : "Antwortet gerade …"}</small></div><span className="codex-badge">Codex App Server</span></header>
          <div className="messages" ref={messagesRef} aria-live="polite">
            {!messages.length && <div className="chat-welcome"><span><Sparkles size={25} /></span><h2>Was möchtest du essen?</h2><p>Frag nach einer ganzen Woche, einem einzelnen Rezept oder ändere gemeinsam mit mir deinen bestehenden Plan.</p><div>{suggestions.map((suggestion) => <button key={suggestion} onClick={() => setText(suggestion)}>{suggestion}</button>)}</div></div>}
            {messages.map((message) => (
              <article className={`message message--${message.role}`} key={message.id}>
                {message.role === "assistant" && <span className="message__avatar"><Sparkles size={15} /></span>}
                <div className="message__content">
                  {message.role === "assistant" ? <ReactMarkdown remarkPlugins={[remarkGfm]} components={{ a: ({ href, children }) => href ? <ExternalLink href={href}>{children}</ExternalLink> : <>{children}</> }}>{message.content}</ReactMarkdown> : <p>{message.content}</p>}
                  {message.tools?.length ? <ToolTimeline tools={message.tools} /> : null}
                  <RecipeResultCard recipe={message.tools?.some((tool) => tool.name === "recipes_save") ? recipes.find((recipe) => recipe.id === message.recipeId || recipe.title === message.recipeTitle || message.tools?.some((tool) => tool.name === "recipes_save" && (tool.recipeId === recipe.id || tool.recipeTitle === recipe.title))) : undefined} onOpen={(id) => { setSelectedRecipeId(id); setView("recipes"); }} />
                </div>
              </article>
            ))}
            {agentStatus === "thinking" && <article className="message message--assistant"><span className="message__avatar"><Sparkles size={15} /></span><div className="thinking-indicator"><i /><i /><i /><span>Kontext wird geprüft</span></div></article>}
          </div>
          <footer className="composer-wrap">
            <div className="composer">
              <textarea id="agent-composer" value={text} onChange={(event) => setText(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); submit(); } }} rows={1} placeholder={`Schreib ${profile.agentName || "deinem Agenten"}, was du planst, ändern oder kochen möchtest …`} aria-label={`Nachricht an ${profile.agentName || "deinen Agenten"}`} />
              {agentStatus !== "idle" ? <IconButton className="composer__send composer__send--stop" label="Agent stoppen" onClick={stopAgent}><CircleStop size={18} /></IconButton> : <IconButton className="composer__send" label="Nachricht senden" disabled={!text.trim()} onClick={submit}><Send size={17} /></IconButton>}
            </div>
            <p>Enter zum Senden · Shift + Enter für eine neue Zeile · Änderungen werden strukturiert gespeichert</p>
          </footer>
        </section>
        <aside className="context-panel">
          <section><header><BrainCircuit size={17} /><h3>Aktiver Kontext</h3></header><p>{memories.filter((memory) => memory.active).length} Erinnerungen fließen in Vorschläge ein.</p><div className="context-chips">{memories.filter((memory) => memory.active).slice(0, 4).map((memory) => <span key={memory.id}>{memory.title}</span>)}</div><button className="text-link" onClick={() => setView("memory")}>Memory ansehen</button></section>
          <section><header><Globe2 size={17} /><h3>Web-Recherche</h3></header><p>Bei neuen Rezepten kann {profile.agentName || "dein Agent"} Quellen recherchieren, vergleichen und direkt am Rezept speichern.</p><span className="context-state">{agentCapabilities.webSearch === true ? <CheckCircle2 size={14} /> : agentCapabilities.webSearch === false ? <CircleAlert size={14} /> : <LoaderCircle size={14} />}{agentCapabilities.webSearch === true ? "Verfügbar" : agentCapabilities.webSearch === false ? "Nicht verfügbar" : "Wird beim ersten Auftrag geprüft"}</span></section>
          <section className="context-panel__privacy"><strong>Lokal & transparent</strong><p>Pläne, Bewertungen und Erinnerungen liegen in deiner lokalen MealZ-Datenbank.</p></section>
        </aside>
      </div>
    </div>
  );
}

function ToolTimeline({ tools }: { tools: NonNullable<import("../types").AgentMessage["tools"]> }) {
  const [expanded, setExpanded] = useState(true);
  const running = tools.some((tool) => tool.status === "running");
  return <section className={`tool-timeline ${running ? "is-running" : ""}`} aria-label="Arbeitsprotokoll">
    <button type="button" className="tool-timeline__toggle" aria-expanded={expanded} onClick={() => setExpanded((value) => !value)}><span>{running ? <LoaderCircle size={14} /> : <CheckCircle2 size={14} />}</span><strong>{running ? "Arbeite gerade an deinem Auftrag" : "Arbeitsschritte"}</strong><small>{tools.length}</small></button>
    {expanded && <ol>{tools.map((tool) => <li key={tool.id} className={`tool-activity tool-activity--${tool.status}`}>{tool.status === "running" ? <LoaderCircle size={14} /> : tool.status === "success" ? <CheckCircle2 size={14} /> : <CircleAlert size={14} />}<span><strong>{tool.label || humanizeTool(tool.name)}</strong>{tool.detail && <small>{tool.detail}</small>}</span><time>{tool.status === "running" ? "läuft" : tool.status === "success" ? "erledigt" : "nicht abgeschlossen"}</time></li>)}</ol>}
  </section>;
}

function humanizeTool(name: string) {
  const names: Record<string, string> = { context_read: "Kontext gelesen", memory_read: "Memory geprüft", memory_save: "Memory gespeichert", recipes_search: "Rezepte durchsucht", recipes_save: "Rezept gespeichert", web_search: "Web-Recherche", webSearch: "Web-Recherche", plan_get_week: "Wochenplan geprüft" };
  return names[name] ?? name.replace(/_/g, " ");
}

function RecipeResultCard({ recipe, onOpen }: { recipe?: import("../types").Recipe; onOpen: (id: string) => void }) {
  if (!recipe) return null;
  return <section className="chat-recipe-card" aria-label={`Gespeichertes Rezept ${recipe.title}`}><div className="chat-recipe-card__image"><SafeRecipeImage src={recipe.imageUrl} alt={`Serviervorschlag für ${recipe.title}`} fallback={<Sparkles size={21} />} /></div><div><strong>{recipe.title}</strong><p>{recipe.description}</p><NutritionStrip nutrition={recipe.nutrition} compact /></div><Button tone="secondary" onClick={() => onOpen(recipe.id)}>Rezept öffnen</Button></section>;
}
