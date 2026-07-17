import { createPortal } from "react-dom";
import { useEffect, useRef, useState, type ButtonHTMLAttributes, type PropsWithChildren, type ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Check, CircleAlert, Info, LoaderCircle, X } from "lucide-react";
import type { Nutrition } from "../types";
import { useAppStore } from "../store";

type ButtonTone = "primary" | "secondary" | "quiet" | "danger";

export function Button({ tone = "secondary", icon, children, className = "", ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { tone?: ButtonTone; icon?: ReactNode }) {
  return (
    <button className={`button button--${tone} ${className}`.trim()} {...props}>
      {icon}<span>{children}</span>
    </button>
  );
}

export function IconButton({ label, className = "", ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return <button className={`icon-button ${className}`.trim()} aria-label={label} title={label} {...props} />;
}

export function PageHeader({ title, description, actions }: { title: string; description: string; actions?: ReactNode }) {
  return (
    <header className="page-header">
      <div>
        <h1>{title}</h1>
        <p>{description}</p>
      </div>
      {actions && <div className="page-header__actions">{actions}</div>}
    </header>
  );
}

export function Modal({ open, onClose, title, description, size = "medium", children, footer }: PropsWithChildren<{
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  size?: "small" | "medium" | "large";
  footer?: ReactNode;
}>) {
  const dialogRef = useRef<HTMLElement>(null);
  useEffect(() => {
    if (!open) return;
    const previous = document.activeElement as HTMLElement | null;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); onClose(); }
      if (event.key === "Tab" && dialogRef.current) {
        const focusable = [...dialogRef.current.querySelectorAll<HTMLElement>('button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])')].filter((node) => !node.hasAttribute("hidden"));
        if (!focusable.length) { event.preventDefault(); return; }
        const first = focusable[0]; const last = focusable[focusable.length - 1];
        if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
        else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
      }
    };
    const root = document.querySelector<HTMLElement>("#root");
    if (root) root.inert = true;
    document.addEventListener("keydown", onKeyDown);
    window.setTimeout(() => dialogRef.current?.focus(), 0);
    return () => { document.removeEventListener("keydown", onKeyDown); if (root) root.inert = false; previous?.focus(); };
  }, [onClose, open]);
  if (!open) return null;
  return createPortal(
    <div className="modal-layer" role="presentation" onMouseDown={(event) => { if (event.currentTarget === event.target) onClose(); }}>
      <section ref={dialogRef} tabIndex={-1} className={`modal modal--${size}`} role="dialog" aria-modal="true" aria-labelledby="modal-title">
        <header className="modal__header">
          <div>
            <h2 id="modal-title">{title}</h2>
            {description && <p>{description}</p>}
          </div>
          <IconButton label="Dialog schließen" onClick={onClose}><X size={18} /></IconButton>
        </header>
        <div className="modal__body">{children}</div>
        {footer && <footer className="modal__footer">{footer}</footer>}
      </section>
    </div>,
    document.body,
  );
}

export function ExternalLink({ href, className = "", children }: PropsWithChildren<{ href: string; className?: string }>) {
  const open = async (event: React.MouseEvent<HTMLAnchorElement>) => {
    event.preventDefault();
    if (!/^https?:\/\//i.test(href)) return;
    try { await openUrl(href); }
    catch { window.open(href, "_blank", "noopener,noreferrer"); }
  };
  return <a className={className} href={href} target="_blank" rel="noreferrer" onClick={open}>{children}</a>;
}

export function SafeRecipeImage({ src, alt, fallback }: { src?: string; alt: string; fallback: ReactNode }) {
  const imageRef = useRef<HTMLImageElement>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => { setFailed(false); }, [src]);
  if (!src || failed) return <>{fallback}</>;
  return <img ref={imageRef} src={recipeImageSrc(src)} alt={alt} loading="eager" decoding="async" onError={() => setFailed(true)} />;
}

/** Native media is an app-data path. Tauri's asset protocol keeps it out of the webview's file:// space. */
export function recipeImageSrc(value: string): string {
  if (/^(https?:|data:|asset:)/i.test(value)) return value;
  if (value.startsWith("/")) {
    try { return convertFileSrc(value); } catch { return value; }
  }
  return value;
}

export function NutritionStrip({ nutrition, compact = false }: { nutrition: Nutrition; compact?: boolean }) {
  const items = [
    ["kcal", Math.round(nutrition.calories)],
    ["Protein", `${Math.round(nutrition.protein)} g`],
    ["Kohlenhydrate", `${Math.round(nutrition.carbs)} g`],
    ["Fett", `${Math.round(nutrition.fat)} g`],
    ["Ballaststoffe", `${Math.round(nutrition.fiber)} g`],
  ];
  return (
    <dl className={`nutrition-strip ${compact ? "nutrition-strip--compact" : ""}`}>
      {items.map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}
    </dl>
  );
}

export function EmptyState({ icon, title, children, action }: PropsWithChildren<{ icon: ReactNode; title: string; action?: ReactNode }>) {
  return (
    <div className="empty-state">
      <span className="empty-state__icon">{icon}</span>
      <h3>{title}</h3>
      <div className="empty-state__copy">{children}</div>
      {action}
    </div>
  );
}

export function Skeleton({ lines = 3 }: { lines?: number }) {
  return <div className="skeleton" aria-label="Inhalt wird geladen">{Array.from({ length: lines }, (_, index) => <span key={index} />)}</div>;
}

export function ToastRegion() {
  const toasts = useAppStore((state) => state.toasts);
  const dismiss = useAppStore((state) => state.dismissToast);
  const icons = { success: <Check size={17} />, error: <CircleAlert size={17} />, info: <Info size={17} /> };
  return createPortal(
    <div className="toast-region" aria-live="polite">
      {toasts.map((toast) => (
        <div className={`toast toast--${toast.tone}`} key={toast.id}>
          <span className="toast__icon">{icons[toast.tone]}</span>
          <div><strong>{toast.title}</strong>{toast.detail && <p>{toast.detail}</p>}</div>
          <IconButton label="Meldung schließen" onClick={() => dismiss(toast.id)}><X size={15} /></IconButton>
        </div>
      ))}
    </div>,
    document.body,
  );
}

export function InlineLoader({ label = "Wird geladen" }: { label?: string }) {
  return <span className="inline-loader"><LoaderCircle size={16} aria-hidden="true" />{label}</span>;
}

export function formatAmount(value: number) {
  return Number.isInteger(value) ? String(value) : value.toLocaleString("de-DE", { maximumFractionDigits: 1 });
}
