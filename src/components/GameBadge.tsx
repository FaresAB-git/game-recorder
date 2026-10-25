import type { GameType } from "../types";
import { GAME_SHORT } from "../lib/format";

/** Small game logo mark (letter in a shape, not the official logos). */
export function GameMark({ game, size = 20 }: { game: GameType; size?: number }) {
  if (game === "lol") {
    return (
      <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden>
        <path d="M12 1.5 21.5 7v10L12 22.5 2.5 17V7z" fill="rgba(200,170,110,0.16)" stroke="#c8aa6e" strokeWidth="1.5" />
        <path d="M9 7v10h6.5v-2H11.2V7z" fill="#c8aa6e" />
      </svg>
    );
  }
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden>
      <rect x="1.5" y="1.5" width="21" height="21" rx="5" fill="rgba(255,70,85,0.16)" stroke="#ff4655" strokeWidth="1.5" />
      <path d="M6 7h2.6l3.4 6.2L15.4 7H18l-6 10z" fill="#ff4655" />
    </svg>
  );
}

export default function GameBadge({ game, className = "" }: { game: GameType; className?: string }) {
  const color = game === "lol" ? "text-lol bg-lol-soft" : "text-valorant bg-valorant-soft";
  return (
    <span className={`inline-flex items-center gap-1.5 h-6 pl-1 pr-2 rounded-md text-xs font-semibold ${color} ${className}`}>
      <GameMark game={game} size={16} />
      {GAME_SHORT[game]}
    </span>
  );
}
