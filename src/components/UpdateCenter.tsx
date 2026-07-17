import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { CheckCircle2, Download, LoaderCircle, RefreshCw, RotateCcw, ShieldCheck } from "lucide-react";
import { Button } from "./Common";

type Phase = "idle" | "checking" | "available" | "downloading" | "downloaded" | "installing" | "installed" | "current" | "error";

const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function UpdateCenter() {
  const [currentVersion, setCurrentVersion] = useState("0.1.0");
  const [update, setUpdate] = useState<Update | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");
  const [downloaded, setDownloaded] = useState(0);
  const [total, setTotal] = useState<number | undefined>();
  const [error, setError] = useState("");

  useEffect(() => { if (isTauri()) void getVersion().then(setCurrentVersion).catch(() => undefined); }, []);

  const checkNow = async () => {
    setPhase("checking"); setError(""); setUpdate(null); setDownloaded(0); setTotal(undefined);
    if (!isTauri()) { setPhase("error"); setError("Die Update-Prüfung ist nur in der installierten Desktop-App verfügbar."); return; }
    try {
      const next = await check({ timeout: 30_000 });
      if (!next) { setPhase("current"); return; }
      setUpdate(next); setPhase("available");
    } catch (reason) {
      setError(readableError(reason)); setPhase("error");
    }
  };

  const onDownload = (event: DownloadEvent) => {
    if (event.event === "Started") { setTotal(event.data.contentLength); setDownloaded(0); }
    if (event.event === "Progress") setDownloaded((value) => value + event.data.chunkLength);
    if (event.event === "Finished") setPhase("downloaded");
  };

  const download = async () => {
    if (!update) return;
    setPhase("downloading"); setError("");
    try { await update.download(onDownload, { timeout: 120_000 }); setPhase("downloaded"); }
    catch (reason) { setError(readableError(reason)); setPhase("error"); }
  };

  const install = async () => {
    if (!update) return;
    setPhase("installing"); setError("");
    try { await update.install(); setPhase("installed"); }
    catch (reason) { setError(readableError(reason)); setPhase("error"); }
  };

  const percentage = total ? Math.min(100, Math.round((downloaded / total) * 100)) : undefined;
  return <section className="update-center" aria-labelledby="update-center-title">
    <header><span><RefreshCw size={20} /></span><div><h2 id="update-center-title">MealZ Updates</h2><p>Signierte Releases werden direkt von GitHub geprüft. Installation und Neustart bleiben vollständig in deiner Hand.</p></div><span className="version-badge">v{currentVersion}</span></header>
    <div className="update-status">
      <StatusIcon phase={phase} />
      <div>
        <strong>{statusTitle(phase, update?.version)}</strong>
        <p>{statusCopy(phase, update, downloaded, total)}</p>
        {update?.body && ["available", "downloading", "downloaded"].includes(phase) && <details><summary>Versionshinweise</summary><p>{update.body}</p></details>}
      </div>
    </div>
    {phase === "downloading" && <div className="update-progress"><div role="progressbar" aria-label="Update-Download" aria-valuemin={0} aria-valuemax={100} aria-valuenow={percentage}><i style={{ width: `${percentage ?? 12}%` }} /></div><span>{percentage !== undefined ? `${percentage} %` : formatBytes(downloaded)}</span></div>}
    {error && <p className="form-error" role="alert">{error}</p>}
    <div className="update-actions">
      {(["idle", "current", "error"] as Phase[]).includes(phase) && <Button tone="primary" icon={<RefreshCw size={15} />} onClick={checkNow}>Jetzt nach Updates suchen</Button>}
      {phase === "available" && <Button tone="primary" icon={<Download size={15} />} onClick={download}>Version {update?.version} herunterladen</Button>}
      {phase === "downloaded" && <Button tone="primary" icon={<ShieldCheck size={15} />} onClick={install}>Update installieren</Button>}
      {phase === "installed" && <Button tone="primary" icon={<RotateCcw size={15} />} onClick={() => void relaunch()}>MealZ neu starten</Button>}
    </div>
    <p className="update-security"><ShieldCheck size={14} />Jedes Update wird vor der Installation mit dem fest eingebauten MealZ-Signaturschlüssel geprüft.</p>
  </section>;
}

function StatusIcon({ phase }: { phase: Phase }) {
  if (["checking", "downloading", "installing"].includes(phase)) return <span className="update-status__icon is-busy"><LoaderCircle size={22} /></span>;
  if (["current", "downloaded", "installed"].includes(phase)) return <span className="update-status__icon is-success"><CheckCircle2 size={22} /></span>;
  return <span className="update-status__icon"><Download size={22} /></span>;
}

function statusTitle(phase: Phase, version?: string) {
  const labels: Record<Phase, string> = {
    idle: "Bereit für die Update-Prüfung", checking: "GitHub wird geprüft …", available: `Version ${version ?? ""} ist verfügbar`, downloading: "Update wird heruntergeladen …", downloaded: "Download abgeschlossen", installing: "Update wird installiert …", installed: "Update ist installiert", current: "MealZ ist aktuell", error: "Update-Prüfung nicht abgeschlossen",
  };
  return labels[phase];
}

function statusCopy(phase: Phase, update: Update | null, downloaded: number, total?: number) {
  if (phase === "available") return `Installiert ist ${update?.currentVersion}. Das neue Paket wird erst nach deiner Bestätigung heruntergeladen.`;
  if (phase === "downloading") return total ? `${formatBytes(downloaded)} von ${formatBytes(total)}` : `${formatBytes(downloaded)} geladen`;
  if (phase === "downloaded") return "Das signierte Paket ist bereit. Mit dem nächsten Schritt wird es lokal installiert.";
  if (phase === "installed") return "Starte MealZ neu, um die neue Version zu verwenden.";
  if (phase === "current") return "Für diese Installation liegt derzeit keine neuere stabile Version vor.";
  if (phase === "checking") return "Die aktuelle Release-Datei und ihre Signatur werden abgefragt.";
  if (phase === "installing") return "MealZ ersetzt die App-Dateien. Bitte schließe die Anwendung jetzt nicht.";
  return "Prüfe manuell, wann eine neue Version verfügbar ist. Es wird nichts automatisch installiert.";
}

function readableError(reason: unknown) {
  const text = reason instanceof Error ? reason.message : String(reason);
  if (/404|latest\.json|release/i.test(text)) return "Noch kein veröffentlichter MealZ-Release gefunden. Versuche es nach dem nächsten Release erneut.";
  if (/network|fetch|timed?\s*out/i.test(text)) return "GitHub konnte nicht erreicht werden. Prüfe deine Internetverbindung und versuche es erneut.";
  return `Update konnte nicht geprüft werden: ${text}`;
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}
