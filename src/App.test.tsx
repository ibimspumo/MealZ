// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { api, resetDemo } from "./bridge";
import { nextMondayRange } from "./components/DateRangePicker";
import { useAppStore } from "./store";

beforeEach(() => {
  resetDemo(true);
  useAppStore.setState({ view: "today", loading: true, agentStatus: "idle", onboardingComplete: true, onboardingSessionDismissed: false, toasts: [] });
  vi.spyOn(window, "confirm").mockReturnValue(true);
  Element.prototype.scrollIntoView = vi.fn();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("MealZ app flows", () => {
  it("loads the demo bootstrap and navigates through the primary surfaces", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Wochenplan/ }));
    expect(screen.getByRole("heading", { name: "Wochenplan" })).toBeInTheDocument();
    expect(screen.getAllByRole("listitem")).toHaveLength(7);

    await user.click(screen.getByRole("button", { name: "Agenda" }));
    expect(screen.getByRole("button", { name: "Agenda" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("region", { name: "Agenda des ausgewählten Zeitraums" })).toBeInTheDocument();
    const agendaCard = document.querySelector<HTMLElement>(".agenda-day .plan-card");
    expect(agendaCard?.querySelector(".plan-card__image + .plan-card__content")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Kalender" }));
    expect(screen.getByRole("list", { name: "Kalenderplan" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /^Rezepte/ }));
    expect(screen.getByRole("heading", { name: "Rezeptkatalog" })).toBeInTheDocument();
    const search = screen.getByRole("textbox", { name: "Rezepte suchen" });
    await user.type(search, "Lasagne");
    expect(screen.getByRole("button", { name: "Timos Lasagne öffnen" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Miso-Lachs mit Sesamreis öffnen/ })).not.toBeInTheDocument();
  });

  it("adds and checks a manual shopping item", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    await user.click(screen.getByRole("button", { name: /^Einkauf/ }));

    const itemName = screen.getByRole("textbox", { name: "Artikelname" });
    await user.type(itemName, "Mineralwasser");
    await user.click(screen.getByRole("button", { name: "Hinzufügen" }));
    expect(await screen.findByText("Mineralwasser")).toBeInTheDocument();

    const row = screen.getByText("Mineralwasser").closest(".shopping-row");
    expect(row).not.toBeNull();
    const checkbox = within(row as HTMLElement).getByRole("checkbox");
    await user.click(checkbox);
    await waitFor(() => expect(checkbox).toBeChecked());
  });

  it("loads the shopping list for the displayed next-week range and resets cancelled range picks", async () => {
    const user = userEvent.setup();
    const listSpy = vi.spyOn(api, "getShoppingList");
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    await user.click(screen.getByRole("button", { name: /^Einkauf/ }));
    const expected = nextMondayRange();
    await waitFor(() => expect(listSpy).toHaveBeenCalledWith(expected.start, expected.end));

    const trigger = screen.getByRole("button", { name: /Einkaufszeitraum/ });
    await user.click(trigger);
    const firstChoice = document.querySelector<HTMLButtonElement>(".date-range-picker__calendar button:not(.is-selected)");
    expect(firstChoice).not.toBeNull();
    await user.click(firstChoice!);
    expect(screen.getByText("Jetzt das Enddatum auswählen.")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "Datumsbereich auswählen" })).not.toBeInTheDocument();

    await user.click(trigger);
    const freshChoice = document.querySelector<HTMLButtonElement>(".date-range-picker__calendar button:not(.is-selected)");
    await user.click(freshChoice!);
    expect(screen.getByText("Jetzt das Enddatum auswählen.")).toBeInTheDocument();
  });

  it("opens a recipe, captures a rating, and persists feedback in the catalog state", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    await user.click(screen.getByRole("button", { name: /^Rezepte/ }));
    await user.click(screen.getByRole("button", { name: "Timos Lasagne öffnen" }));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Bewerten" }));
    await user.click(screen.getByRole("button", { name: "4 Sterne" }));
    await user.type(screen.getByPlaceholderText(/Was soll genau so bleiben/), "Etwas weniger Käse beim nächsten Mal.");
    await user.click(screen.getByRole("button", { name: "Feedback speichern" }));
    await waitFor(() => expect(useAppStore.getState().recipes.find((recipe) => recipe.id === "r-lasagna")?.ratingComment).toContain("weniger Käse"));
  });

  it("keeps the recipe dialog open and reports a failed deletion", async () => {
    const user = userEvent.setup();
    vi.spyOn(api, "deleteRecipe").mockRejectedValueOnce(new Error("Lokale Datenbank nicht erreichbar"));
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    await user.click(screen.getByRole("button", { name: /^Rezepte/ }));
    await user.click(screen.getByRole("button", { name: "Timos Lasagne öffnen" }));
    await user.click(screen.getByRole("button", { name: "Löschen" }));
    expect(await screen.findByText("Rezept konnte nicht gelöscht werden")).toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "Timos Lasagne" })).toBeInTheDocument();
  });

  it("reports a failed new-chat request without clearing the visible conversation", async () => {
    const user = userEvent.setup();
    vi.spyOn(api, "agentNewThread").mockRejectedValueOnce(new Error("Codex App Server nicht erreichbar"));
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    const nav = screen.getByRole("navigation", { name: "Hauptnavigation" });
    await user.click(within(nav).getByText("Mila").closest("button") as HTMLButtonElement);
    expect(screen.getByText(/Guten Morgen, Timo/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Neues Gespräch" }));
    expect(await screen.findByText("Neues Gespräch konnte nicht gestartet werden")).toBeInTheDocument();
    expect(screen.getByText(/Guten Morgen, Timo/)).toBeInTheDocument();
  });

  it("keeps the agent surface fixed while only the message list scrolls", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    const nav = screen.getByRole("navigation", { name: "Hauptnavigation" });
    await user.click(within(nav).getByText("Mila").closest("button") as HTMLButtonElement);
    expect(document.querySelector(".workspace")).toHaveClass("workspace--fixed");
    expect(document.querySelector(".page--agent .messages")).toBeInTheDocument();
  });

  it("reloads the current calendar range every time the calendar is opened", async () => {
    const user = userEvent.setup();
    const rangeSpy = vi.spyOn(api, "getWeekPlan");
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    await user.click(screen.getByRole("button", { name: /Wochenplan/ }));
    await waitFor(() => expect(rangeSpy).toHaveBeenCalledWith(useAppStore.getState().weekStart, useAppStore.getState().weekEnd));
    const firstCount = rangeSpy.mock.calls.length;
    await user.click(screen.getByRole("button", { name: /^Rezepte/ }));
    await user.click(screen.getByRole("button", { name: /Wochenplan/ }));
    await waitFor(() => expect(rangeSpy.mock.calls.length).toBeGreaterThan(firstCount));
  });

  it("opens full recipe details from calendar and agenda cards without leaving the plan", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    await user.click(screen.getByRole("button", { name: /Wochenplan/ }));
    const calendarCard = (await screen.findAllByRole("button", { name: /öffnen$/ }))[0];
    await user.click(calendarCard);
    expect(screen.getByText("Rezeptdetails aus deinem Kalender")).toBeInTheDocument();
    expect(screen.getByText("Zutaten")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Wochenplan" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Fertig" }));
    await user.click(screen.getByRole("button", { name: "Agenda" }));
    const agendaCard = screen.getAllByRole("button", { name: /öffnen$/ })[0];
    expect(within(agendaCard).getByText(/Orzo, Spinat und Feta in einer würzigen Tomatensauce/)).toBeInTheDocument();
    expect(within(agendaCard).getByText("628")).toBeInTheDocument();
    expect(within(agendaCard).getByText("25 g")).toBeInTheDocument();
    await user.click(agendaCard);
    expect(screen.getByText("Rezeptdetails aus deinem Kalender")).toBeInTheDocument();
  });

  it("does not open calendar details when removing a planned recipe", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    await user.click(screen.getByRole("button", { name: /Wochenplan/ }));
    const removeButton = (await screen.findAllByRole("button", { name: /entfernen$/ }))[0];
    await user.click(removeButton);
    expect(screen.queryByText("Rezeptdetails aus deinem Kalender")).not.toBeInTheDocument();
  });

  it("sends a message to the browser demo agent and renders structured tool activity", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    const mainNavigation = screen.getByRole("navigation", { name: "Hauptnavigation" });
    await user.click(within(mainNavigation).getByText("Mila").closest("button") as HTMLButtonElement);
    const composer = screen.getByRole("textbox", { name: "Nachricht an Mila" });
    await user.type(composer, "Plane bitte meine kommende Woche");
    await user.click(screen.getByRole("button", { name: "Nachricht senden" }));
    expect(screen.getByText("Kontext wird geprüft")).toBeInTheDocument();
    expect(await screen.findByText("Vorlieben gelesen", {}, { timeout: 2500 })).toBeInTheDocument();
    expect(screen.getByText("Wochenplan geprüft")).toBeInTheDocument();
  });

  it("keeps a completed tool timeline and saved recipe card after navigating away from the agent", async () => {
    const user = userEvent.setup();
    let emit: ((event: Parameters<Parameters<typeof api.onAgentEvent>[0]>[0]) => void) | undefined;
    vi.spyOn(api, "onAgentEvent").mockImplementation(async (handler) => { emit = handler; return () => undefined; });
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    const nav = screen.getByRole("navigation", { name: "Hauptnavigation" });
    await user.click(within(nav).getByText("Mila").closest("button") as HTMLButtonElement);
    act(() => emit?.({ type: "tool_started", activity: { id: "save-airfryer", name: "recipes_save", label: "Airfryer-Hähnchen wird gespeichert", status: "running" } }));
    act(() => emit?.({ type: "message_delta", messageId: "stream-airfryer", delta: "Ich speichere das Rezept." }));
    await user.click(within(nav).getByText("Wochenplan").closest("button") as HTMLButtonElement);
    act(() => emit?.({ type: "tool_completed", activity: { id: "save-airfryer", name: "recipes_save", label: "Airfryer-Hähnchen gespeichert", status: "success", recipeId: "r-lasagna", recipeTitle: "Timos Lasagne" } }));
    act(() => emit?.({ type: "message_completed", message: { id: "stream-airfryer", role: "assistant", content: "Fertig.", createdAt: new Date().toISOString() } }));
    await user.click(within(nav).getByText("Mila").closest("button") as HTMLButtonElement);
    expect(screen.getByText("Airfryer-Hähnchen gespeichert")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Rezept öffnen" })).toBeInTheDocument();
  });

  it("shows a clear fallback when update checks run in the browser demo", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    await user.click(screen.getByRole("button", { name: /Timo Einstellungen/ }));
    await user.click(screen.getByRole("button", { name: /Updates & Releases/ }));
    expect(screen.getByRole("heading", { name: "MealZ Updates" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Jetzt nach Updates suchen" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("nur in der installierten Desktop-App verfügbar");
  });
});

describe("MealZ onboarding", () => {
  it("opens automatically on the first start", async () => {
    resetDemo(false);
    useAppStore.setState({ loading: true, onboardingComplete: false, onboardingSessionDismissed: false });
    render(<App />);
    expect(await screen.findByRole("dialog", { name: "Willkommen bei MealZ." })).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Onboarding-Fortschritt" })).toHaveAttribute("aria-valuenow", "1");
    expect(screen.getByRole("textbox", { name: "Dein Name" })).toHaveValue("Timo");
  });

  it("completes all steps directly and persists the onboarding flag", async () => {
    const user = userEvent.setup();
    resetDemo(false);
    useAppStore.setState({ loading: true, onboardingComplete: false, onboardingSessionDismissed: false });
    render(<App />);
    await screen.findByRole("dialog", { name: "Willkommen bei MealZ." });
    for (let index = 0; index < 6; index += 1) {
      await user.click(screen.getByRole("button", { name: "Weiter" }));
    }
    expect(screen.getByRole("heading", { name: "Das ist dein persönlicher Ausgangspunkt." })).toBeInTheDocument();
    await user.type(screen.getByPlaceholderText(/Zum Beispiel besondere Routinen/), "Sonntags gern etwas Besonderes.");
    await user.click(screen.getByRole("button", { name: "MealZ einrichten" }));
    await waitFor(() => expect(useAppStore.getState().onboardingComplete).toBe(true));
    expect(screen.queryByRole("dialog", { name: /MealZ/ })).not.toBeInTheDocument();
  });

  it("persists the drafted profile before continuing onboarding with Mila", async () => {
    const user = userEvent.setup();
    resetDemo(false);
    useAppStore.setState({ loading: true, onboardingComplete: false, onboardingSessionDismissed: false });
    render(<App />);
    await screen.findByRole("dialog", { name: "Willkommen bei MealZ." });
    const name = screen.getByRole("textbox", { name: "Dein Name" });
    await user.clear(name);
    await user.type(name, "Timo Chat");
    for (let index = 0; index < 6; index += 1) await user.click(screen.getByRole("button", { name: "Weiter" }));
    await user.click(screen.getByRole("button", { name: "Mit Mila im Chat weiter" }));
    await waitFor(() => expect(useAppStore.getState().profile.name).toBe("Timo Chat"));
    expect(useAppStore.getState().view).toBe("agent");
    expect(useAppStore.getState().onboardingSessionDismissed).toBe(true);
    expect(useAppStore.getState().onboardingComplete).toBe(false);
  });

  it("restarts onboarding from agent settings", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    await user.click(screen.getByRole("button", { name: /Timo Einstellungen/ }));
    await user.click(screen.getByRole("button", { name: /Agent & Autonomie/ }));
    await user.click(screen.getByRole("button", { name: "Onboarding erneut starten" }));
    expect(await screen.findByRole("dialog", { name: "Willkommen bei MealZ." })).toBeInTheDocument();
    expect(useAppStore.getState().onboardingComplete).toBe(false);
  });

  it("loads, edits and saves both agent markdown files", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: /Guten (Morgen|Tag|Abend), Timo/ });
    await user.click(screen.getByRole("button", { name: /Timo Einstellungen/ }));
    await user.click(screen.getByRole("button", { name: /Agent & Autonomie/ }));
    const persona = await screen.findByRole("textbox", { name: "PERSONA.md" });
    const memory = screen.getByRole("textbox", { name: "MEMORY.md" });
    expect((persona as HTMLTextAreaElement).value).toContain("U+2014");
    await user.clear(persona);
    await user.type(persona, "# PERSONA.md\n\nKlar und freundlich. Niemals U+2014.");
    await user.clear(memory);
    await user.type(memory, "# MEMORY.md\n\nTimo liebt Sonntagsgerichte.");
    expect(screen.getByText("Ungespeicherte Änderungen")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Agent-Dateien speichern" }));
    await waitFor(() => expect(screen.getByText("Gespeichert")).toBeInTheDocument());
    await expect(api.getAgentFiles()).resolves.toEqual({ persona: "# PERSONA.md\n\nKlar und freundlich. Niemals U+2014.", memory: "# MEMORY.md\n\nTimo liebt Sonntagsgerichte." });
  });
});
