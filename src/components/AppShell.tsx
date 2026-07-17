import type { PropsWithChildren } from "react";
import {
  BrainCircuit,
  CalendarDays,
  ChefHat,
  CircleUserRound,
  CookingPot,
  MessageCircleMore,
  Settings2,
  ShoppingBasket,
  Sparkles,
} from "lucide-react";
import type { ViewId } from "../types";
import { useAppStore } from "../store";

const navigation: { id: ViewId; label: string; icon: typeof CalendarDays; shortcut?: string }[] = [
  { id: "today", label: "Heute", icon: CookingPot, shortcut: "⌘1" },
  { id: "week", label: "Wochenplan", icon: CalendarDays, shortcut: "⌘2" },
  { id: "recipes", label: "Rezepte", icon: ChefHat, shortcut: "⌘3" },
  { id: "shopping", label: "Einkauf", icon: ShoppingBasket, shortcut: "⌘4" },
  { id: "agent", label: "Agent", icon: MessageCircleMore, shortcut: "⌘5" },
  { id: "memory", label: "Memory", icon: BrainCircuit },
];

export function AppShell({ children }: PropsWithChildren) {
  const view = useAppStore((state) => state.view);
  const setView = useAppStore((state) => state.setView);
  const profile = useAppStore((state) => state.profile);
  const agentStatus = useAppStore((state) => state.agentStatus);

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="titlebar" data-tauri-drag-region><span className="traffic-light-space" /></div>
        <button className="brand" onClick={() => setView("today")} aria-label="MealZ – zur Startseite">
          <span className="brand__mark"><ChefHat size={21} strokeWidth={2.2} /></span>
          <span><strong>MealZ</strong><small>Persönliche Küche</small></span>
        </button>
        <nav className="sidebar__nav" aria-label="Hauptnavigation">
          {navigation.map(({ id, label, icon: Icon, shortcut }) => (
            <button key={id} className={view === id ? "is-active" : ""} aria-current={view === id ? "page" : undefined} onClick={() => setView(id)}>
              <Icon size={18} /><span>{id === "agent" ? profile.agentName || label : label}</span>{id === "agent" && agentStatus !== "idle" && <i className="status-pulse" />}{shortcut && <kbd>{shortcut}</kbd>}
            </button>
          ))}
        </nav>
        <div className="sidebar__agent">
          <button onClick={() => setView("agent")}>
            <span className="agent-avatar"><Sparkles size={17} /></span>
            <span><strong>{profile.agentName || "Mila"}</strong><small>{agentStatus === "idle" ? "Bereit für dich" : "Denkt gerade …"}</small></span>
            <span className={`presence ${agentStatus !== "idle" ? "presence--busy" : ""}`} />
          </button>
        </div>
        <button className={`sidebar__profile ${view === "settings" ? "is-active" : ""}`} onClick={() => setView("settings")}>
          <CircleUserRound size={19} />
          <span><strong>{profile.name || "Profil"}</strong><small>Einstellungen</small></span>
          <Settings2 size={16} />
        </button>
      </aside>
      <main className="workspace">
        <div className="workspace__drag" data-tauri-drag-region />
        {children}
      </main>
    </div>
  );
}
