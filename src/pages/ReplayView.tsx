import { useEffect, useMemo, useState } from "react";
import { useLocation, useNavigate, useParams } from "react-router-dom";
import { ArrowLeft, Star, FolderOpen, Trash2, Scissors, Clock, HardDrive, CalendarDays, Swords, Film } from "lucide-react";
import { useStore } from "../store";
import * as api from "../lib/tauri";
import VideoPlayer from "../components/VideoPlayer";
import GameBadge from "../components/GameBadge";
import { Thumbnail } from "../components/ReplayCard";
import { useToggleFavorite } from "./Library";
import { formatDateTime, formatDuration, formatRelative, formatSize, GAME_NAMES } from "../lib/format";
import type { Replay } from "../types";

function InfoRow({ icon, label, value }: { icon: React.ReactNode; label: string; value: string }) {
  return (
    <div className="flex items-center gap-3 py-2 text-sm">
      <span className="text-fg-subtle">{icon}</span>
      <span className="text-fg-muted flex-1">{label}</span>
      <span className="font-medium tabular">{value}</span>
    </div>
  );
}

function ClipRow({ clip, active, onOpen }: { clip: Replay; active: boolean; onOpen: () => void }) {
  return (
    <button onClick={onOpen}
      className={`w-full flex items-center gap-3 p-2 rounded-lg text-left transition-colors ${
        active ? "bg-accent-soft" : "hover:bg-bg-hover"
      }`}>
      <div className="relative w-24 aspect-video rounded-md overflow-hidden shrink-0 bg-bg-base">
        <Thumbnail replay={clip} />
        <span className="absolute bottom-0.5 right-0.5 px-1 rounded bg-black/70 text-[10px] text-white tabular">
          {formatDuration(clip.duration)}
        </span>
      </div>
      <div className="min-w-0">
        <p className="text-sm font-medium truncate">Clip {formatDuration(clip.duration)}</p>
        <p className="text-xs text-fg-subtle">{formatRelative(clip.date)}</p>
      </div>
    </button>
  );
}

