import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { BrainCircuit, CheckCircle2, CircleAlert, CircleStop, Clock3, Gauge, Globe2, History, LoaderCircle, MessageSquarePlus, Minimize2, Send, Sparkles } from "lucide-react";
import { Button, ExternalLink, IconButton, Modal, NutritionStrip, PageHeader, SafeRecipeImage } from "../components/Common";
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
  const agentConversations = useAppStore((state) => state.agentConversations);
  const loadAgentConversations = useAppStore((state) => state.loadAgentConversations);
  const activateConversation = useAppStore((state) => state.activateConversation);
  const stopAgent = useAppStore((state) => state.stopAgent);
  const setView = useAppStore((state) => state.setView);
  const recipes = useAppStore((state) => state.recipes);
  const setSelectedRecipeId = useAppStore((state) => state.setSelectedRecipeId);
  const agentCapabilities = useAppStore((state) => state.agentCapabilities);
  const agentContext = useAppStore((state) => state.agentContext);
  const compactAgentContext = useAppStore((state) => state.compactAgentContext);
  const toast = useAppStore((state) => state.toast);
  const [text, setText] = useState("");
  const [startingNewThread, setStartingNewThread] = useState(false);
  const [newThreadError, setNewThreadError] = useState("");
  const [newThreadDialogOpen, setNewThreadDialogOpen] = useState(false);
  const [conversationMenuOpen, setConversationMenuOpen] = useState(false);
  const [loadingConversations, setLoadingConversations] = useState(false);
  const [conversationError, setConversationError] = useState("");
  const [activatingConversationId, setActivatingConversationId] = useState<string>();
  const messagesRef = useRef<HTMLDivElement>(null);
  const conversationMenuRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const container = messagesRef.current;
    if (!container) return;
    if (typeof container.scrollTo === "function") container.scrollTo({ top: container.scrollHeight, behavior: "smooth" });
    else container.scrollTop = container.scrollHeight;
  }, [messages, agentStatus]);
  useEffect(() => { if (agentDraft) { setText(agentDraft); setAgentDraft(""); } }, [agentDraft, setAgentDraft]);
  useEffect(() => {
    if (!conversationMenuOpen) return;
    const closeOnOutsideClick = (event: MouseEvent) => {
      if (!conversationMenuRef.current?.contains(event.target as Node)) setConversationMenuOpen(false);
    };
    document.addEventListener("mousedown", closeOnOutsideClick);
    return () => document.removeEventListener("mousedown", closeOnOutsideClick);
  }, [conversationMenuOpen]);
  const submit = async () => {
    const content = text.trim();
    if (!content || agentStatus !== "idle") return;
    setText("");
    try { await sendMessage(content); } catch { setText(content); }
  };
  const refreshConversations = async () => {
    setLoadingConversations(true);
    setConversationError("");
    try { await loadAgentConversations(); }
    catch (error) { setConversationError(String(error)); }
    finally { setLoadingConversations(false); }
  };
  const openConversationMenu = () => {
    const next = !conversationMenuOpen;
    setConversationMenuOpen(next);
    if (next) void refreshConversations();
  };
  const startNewThread = async () => {
    setStartingNewThread(true);
    setNewThreadError("");
    try {
      await newThread();
      setNewThreadDialogOpen(false);
      setConversationMenuOpen(false);
    }
    catch (error) {
      const detail = String(error);
      setNewThreadError(detail);
      toast("error", "Neues Gespräch konnte nicht gestartet werden", detail);
    }
    finally { setStartingNewThread(false); }
  };
  const openConversation = async (sessionId: string) => {
    setActivatingConversationId(sessionId);
    setConversationError("");
    try {
      await activateConversation(sessionId);
      setConversationMenuOpen(false);
    } catch (error) {
      setConversationError(String(error));
    } finally {
      setActivatingConversationId(undefined);
    }
  };

  return (
    <div className="page page--agent">
      <PageHeader title={profile.agentName || "Mila"} description="Dein persönlicher Meal-Planning-Agent · Codex App Server" actions={<>
        <div className="conversation-menu" ref={conversationMenuRef}>
          <Button icon={<History size={16} />} aria-haspopup="dialog" aria-expanded={conversationMenuOpen} onClick={openConversationMenu}>Gespräche</Button>
          {conversationMenuOpen && <section className="conversation-menu__popover" role="dialog" aria-label="Gesprächsverlauf">
            <header><div><strong>Deine Gespräche</strong><small>Verläufe bleiben lokal gespeichert.</small></div><span>{agentConversations.length}</span></header>
            <div className="conversation-menu__list">
              {loadingConversations && <div className="conversation-menu__state"><LoaderCircle size={16} />Gespräche werden geladen …</div>}
              {!loadingConversations && conversationError && <div className="conversation-menu__error"><CircleAlert size={16} /><span><strong>Nicht geladen</strong><small>{conversationError}</small></span><button onClick={refreshConversations}>Erneut versuchen</button></div>}
              {!loadingConversations && !conversationError && !agentConversations.length && <div className="conversation-menu__state">Noch keine gespeicherten Gespräche.</div>}
              {!loadingConversations && !conversationError && agentConversations.map((conversation) => <button
                type="button"
                className={conversation.status === "active" ? "is-active" : ""}
                aria-current={conversation.status === "active" ? "true" : undefined}
                disabled={conversation.status === "active" || Boolean(activatingConversationId)}
                key={conversation.id}
                onClick={() => openConversation(conversation.id)}
              >
                <span className="conversation-menu__icon">{activatingConversationId === conversation.id ? <LoaderCircle size={15} /> : <Clock3 size={15} />}</span>
                <span><strong>{conversation.title}</strong><small>{conversation.preview || (conversation.messageCount ? `${conversation.messageCount} Nachrichten` : "Noch keine Nachrichten")}</small><time>{formatConversationDate(conversation.updatedAt)}</time></span>
                {conversation.status === "active" && <em>Aktiv</em>}
              </button>)}
            </div>
          </section>}
        </div>
        <Button tone="primary" icon={<MessageSquarePlus size={16} />} onClick={() => { setNewThreadError(""); setNewThreadDialogOpen(true); }}>Neues Gespräch</Button>
      </>} />
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
          <section><header><BrainCircuit size={17} /><h3>Persönlicher Kontext</h3></header><p>{memories.filter((memory) => memory.active).length} Erinnerungen fließen in Vorschläge ein.</p><div className="context-chips">{memories.filter((memory) => memory.active).slice(0, 4).map((memory) => <span key={memory.id}>{memory.title}</span>)}</div><button className="text-link" onClick={() => setView("memory")}>Memory ansehen</button></section>
          <ConversationContext
            context={agentContext}
            busy={agentStatus !== "idle"}
            onCompact={() => void compactAgentContext()}
            onNewConversation={() => { setNewThreadError(""); setNewThreadDialogOpen(true); }}
          />
          <section><header><Globe2 size={17} /><h3>Web-Recherche</h3></header><p>Bei neuen Rezepten kann {profile.agentName || "dein Agent"} Quellen recherchieren, vergleichen und direkt am Rezept speichern.</p><span className="context-state">{agentCapabilities.webSearch === true ? <CheckCircle2 size={14} /> : agentCapabilities.webSearch === false ? <CircleAlert size={14} /> : <LoaderCircle size={14} />}{agentCapabilities.webSearch === true ? "Verfügbar" : agentCapabilities.webSearch === false ? "Nicht verfügbar" : "Wird beim ersten Auftrag geprüft"}</span></section>
          <section className="context-panel__privacy"><strong>Lokal & transparent</strong><p>Pläne, Bewertungen und Erinnerungen liegen in deiner lokalen MealZ-Datenbank.</p></section>
        </aside>
      </div>
      <Modal
        open={newThreadDialogOpen}
        onClose={() => { if (!startingNewThread) setNewThreadDialogOpen(false); }}
        title="Neues Gespräch starten?"
        description="Dein bisheriger Verlauf bleibt lokal gespeichert und ist über „Gespräche“ jederzeit wieder erreichbar."
        size="small"
        footer={<><Button disabled={startingNewThread} onClick={() => setNewThreadDialogOpen(false)}>Abbrechen</Button><Button tone="primary" disabled={startingNewThread} icon={startingNewThread ? <LoaderCircle size={16} /> : <MessageSquarePlus size={16} />} onClick={startNewThread}>{startingNewThread ? "Gespräch wird gestartet …" : "Gespräch starten"}</Button></>}
      >
        <div className="new-conversation-note"><Sparkles size={18} /><div><strong>{agentStatus === "idle" ? "Mila beginnt mit einem leeren Verlauf." : "Der laufende Auftrag wird beendet."}</strong><p>Rezepte, Pläne und Erinnerungen bleiben unverändert. Nur der Chat-Kontext beginnt neu.</p></div></div>
        {newThreadError && <div className="new-conversation-error" role="alert"><CircleAlert size={16} /><div><strong>Gespräch konnte nicht gestartet werden</strong><p>{newThreadError}</p></div></div>}
      </Modal>
    </div>
  );
}

