<div align="center">
  <img src="assets/mealz-app-icon.png" alt="MealZ App Icon" width="128" height="128">
  <h1>MealZ</h1>
  <p><strong>Persönliche, lokale Essensplanung mit einem eigenen Codex-Agenten.</strong></p>
  <p><em>A personal, local-first meal planner with its own Codex agent.</em></p>
  <p>
    <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white">
    <img alt="React 19" src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=1b1f23">
    <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white">
    <img alt="License MIT" src="https://img.shields.io/badge/License-MIT-3d7651">
    <img alt="Status Alpha" src="https://img.shields.io/badge/Status-Alpha-eb795b">
  </p>
</div>

> [!IMPORTANT]
> MealZ verwendet ausschließlich `codex app-server` als Agentenlaufzeit. Es gibt weder eine Responses-API-Integration noch ein Agents SDK, einen alternativen Anbieter oder einen stillen Fallback.

**Sprachen / Languages:** [Deutsch](#deutsch) · [English](#english)

---

## Deutsch

MealZ ist eine persönliche Tauri-2-Desktop-App für macOS. Sie verbindet Wochenplanung, Rezeptkatalog, Einkaufsliste und ein transparentes Memory-System mit einem dauerhaft persönlichen Meal-Planning-Agenten. Das Projekt ist Open Source, wird aber bewusst für einen einzelnen lokalen Nutzer statt für einen Multi-Tenant-SaaS optimiert.

### Download auf macOS öffnen

Der aktuelle Apple-Silicon-Build ist updater-signiert, aber noch nicht mit einer Apple Developer ID notarisiert. Verschiebe `MealZ.app` zuerst in den Programme-Ordner und führe anschließend diesen Befehl im Terminal aus:

```bash
xattr -dr com.apple.quarantine "/Applications/MealZ.app"
```

Der Befehl entfernt ausschließlich das macOS-Quarantäne-Attribut von `MealZ.app`. Falls macOS wegen fehlender Rechte ablehnt, kann derselbe Befehl einmalig mit `sudo` ausgeführt werden. Verwende ihn nur für den offiziellen [MealZ-Release](https://github.com/ibimspumo/MealZ/releases/latest), dessen Herkunft du geprüft hast.

### Was MealZ kann

- **Onboarding:** Geführte Ersteinrichtung für Name, Nährwertrahmen, Kochalltag, Equipment, Vorlieben und Agentenpersönlichkeit. Das Onboarding kann in den Einstellungen erneut gestartet oder im Chat abgeschlossen werden.
- **Wochenplan:** Montag bis Sonntag planen, Gerichte manuell eintragen oder gemeinsam mit dem Agenten erstellen und vorhandene Favoriten wiederverwenden.
- **Rezeptkatalog:** Strukturierte Zutaten, Arbeitsschritte, Portionen, Zeiten, Tags, Bilder, Quellen, Nährwerte, Favoriten, Bewertungen und Kommentare lokal speichern.
- **Deterministische Einkaufsliste:** Zutaten für einen Datumsbereich aus dem Plan aggregieren, Einheiten normalisieren, Kategorien bilden, Artikel ergänzen und abhaken.
- **Persönlicher Agent:** Gestreamter Markdown-Chat über einen persistenten Codex-Thread mit sichtbaren Dynamic-Tool-Aufrufen für Rezepte, Planung, Einkauf, Profil und Memory.
- **Transparentes Memory:** Erinnerungen mit Art, Herkunft, Confidence und Status ansehen, bearbeiten, pausieren oder löschen.
- **Editierbare Agentendateien:** `PERSONA.md` steuert Ton und Verhalten; `MEMORY.md` ergänzt das strukturierte Memory um freien Langzeitkontext. Beide Dateien sind direkt in den Einstellungen bearbeitbar.
- **Local-first:** SQLite ist die lokale Quelle der Wahrheit für Rezepte, Planung, Einkauf, Profil, Bewertungen, Memories und Agentenmetadaten.

### Architektur

```text
React 19 + TypeScript
        │ Tauri Commands / Events
        ▼
Tauri 2 + Rust
   ├── MealzStore ── rusqlite ── lokale SQLite-Datenbank
   └── MealzAgent
          │ geordnetes JSONL über stdin/stdout
          ▼
      codex app-server
          ├── persistenter Thread
          ├── gestreamte Events
          ├── native Webrecherche
          └── Dynamic Tools ── Validierung ── MealzStore
```

Chattext wird nicht nachträglich geparst, um App-Daten zu verändern. Dauerhafte Aktionen laufen als typisierte Dynamic Tools durch die Rust-Domain und werden dort validiert. Eine ausführlichere Beschreibung liegt in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

### Voraussetzungen auf macOS

- macOS mit installierten Xcode Command Line Tools
- [Node.js](https://nodejs.org/) 20.19 oder neuer und `pnpm`
- aktuelles stabiles [Rust](https://rustup.rs/)
- installierte und authentifizierte Codex CLI

```bash
xcode-select --install

# Codex CLI installieren und per Browser anmelden
npm install --global @openai/codex
codex login
codex --version
```

MealZ spricht den lokalen Prozess `codex app-server` direkt an. Eine separate OpenAI-API-Key-Konfiguration in MealZ ist nicht erforderlich, wenn die Codex CLI bereits angemeldet ist.

### Lokale Entwicklung

```bash
# Repository klonen oder den Quellcode herunterladen, dann:
cd MealZ

corepack enable
pnpm install
pnpm tauri dev
```

`pnpm tauri dev` startet die echte native App mit SQLite und Codex App Server. `pnpm dev` startet nur die Browser-Oberfläche mit einem In-Memory-Demo-Adapter und ist nicht für die Prüfung echter Agenten- oder Persistenzflüsse gedacht.

### Produktionsbuild

```bash
pnpm tauri build
```

Die erzeugten macOS-Artefakte liegen anschließend unter `src-tauri/target/release/bundle/`. Lokale Builds sind ad-hoc signiert, aber nicht notarisiert.

### Updates & Releases

Installierte MealZ-Versionen können unter **Einstellungen → Updates & Releases** signierte GitHub-Releases prüfen, herunterladen und nach deiner Bestätigung installieren. Der Browser-Demo-Modus zeigt dafür bewusst einen Hinweis statt einen Netzwerkanruf auszuführen. Die Release-Automation baut ausschließlich für Macs mit Apple Silicon und signiert Updater-Artefakte mit `TAURI_SIGNING_PRIVATE_KEY`; für öffentlich verteilte macOS-Bundles bleiben Developer-ID-Signierung und Notarisierung ein separater Release-Schritt.

### Tests und Qualitätschecks

```bash
# Frontend
pnpm typecheck
pnpm test
pnpm build

# Rust-Domain, Integration und Lints
cd src-tauri
cargo test --lib
cargo clippy --lib --tests -- -D warnings
```

Die ignorierten Live-Tests starten einen echten Codex App Server und können einen Modellturn verbrauchen:

```bash
cd src-tauri
cargo test --lib -- --ignored --test-threads=1 --nocapture
```

Dafür muss die Codex CLI installiert und angemeldet sein.

### Lokale Daten und Datenschutz

MealZ betreibt keinen eigenen Cloud-Backend-Dienst. Die lokalen App-Daten befinden sich standardmäßig hier:

```text
~/Library/Application Support/de.agentz.mealz/
├── mealz.sqlite3
├── PERSONA.md
├── MEMORY.md
└── recipe-media/       # lokal erzeugte Rezeptbilder
```

- Die SQLite-Datenbank enthält Profil, Rezepte, Pläne, Einkauf, Bewertungen, Memories und Agentenmetadaten.
- `PERSONA.md` wird nur durch den Nutzer bearbeitet. `MEMORY.md` kann zusätzlich über freigegebene MealZ-Tools ergänzt werden.
- Der Agent erhält keine freie SQL-Schnittstelle. Änderungen sind auf die registrierten MealZ-Tools begrenzt.
- Für Modellantworten werden der jeweilige Chatinhalt und kuratierter Kontext über die lokal angemeldete Codex-Laufzeit verarbeitet. Dabei gelten die Datenschutz- und Kontoeinstellungen des verwendeten Codex-Zugangs.
- Webrecherche und externe Rezeptbilder können Netzwerkzugriffe zu Drittquellen auslösen.

Bitte behandle die App nicht als medizinische Beratung. Nährwerte und Zielwerte sind editierbare Orientierung für die Essensplanung.

### Projektstatus

MealZ befindet sich in aktiver Entwicklung (`0.1.0`, Alpha) und ist derzeit auf die persönliche Nutzung unter macOS ausgerichtet. Datenmodelle und interne Schnittstellen können sich noch ändern. Rezeptbilder werden aus belastbaren Quellen übernommen oder über die native Bildgenerierung des Codex App Servers erzeugt und lokal zwischengespeichert. Web- oder Mobile-Clients sowie notarisiert verteilte Releases sind mögliche spätere Ausbaustufen.

### Mitwirken

Issues und Pull Requests sind willkommen. Beiträge sollten die feste Architektur respektieren:

1. `codex app-server` bleibt die einzige Agentenlaufzeit.
2. Dauerhafte Änderungen erfolgen über validierte Dynamic Tools und SQLite, nicht durch das Parsen von Chattext.
3. Local-first und transparente, editierbare Memories haben Vorrang.
4. Vor einem Pull Request bitte Frontendtests, Rust-Tests und Clippy ausführen.

---

## English

MealZ is a personal, local-first Tauri 2 desktop app for macOS. It combines Monday-to-Sunday meal planning, a durable recipe catalog, deterministic shopping lists, and a transparent memory system with a persistent meal-planning agent.

### Highlights

- Guided onboarding for profile, nutrition frame, cooking routine, equipment, preferences, and agent personality
- Weekly planning with manual editing and agent-driven structured changes
- Recipe catalog with ingredients, steps, servings, nutrition, sources, images, favorites, ratings, and comments
- Date-range shopping lists aggregated from structured recipe ingredients
- Streaming Markdown chat backed exclusively by `codex app-server`
- Persistent Codex threads and visible Dynamic Tool activity
- Editable structured memories plus local `PERSONA.md` and `MEMORY.md`
- SQLite-backed local source of truth

### Quick start on macOS

Install Xcode Command Line Tools, Node.js 20.19+, pnpm, stable Rust, and an authenticated Codex CLI:

```bash
xcode-select --install
npm install --global @openai/codex
codex login

# Clone or download the repository, then:
cd MealZ
corepack enable
pnpm install
pnpm tauri dev
```

Use `pnpm tauri dev` for the real native application. Plain `pnpm dev` runs the browser UI with demo data and does not exercise Codex or SQLite persistence.

### Runtime and privacy

`codex app-server` is the only agent runtime. MealZ contains no Responses API path, Agents SDK integration, provider abstraction, or fallback model provider. App data is stored locally under `~/Library/Application Support/de.agentz.mealz/`. Recipe images use a verified source image or Codex App Server's native image generation; generated files are cached under `recipe-media/`. Model turns still send the current conversation and curated context through the authenticated Codex runtime, and web research or remote images may contact external services.

### Development checks

```bash
pnpm typecheck
pnpm test
pnpm build

cd src-tauri
cargo test --lib
cargo clippy --lib --tests -- -D warnings
```

MealZ is currently an early alpha focused on one macOS user. It is not medical software. Local builds are ad-hoc signed, not notarized; the built-in updater verifies signed GitHub release artifacts before installation.

## License

MealZ is released under the [MIT License](LICENSE). Copyright © 2026 Timo.