export default function ReplayView() {
  const { id } = useParams();
  const navigate = useNavigate();
  const location = useLocation();
  const { replays, upsertReplay, removeReplays, refreshStats, toast, markSeen } = useStore();
  const toggleFavorite = useToggleFavorite();
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const replay = replays.find((r) => r.id === id);
  // The game this clip comes from (or the replay itself).
  const parentId = replay?.isClip ? replay.sourceId : replay?.id;
  const parent = replays.find((r) => r.id === parentId);
  const clips = useMemo(
    () => replays.filter((r) => r.isClip && r.sourceId && r.sourceId === parentId),
    [replays, parentId],
  );

  useEffect(() => {
    if (id) markSeen(id);
    setConfirmDelete(false);
  }, [id]);

  useEffect(() => {
    if (!confirmDelete) return;
    const t = setTimeout(() => setConfirmDelete(false), 3000);
    return () => clearTimeout(t);
  }, [confirmDelete]);

  if (!replay) {
    return (
      <div className="h-full flex flex-col items-center justify-center gap-3 text-fg-muted">
        <p>Cette vidéo n'existe plus.</p>
        <button className="btn-outline" onClick={() => navigate("/")}>Retour aux parties</button>
      </div>
    );
  }

  async function handleDelete() {
    if (!replay) return;
    if (!confirmDelete) {
      setConfirmDelete(true);
      return;
    }
    // Unmount the player first: Windows can't delete a file the <video> still has open.
    setDeleting(true);
    await new Promise((r) => setTimeout(r, 300));
    try {
      await api.deleteReplay(replay.id);
      removeReplays([replay.id]);
      refreshStats();
      toast("info", replay.isClip ? "Clip supprimé" : "Partie supprimée");
      navigate(replay.isClip && parent ? `/replay/${parent.id}` : replay.isClip ? "/clips" : "/");
    } catch (e) {
      toast("error", String(e));
      setDeleting(false);
      setConfirmDelete(false);
    }
  }

  // MemoryRouter: key is "default" on the first entry (nothing to go back to).
  const back = () => (location.key !== "default" ? navigate(-1) : navigate("/"));

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-[1600px] mx-auto px-6 py-5 space-y-4">
        {/* Header */}
        <div className="flex items-center gap-3">
          <button className="icon-btn" onClick={back} title="Retour (Échap)">
            <ArrowLeft size={18} />
          </button>
          <GameBadge game={replay.game} />
          <div className="min-w-0">
            <h1 className="text-lg font-bold truncate">
              {replay.isClip ? "Clip" : GAME_NAMES[replay.game]}
              <span className="text-fg-subtle font-normal"> · {formatDateTime(replay.date).toLowerCase()}</span>
            </h1>
          </div>
          <div className="flex-1" />
          <button className={`btn-outline ${replay.favorite ? "!text-warning !border-warning/40" : ""}`}
            onClick={() => toggleFavorite(replay)}>
            <Star size={15} fill={replay.favorite ? "currentColor" : "none"} />
            {replay.favorite ? "Favori" : "Ajouter aux favoris"}
          </button>
          <button className="icon-btn" title="Afficher dans l'explorateur"
            onClick={() => api.openReplayInExplorer(replay.id).catch((e) => toast("error", String(e)))}>
            <FolderOpen size={18} />
          </button>
          <button className={confirmDelete ? "btn bg-danger text-white hover:bg-danger/90" : "icon-btn hover:!text-danger"}
            title="Supprimer" onClick={handleDelete} disabled={deleting}>
            <Trash2 size={17} />
            {confirmDelete && "Confirmer"}
          </button>
        </div>

        <div className="grid grid-cols-1 xl:grid-cols-[1fr_340px] gap-5 items-start">
          <div>
            {!deleting && (
              <VideoPlayer
                replay={replay}
                onEscape={back}
                onClipCreated={(clip) => {
                  upsertReplay(clip);
                  refreshStats();
                }}
              />
            )}
            <p className="mt-3 text-xs text-fg-subtle">
              Raccourcis : <b>Espace</b> lecture · <b>← →</b> 5 s · <b>J L</b> 10 s · <b>, .</b> image par image ·{" "}
              <b>C</b> créer un clip · <b>F</b> plein écran · <b>M</b> muet
            </p>
          </div>

          <aside className="space-y-4">
            <div className="card p-4">
              <h2 className="text-sm font-semibold mb-1">Détails</h2>
              <div className="divide-y divide-line">
                <InfoRow icon={<CalendarDays size={15} />} label="Date" value={formatRelative(replay.date)} />
                <InfoRow icon={<Clock size={15} />} label="Durée" value={formatDuration(replay.duration)} />
                <InfoRow icon={<HardDrive size={15} />} label="Taille" value={formatSize(replay.size)} />
              </div>
            </div>

            {replay.isClip && parent && (
              <button className="card p-3 w-full flex items-center gap-3 text-left hover:border-line-strong transition-colors"
                onClick={() => navigate(`/replay/${parent.id}`)}>
                <div className="w-9 h-9 rounded-lg bg-bg-elevated flex items-center justify-center text-fg-muted">
                  <Film size={17} />
                </div>
                <div className="min-w-0">
                  <p className="text-sm font-medium">Voir la partie complète</p>
                  <p className="text-xs text-fg-subtle">{formatDuration(parent.duration)} · {formatRelative(parent.date)}</p>
                </div>
              </button>
            )}

            <div className="card p-4">
              <div className="flex items-center justify-between mb-2">
                <h2 className="text-sm font-semibold">Clips de cette partie</h2>
                <span className="text-xs text-fg-subtle tabular">{clips.length}</span>
              </div>
              {clips.length === 0 ? (
                <div className="py-6 text-center">
                  <Scissors size={22} className="mx-auto text-fg-subtle" />
                  <p className="text-sm text-fg-muted mt-2">Aucun clip</p>
                  <p className="text-xs text-fg-subtle mt-1">Appuie sur <b>C</b> pendant la lecture pour couper un moment.</p>
                </div>
              ) : (
                <div className="space-y-1 max-h-[420px] overflow-y-auto -mx-1 px-1">
                  {clips.map((c) => (
                    <ClipRow key={c.id} clip={c} active={c.id === replay.id} onOpen={() => navigate(`/replay/${c.id}`)} />
                  ))}
                </div>
              )}
            </div>

            <div className="card p-4 border-dashed">
              <div className="flex items-center gap-2 text-sm font-semibold text-fg-muted">
                <Swords size={15} /> Événements de la partie
              </div>
              <p className="text-xs text-fg-subtle mt-1.5">
                Kills, morts et objectifs apparaîtront ici et sur la timeline une fois l'API Riot branchée.
              </p>
            </div>
          </aside>
        </div>
      </div>
    </div>
  );
}
