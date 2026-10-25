import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Scissors } from "lucide-react";
import { useStore } from "../store";
import ReplayCard from "../components/ReplayCard";
import { GameMark } from "../components/GameBadge";
import { useToggleFavorite } from "./Library";
import type { GameType } from "../types";

export default function Clips() {
  const navigate = useNavigate();
  const replays = useStore((s) => s.replays);
  const toggleFavorite = useToggleFavorite();
  const [game, setGame] = useState<GameType | "all">("all");

  const clips = useMemo(
    () => replays.filter((r) => r.isClip && (game === "all" || r.game === game)),
    [replays, game],
  );
  const total = replays.filter((r) => r.isClip).length;

  return (
    <div className="h-full overflow-y-auto">
      <div className="max-w-[1500px] mx-auto px-8 py-7 space-y-6">
        <div className="flex items-end justify-between gap-4">
          <div>
            <h1 className="text-2xl font-bold">Clips</h1>
            <p className="text-sm text-fg-muted mt-1">Tes meilleurs moments, découpés depuis tes parties.</p>
          </div>
          {total > 0 && (
            <div className="flex gap-1.5">
              {(["all", "lol", "valorant"] as const).map((g) => (
                <button key={g} onClick={() => setGame(g)}
                  className={`chip ${game === g ? "bg-fg text-bg-base" : "bg-bg-card border border-line text-fg-muted hover:text-fg"}`}>
                  {g !== "all" && <GameMark game={g} size={16} />}
                  {g === "all" ? "Tous" : g === "lol" ? "LoL" : "Valorant"}
                </button>
              ))}
            </div>
          )}
        </div>

        {clips.length === 0 ? (
          <div className="card flex flex-col items-center justify-center text-center py-20 px-6 gap-4">
            <div className="w-16 h-16 rounded-2xl bg-accent-soft text-accent flex items-center justify-center">
              <Scissors size={28} />
            </div>
            <div>
              <p className="text-lg font-semibold">Pas encore de clip</p>
              <p className="text-sm text-fg-muted mt-1 max-w-md">
                Ouvre une partie, appuie sur <b>C</b> (ou le bouton Clip), ajuste le début et la fin, puis enregistre.
              </p>
            </div>
            <button className="btn-outline" onClick={() => navigate("/")}>Voir mes parties</button>
          </div>
        ) : (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-4">
            {clips.map((c) => (
              <ReplayCard key={c.id} replay={c} onOpen={() => navigate(`/replay/${c.id}`)}
                onToggleFavorite={() => toggleFavorite(c)} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
