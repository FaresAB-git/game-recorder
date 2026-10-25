import { memo, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Play, Star, Scissors } from "lucide-react";
import type { Replay } from "../types";
import { formatDuration, formatRelative, formatSize, formatTime, GAME_NAMES } from "../lib/format";
import { GameMark } from "./GameBadge";

interface Props {
  replay: Replay;
  clipCount?: number;
  onOpen: () => void;
  onToggleFavorite: () => void;
}

export function Thumbnail({ replay, className = "" }: { replay: Replay; className?: string }) {
  const [failed, setFailed] = useState(false);
  const src = replay.thumbnailPath && !failed ? convertFileSrc(replay.thumbnailPath) : null;
  const tint = replay.game === "lol" ? "from-lol/25" : "from-valorant/25";
  return src ? (
    <img src={src} alt="" draggable={false} onError={() => setFailed(true)}
      className={`w-full h-full object-cover ${className}`} />
  ) : (
    <div className={`w-full h-full bg-gradient-to-br ${tint} to-bg-elevated flex items-center justify-center ${className}`}>
      <GameMark game={replay.game} size={40} />
    </div>
  );
}

function ReplayCard({ replay, clipCount = 0, onOpen, onToggleFavorite }: Props) {
  const accent = replay.game === "lol" ? "bg-lol" : "bg-valorant";

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onOpen}
      onKeyDown={(e) => e.key === "Enter" && onOpen()}
      className="group relative card overflow-hidden cursor-pointer transition-all duration-200
                 hover:-translate-y-0.5 hover:border-line-strong hover:shadow-glow"
    >
      <div className="relative aspect-video overflow-hidden bg-bg-base">
        <Thumbnail replay={replay} className="transition-transform duration-300 group-hover:scale-[1.04]" />
        <div className="absolute inset-0 bg-gradient-to-t from-black/70 via-transparent to-black/20" />

        <div className="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
          <div className="w-12 h-12 rounded-full bg-white/15 backdrop-blur-md border border-white/20 flex items-center justify-center">
            <Play size={20} className="text-white ml-0.5" fill="currentColor" />
          </div>
        </div>

        <div className="absolute top-2 left-2 flex gap-1.5">
          {replay.isNew && (
            <span className="h-5 px-2 rounded-md bg-accent text-white text-[11px] font-bold tracking-wide flex items-center">
              NOUVEAU
            </span>
          )}
          {replay.isClip && (
            <span className="h-5 px-2 rounded-md bg-black/60 backdrop-blur text-white text-[11px] font-semibold flex items-center gap-1">
              <Scissors size={11} /> CLIP
            </span>
          )}
        </div>

        <button
          onClick={(e) => {
            e.stopPropagation();
            onToggleFavorite();
          }}
          title={replay.favorite ? "Retirer des favoris" : "Ajouter aux favoris"}
          className={`absolute top-2 right-2 w-7 h-7 rounded-md flex items-center justify-center backdrop-blur transition-all ${
            replay.favorite
              ? "bg-warning/20 text-warning opacity-100"
              : "bg-black/50 text-white/80 opacity-0 group-hover:opacity-100 hover:text-warning"
          }`}
        >
          <Star size={14} fill={replay.favorite ? "currentColor" : "none"} />
        </button>

        <span className="absolute bottom-2 right-2 h-5 px-1.5 rounded bg-black/70 text-white text-[11px] font-semibold tabular flex items-center">
          {formatDuration(replay.duration)}
        </span>
      </div>

      <div className={`absolute left-0 bottom-0 top-0 w-0.5 ${accent} opacity-0 group-hover:opacity-100 transition-opacity`} />

      <div className="p-3 flex items-center gap-3">
        <GameMark game={replay.game} size={28} />
        <div className="min-w-0 flex-1">
          <p className="text-sm font-semibold truncate">{GAME_NAMES[replay.game]}</p>
          <p className="text-xs text-fg-subtle truncate" title={new Date(replay.date).toLocaleString("fr-FR")}>
            {formatTime(replay.date)} · {formatRelative(replay.date)}
          </p>
        </div>
        <div className="text-right shrink-0">
          {clipCount > 0 && (
            <p className="text-xs text-accent font-medium flex items-center gap-1 justify-end">
              <Scissors size={11} /> {clipCount}
            </p>
          )}
          <p className="text-[11px] text-fg-subtle tabular">{formatSize(replay.size)}</p>
        </div>
      </div>
    </div>
  );
}

export default memo(ReplayCard);
