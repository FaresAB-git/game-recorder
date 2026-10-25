import { useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { Play, Star, Clock, Film, Scissors, Loader2, Gamepad2 } from "lucide-react";
import { useStore } from "../store";
import * as api from "../lib/tauri";
import ReplayCard, { Thumbnail } from "../components/ReplayCard";
import { GameMark } from "../components/GameBadge";
import { useElapsed } from "../components/RecordingPill";
import type { FilterType, Replay, SortType } from "../types";
import {
  dayKey, dayLabel, formatDateTime, formatDuration, formatLongDuration, formatRelative, formatSize, GAME_NAMES,
} from "../lib/format";

const SORTS: { key: SortType; label: string }[] = [
  { key: "date-desc", label: "Plus récentes" },
  { key: "date-asc", label: "Plus anciennes" },
  { key: "duration", label: "Plus longues" },
  { key: "size", label: "Plus lourdes" },
];

export function useToggleFavorite() {
  const updateReplay = useStore((s) => s.updateReplay);
  const toast = useStore((s) => s.toast);
  return async (r: Replay) => {
    try {
      updateReplay(r.id, { favorite: await api.toggleFavorite(r.id) });
    } catch (e) {
      toast("error", String(e));
    }
  };
}

function Stat({ icon, label, value }: { icon: React.ReactNode; label: string; value: string }) {
  return (
    <div className="card px-4 py-3 flex items-center gap-3">
      <div className="w-9 h-9 rounded-lg bg-bg-elevated text-fg-muted flex items-center justify-center">{icon}</div>
      <div>
        <p className="text-lg font-bold leading-tight tabular">{value}</p>
        <p className="text-xs text-fg-subtle">{label}</p>
      </div>
    </div>
  );
}

function LiveBanner() {
  const status = useStore((s) => s.recordingStatus);
  const elapsed = useElapsed(status.isRecording ? status.startedAt : null);
  if (!status.isRecording && !status.isProcessing) return null;

  return (
    <div className="relative overflow-hidden card p-4 flex items-center gap-4 border-live/30 animate-slide-up">
      <div className="absolute inset-0 bg-gradient-to-r from-live/10 to-transparent pointer-events-none" />
      {status.isProcessing ? (
        <Loader2 size={22} className="text-accent animate-spin" />
      ) : (
        <span className="relative flex h-3 w-3">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-live opacity-70" />
          <span className="relative inline-flex rounded-full h-3 w-3 bg-live" />
        </span>
      )}
      <div className="relative">
        <p className="font-semibold">
          {status.isProcessing ? "Sauvegarde de la partie…" : `Enregistrement en cours — ${status.game ? GAME_NAMES[status.game] : ""}`}
        </p>
        <p className="text-sm text-fg-muted">
          {status.isProcessing
            ? "Assemblage de la vidéo et du son, ça ne prend que quelques secondes."
            : <>La vidéo apparaîtra ici à la fin de la partie · <span className="tabular">{formatDuration(elapsed)}</span></>}
        </p>
      </div>
    </div>
  );
}

function Hero({ replay, clipCount, onOpen }: { replay: Replay; clipCount: number; onOpen: () => void }) {
  const glow = replay.game === "lol" ? "from-lol/20" : "from-valorant/20";
  return (
    <div className="relative card overflow-hidden flex cursor-pointer group" onClick={onOpen}>
      <div className={`absolute inset-0 bg-gradient-to-r ${glow} via-transparent to-transparent pointer-events-none`} />
      <div className="relative w-[46%] max-w-xl aspect-video shrink-0 overflow-hidden bg-bg-base">
        <Thumbnail replay={replay} className="transition-transform duration-500 group-hover:scale-[1.03]" />
        <div className="absolute inset-0 bg-gradient-to-r from-transparent to-bg-card/90" />
        <span className="absolute bottom-3 left-3 h-6 px-2 rounded-md bg-black/70 text-white text-xs font-semibold tabular flex items-center">
          {formatDuration(replay.duration)}
        </span>
      </div>
      <div className="relative flex-1 p-6 flex flex-col justify-center gap-3 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-[11px] font-bold uppercase tracking-wider text-accent">Dernière partie</span>
          {replay.isNew && <span className="h-5 px-2 rounded-md bg-accent text-white text-[11px] font-bold flex items-center">NOUVEAU</span>}
        </div>
        <h2 className="text-2xl font-bold truncate">{GAME_NAMES[replay.game]}</h2>
        <p className="text-sm text-fg-muted">{formatDateTime(replay.date)} · {formatRelative(replay.date)}</p>
        <div className="flex items-center gap-4 text-sm text-fg-muted">
          <span className="flex items-center gap-1.5"><Clock size={14} /> {formatDuration(replay.duration)}</span>
          <span className="flex items-center gap-1.5"><Scissors size={14} /> {clipCount} clip{clipCount > 1 ? "s" : ""}</span>
          <span>{formatSize(replay.size)}</span>
        </div>
        <div className="pt-1">
          <button className="btn-primary h-10 px-5" onClick={(e) => { e.stopPropagation(); onOpen(); }}>
            <Play size={16} fill="currentColor" /> Revoir la partie
          </button>
        </div>
      </div>
    </div>
  );
}

export default function Library() {
  const navigate = useNavigate();
  const { replays, filter, sort, isLoading, setFilter, setSort, markSeen } = useStore();
  const toggleFavorite = useToggleFavorite();

  const games = useMemo(() => replays.filter((r) => !r.isClip), [replays]);
  const clipCounts = useMemo(() => {
    const m = new Map<string, number>();
    for (const r of replays) if (r.isClip && r.sourceId) m.set(r.sourceId, (m.get(r.sourceId) ?? 0) + 1);
    return m;
  }, [replays]);

  const filtered = useMemo(() => {
    let list = games;
    if (filter === "lol" || filter === "valorant") list = list.filter((r) => r.game === filter);
    else if (filter === "favorites") list = list.filter((r) => r.favorite);
    return [...list].sort((a, b) => {
      switch (sort) {
        case "date-asc": return a.date.localeCompare(b.date);
        case "duration": return b.duration - a.duration;
        case "size": return b.size - a.size;
        default: return b.date.localeCompare(a.date);
      }
    });
  }, [games, filter, sort]);

  // Group by day only when sorted by date (otherwise the order would be broken).
  const groups = useMemo(() => {
    if (sort !== "date-desc" && sort !== "date-asc") return [{ key: "all", label: "", items: filtered }];
    const out: { key: string; label: string; items: Replay[] }[] = [];
    for (const r of filtered) {
      const k = dayKey(r.date);
      const last = out[out.length - 1];
      if (last?.key === k) last.items.push(r);
      else out.push({ key: k, label: dayLabel(r.date), items: [r] });
    }
    return out;
  }, [filtered, sort]);

  const open = (r: Replay) => {
    markSeen(r.id);
    navigate(`/replay/${r.id}`);
  };

  const latest = games[0];
  const totalTime = games.reduce((s, r) => s + r.duration, 0);
  const counts: Record<FilterType, number> = {
    all: games.length,
    lol: games.filter((r) => r.game === "lol").length,
    valorant: games.filter((r) => r.game === "valorant").length,
    favorites: games.filter((r) => r.favorite).length,
  };
  const FILTERS: { key: FilterType; label: string }[] = [
    { key: "all", label: "Toutes" },
    { key: "lol", label: "League of Legends" },
    { key: "valorant", label: "Valorant" },
    { key: "favorites", label: "Favoris" },
  ];

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-[1500px] mx-auto px-8 py-7 space-y-7">
        <div className="flex items-end justify-between gap-4">
          <div>
            <h1 className="text-2xl font-bold">Mes parties</h1>
            <p className="text-sm text-fg-muted mt-1">Tes games sont enregistrées automatiquement dès qu'elles commencent.</p>
          </div>
        </div>

        <LiveBanner />

        {isLoading ? (
          <div className="flex items-center justify-center py-32 text-fg-muted gap-3">
            <Loader2 className="animate-spin" size={20} /> Chargement…
          </div>
        ) : games.length === 0 ? (
          <div className="card flex flex-col items-center justify-center text-center py-20 px-6 gap-4">
            <div className="w-16 h-16 rounded-2xl bg-accent-soft text-accent flex items-center justify-center">
              <Gamepad2 size={30} />
            </div>
            <div>
              <p className="text-lg font-semibold">Aucune partie pour l'instant</p>
              <p className="text-sm text-fg-muted mt-1 max-w-md">
                Lance une partie de League of Legends ou de Valorant : l'enregistrement démarre tout seul
                et la vidéo apparaîtra ici à la fin.
              </p>
            </div>
          </div>
        ) : (
          <>
            <div className="grid grid-cols-2 xl:grid-cols-4 gap-3">
              <Stat icon={<Film size={18} />} label="Parties enregistrées" value={String(games.length)} />
              <Stat icon={<Clock size={18} />} label="Temps de jeu enregistré" value={formatLongDuration(totalTime)} />
              <Stat icon={<Scissors size={18} />} label="Clips créés" value={String(replays.length - games.length)} />
              <Stat icon={<Star size={18} />} label="Favoris" value={String(counts.favorites)} />
            </div>

            {latest && filter === "all" && (
              <Hero replay={latest} clipCount={clipCounts.get(latest.id) ?? 0} onOpen={() => open(latest)} />
            )}

            <div className="flex items-center justify-between gap-4 flex-wrap">
              <div className="flex items-center gap-1.5">
                {FILTERS.map(({ key, label }) => (
                  <button key={key} onClick={() => setFilter(key)}
                    className={`chip ${filter === key ? "bg-fg text-bg-base" : "bg-bg-card border border-line text-fg-muted hover:text-fg"}`}>
                    {(key === "lol" || key === "valorant") && <GameMark game={key} size={16} />}
                    {label}
                    <span className={`text-xs tabular ${filter === key ? "text-bg-base/60" : "text-fg-subtle"}`}>{counts[key]}</span>
                  </button>
                ))}
              </div>
              <select value={sort} onChange={(e) => setSort(e.target.value as SortType)} className="input pr-8">
                {SORTS.map((s) => <option key={s.key} value={s.key}>{s.label}</option>)}
              </select>
            </div>

            {filtered.length === 0 ? (
              <p className="text-center text-fg-muted py-16">
                {filter === "favorites" ? "Aucun favori — clique sur l'étoile d'une partie pour la garder." : "Aucune partie pour ce jeu."}
              </p>
            ) : (
              groups.map((g) => (
                <section key={g.key} className="space-y-3">
                  {g.label && (
                    <h3 className="text-sm font-semibold text-fg-muted">
                      {g.label} <span className="text-fg-subtle font-normal">· {g.items.length}</span>
                    </h3>
                  )}
                  <div className="grid grid-cols-[repeat(auto-fill,minmax(270px,1fr))] gap-4">
                    {g.items.map((r) => (
                      <ReplayCard key={r.id} replay={r} clipCount={clipCounts.get(r.id) ?? 0}
                        onOpen={() => open(r)} onToggleFavorite={() => toggleFavorite(r)} />
                    ))}
                  </div>
                </section>
              ))
            )}
          </>
        )}
      </div>
    </div>
  );
}
