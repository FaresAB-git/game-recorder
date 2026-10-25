import { NavLink } from "react-router-dom";
import { Film, Scissors, Settings, FolderOpen, HardDrive } from "lucide-react";
import { useStore } from "../store";
import * as api from "../lib/tauri";
import { formatSize } from "../lib/format";
import { GameMark } from "./GameBadge";

const NAV = [
  { to: "/", label: "Parties", icon: Film, end: true },
  { to: "/clips", label: "Clips", icon: Scissors, end: false },
  { to: "/settings", label: "Paramètres", icon: Settings, end: false },
];

export default function Sidebar() {
  const replays = useStore((s) => s.replays);
  const stats = useStore((s) => s.libraryStats);
  const config = useStore((s) => s.config);
  const newCount = replays.filter((r) => r.isNew && !r.isClip).length;
  const clipCount = replays.filter((r) => r.isClip).length;

  const used = stats?.storage.usedBytes ?? 0;
  const max = stats?.storage.maxBytes || 1;
  const ratio = Math.min(used / max, 1);

  return (
    <aside className="w-56 shrink-0 flex flex-col bg-bg-surface border-r border-line">
      <nav className="flex flex-col gap-0.5 p-3">
        {NAV.map(({ to, label, icon: Icon, end }) => (
          <NavLink
            key={to}
            to={to}
            end={end}
            className={({ isActive }) =>
              `group flex items-center gap-3 h-10 px-3 rounded-lg text-sm font-medium transition-colors ${
                isActive ? "bg-accent-soft text-fg" : "text-fg-muted hover:text-fg hover:bg-bg-hover"
              }`
            }
          >
            {({ isActive }) => (
              <>
                <Icon size={18} className={isActive ? "text-accent" : ""} />
                <span>{label}</span>
                {to === "/" && newCount > 0 && (
                  <span className="ml-auto min-w-5 h-5 px-1.5 rounded-full bg-accent text-white text-[11px] font-semibold flex items-center justify-center">
                    {newCount}
                  </span>
                )}
                {to === "/clips" && clipCount > 0 && (
                  <span className="ml-auto text-xs text-fg-subtle tabular">{clipCount}</span>
                )}
              </>
            )}
          </NavLink>
        ))}
      </nav>

      <div className="px-3 mt-2">
        <p className="px-3 mb-2 text-[11px] font-semibold uppercase tracking-wider text-fg-subtle">Détection</p>
        <div className="flex flex-col gap-1">
          {(["lol", "valorant"] as const).map((g) => {
            const on = g === "lol" ? config.enableLol : config.enableValorant;
            return (
              <div key={g} className="flex items-center gap-2.5 h-8 px-3 text-sm text-fg-muted">
                <GameMark game={g} size={18} />
                <span className="flex-1">{g === "lol" ? "League of Legends" : "Valorant"}</span>
                <span className={`w-1.5 h-1.5 rounded-full ${on ? "bg-success" : "bg-fg-subtle"}`}
                  title={on ? "Détection active" : "Désactivé"} />
              </div>
            );
          })}
        </div>
      </div>

      <div className="mt-auto p-3">
        <div className="rounded-xl bg-bg-card border border-line p-3 space-y-2.5">
          <div className="flex items-center gap-2 text-xs text-fg-muted">
            <HardDrive size={14} />
            <span className="flex-1">Stockage</span>
            <button className="text-fg-subtle hover:text-fg transition-colors" title="Ouvrir le dossier des vidéos"
              onClick={() => api.openOutputFolder()}>
              <FolderOpen size={14} />
            </button>
          </div>
          <div className="h-1.5 rounded-full bg-bg-base overflow-hidden">
            <div
              className={`h-full rounded-full transition-all duration-500 ${
                ratio > 0.9 ? "bg-danger" : ratio > 0.75 ? "bg-warning" : "bg-accent"
              }`}
              style={{ width: `${ratio * 100}%` }}
            />
          </div>
          <div className="text-[11px] text-fg-subtle tabular leading-relaxed">
            <p><span className="text-fg-muted">{formatSize(used)}</span> sur {formatSize(max)}</p>
            {stats && <p>{formatSize(stats.storage.freeBytes)} libres sur le disque</p>}
          </div>
        </div>
      </div>
    </aside>
  );
}
