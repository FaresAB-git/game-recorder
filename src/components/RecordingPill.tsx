import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { useStore } from "../store";
import { formatDuration, GAME_NAMES } from "../lib/format";

export function useElapsed(startedAt?: string | null): number {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (!startedAt) return;
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, [startedAt]);
  if (!startedAt) return 0;
  return Math.max(0, (now - new Date(startedAt).getTime()) / 1000);
}

/** Live status shown in the title bar while a game is recorded / saved. */
export default function RecordingPill() {
  const status = useStore((s) => s.recordingStatus);
  const elapsed = useElapsed(status.isRecording ? status.startedAt : null);

  if (status.isProcessing) {
    return (
      <div className="flex items-center gap-2 h-6 px-3 rounded-full bg-accent-soft text-accent text-xs font-medium animate-fade-in">
        <Loader2 size={13} className="animate-spin" />
        Sauvegarde de la partie…
      </div>
    );
  }
  if (!status.isRecording) return null;

  return (
    <div className="flex items-center gap-2 h-6 px-3 rounded-full bg-live/15 text-live text-xs font-semibold animate-fade-in">
      <span className="relative flex h-2 w-2">
        <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-live opacity-70" />
        <span className="relative inline-flex rounded-full h-2 w-2 bg-live" />
      </span>
      REC
      <span className="text-fg-muted font-medium">
        {status.game ? GAME_NAMES[status.game] : ""} · <span className="tabular">{formatDuration(elapsed)}</span>
      </span>
    </div>
  );
}