function ConversationContext({ context, busy, onCompact, onNewConversation }: {
  context: import("../types").AgentContextState;
  busy: boolean;
  onCompact: () => void;
  onNewConversation: () => void;
}) {
  const content = context.stage === "compacting"
    ? { title: "Kontext wird verdichtet", detail: "Wichtige Absprachen bleiben erhalten. Danach kannst du im selben Gespräch weitermachen." }
    : context.stage === "recommend_new"
      ? { title: "Neues Gespräch empfohlen", detail: "Trotz automatischer Verdichtung ist nur noch wenig Platz. Dein bisheriger Verlauf bleibt gespeichert." }
      : context.stage === "warning"
        ? { title: "Kontext wird bald verdichtet", detail: "Du kannst weiter schreiben. Ältere Nachrichten werden automatisch zusammengefasst." }
        : context.stage === "error"
          ? { title: "Verdichtung nicht abgeschlossen", detail: context.detail || "Starte ein neues Gespräch, wenn der Kontext weiter knapp bleibt." }
          : context.stage === "healthy"
            ? { title: "Genug Platz im Gespräch", detail: "Ältere Inhalte werden bei Bedarf automatisch verdichtet." }
            : { title: "Noch keine Messung", detail: "Die Auslastung erscheint nach der ersten Antwort." };
  const percentage = context.utilizationPercent == null ? undefined : Math.max(0, Math.min(100, Math.round(context.utilizationPercent)));
  const className = `conversation-context conversation-context--${context.stage}`;
  return <section className={className}>
    <header><Gauge size={17} /><h3>Gesprächskontext</h3></header>
    <div className="conversation-context__summary" aria-live="polite">
      {context.stage === "compacting" ? <LoaderCircle size={16} /> : context.stage === "recommend_new" || context.stage === "error" ? <CircleAlert size={16} /> : <CheckCircle2 size={16} />}
      <strong>{content.title}</strong>
      {percentage != null && context.stage !== "compacting" && <span>{percentage} % belegt</span>}
    </div>
    {percentage != null && <div className="conversation-context__meter" role="progressbar" aria-label="Belegter Gesprächskontext" aria-valuemin={0} aria-valuemax={100} aria-valuenow={percentage}><span style={{ width: `${percentage}%` }} /></div>}
    <p>{content.detail}</p>
    {context.lastCompactedAt && context.stage !== "compacting" && <small className="conversation-context__last">Zuletzt automatisch verdichtet</small>}
    <div className="conversation-context__actions">
      {(context.stage === "warning" || context.stage === "error") && <button className="text-link" disabled={busy} onClick={onCompact}><Minimize2 size={14} />Jetzt verdichten</button>}
      {(context.stage === "recommend_new" || context.stage === "error") && <button className="text-link text-link--strong" onClick={onNewConversation}><MessageSquarePlus size={14} />Neues Gespräch</button>}
    </div>
  </section>;
}

function formatConversationDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Unbekannter Zeitpunkt";
  return new Intl.DateTimeFormat("de-DE", { dateStyle: "medium", timeStyle: "short" }).format(date);
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
