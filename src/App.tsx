import { useEffect } from "react";
import { AppShell } from "./components/AppShell";
import { Skeleton, ToastRegion } from "./components/Common";
import { Onboarding } from "./components/Onboarding";
import { AgentChat } from "./pages/AgentChat";
import { Dashboard } from "./pages/Dashboard";
import { MemoryCenter } from "./pages/MemoryCenter";
import { Recipes } from "./pages/Recipes";
import { Settings } from "./pages/Settings";
import { Shopping } from "./pages/Shopping";
import { WeekPlan } from "./pages/WeekPlan";
import { useAppStore } from "./store";
import "./App.css";

function App() {
  const initialize = useAppStore((state) => state.initialize);
  const view = useAppStore((state) => state.view);
  const loading = useAppStore((state) => state.loading);
  const onboardingComplete = useAppStore((state) => state.onboardingComplete);
  const onboardingSessionDismissed = useAppStore((state) => state.onboardingSessionDismissed);
  const setView = useAppStore((state) => state.setView);
  useEffect(() => { initialize(); }, [initialize]);
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (!onboardingComplete && !onboardingSessionDismissed) return;
      if (!event.metaKey || event.shiftKey || event.altKey) return;
      const shortcuts = { "1": "today", "2": "week", "3": "recipes", "4": "shopping", "5": "agent" } as const;
      const next = shortcuts[event.key as keyof typeof shortcuts];
      if (next) { event.preventDefault(); setView(next); }
      if (event.key.toLocaleLowerCase() === "k") { event.preventDefault(); setView("agent"); window.setTimeout(() => document.querySelector<HTMLTextAreaElement>("#agent-composer")?.focus(), 80); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onboardingComplete, onboardingSessionDismissed, setView]);
  const content = loading ? <div className="page page--loading"><Skeleton lines={7} /></div> : view === "today" ? <Dashboard /> : view === "week" ? <WeekPlan /> : view === "recipes" ? <Recipes /> : view === "shopping" ? <Shopping /> : view === "agent" ? <AgentChat /> : view === "memory" ? <MemoryCenter /> : <Settings />;
  return <><AppShell>{content}</AppShell>{!loading && !onboardingComplete && !onboardingSessionDismissed && <Onboarding />}<ToastRegion /></>;
}

export default App;
