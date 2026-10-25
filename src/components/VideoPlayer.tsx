import { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  Play, Pause, RotateCcw, RotateCw, Volume2, VolumeX, Maximize, Minimize, Scissors, Loader2, Check, X, Repeat,
} from "lucide-react";
import type { Replay } from "../types";
import { createClip } from "../lib/tauri";
import { formatTimecode } from "../lib/format";
import { useStore } from "../store";

/** Future: kills/objectives from the Riot API will be drawn on the timeline. */
export interface TimelineMarker {
  time: number;
  label: string;
  color?: string;
}

interface Props {
  replay: Replay;
  markers?: TimelineMarker[];
  onClipCreated?: (clip: Replay) => void;
  onEscape?: () => void;
}

const SPEEDS = [0.25, 0.5, 1, 1.5, 2];
const DEFAULT_CLIP = 30; // seconds
const VOLUME_KEY = "player.volume";

type Drag = "seek" | "start" | "end" | null;

function readVolume(): number {
  try {
    const v = parseFloat(localStorage.getItem(VOLUME_KEY) ?? "");
    return Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 1;
  } catch {
    return 1;
  }
}

export default function VideoPlayer({ replay, markers = [], onClipCreated, onEscape }: Props) {
  const toast = useStore((s) => s.toast);
  const videoRef = useRef<HTMLVideoElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const trackRef = useRef<HTMLDivElement>(null);
  const hideTimer = useRef<ReturnType<typeof setTimeout>>();

  const [playing, setPlaying] = useState(false);
  const [time, setTime] = useState(0);
  const [duration, setDuration] = useState(replay.duration || 0);
  const [volume, setVolume] = useState(readVolume);
  const [muted, setMuted] = useState(false);
  const [rate, setRate] = useState(1);
  const [fullscreen, setFullscreen] = useState(false);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [hover, setHover] = useState<{ x: number; t: number } | null>(null);
  const [drag, setDrag] = useState<Drag>(null);
  const [error, setError] = useState(false);

  const [clipMode, setClipMode] = useState(false);
  const [range, setRange] = useState<[number, number]>([0, 0]);
  const [loopRange, setLoopRange] = useState(false);
  const [saving, setSaving] = useState(false);

  const src = convertFileSrc(replay.path);
  const v = () => videoRef.current;

  // Reset when switching video.
  useEffect(() => {
    setClipMode(false);
    setTime(0);
    setError(false);
    setDuration(replay.duration || 0);
  }, [replay.id]);

  useEffect(() => {
    if (videoRef.current) videoRef.current.volume = volume;
  }, []);

  const pokeControls = useCallback(() => {
    setControlsVisible(true);
    clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => {
      if (videoRef.current && !videoRef.current.paused) setControlsVisible(false);
    }, 2500);
  }, []);

  const togglePlay = useCallback(() => {
    const el = v();
    if (!el) return;
    if (el.paused) el.play().catch(() => {});
    else el.pause();
  }, []);

  const seekTo = useCallback((t: number) => {
    const el = v();
    if (!el || !Number.isFinite(el.duration)) return;
    el.currentTime = Math.min(Math.max(0, t), el.duration);
    setTime(el.currentTime);
  }, []);

  const toggleFullscreen = useCallback(() => {
    if (document.fullscreenElement) document.exitFullscreen();
    else wrapRef.current?.requestFullscreen();
  }, []);

  const enterClipMode = useCallback(() => {
    const el = v();
    const d = el?.duration || duration;
    const now = el?.currentTime ?? 0;
    // Default: the last 30 s up to the playhead, like an instant replay.
    const end = Math.min(d, Math.max(now, Math.min(DEFAULT_CLIP, d)));
    setRange([Math.max(0, end - DEFAULT_CLIP), end]);
    setClipMode(true);
  }, [duration]);

  const exitClipMode = useCallback(() => {
    setClipMode(false);
    setLoopRange(false);
  }, []);

  const saveClip = useCallback(async () => {
    const [a, b] = range;
    if (b - a < 1 || saving) return;
    setSaving(true);
    try {
      const clip = await createClip(replay.id, a, b);
      toast("success", "Clip enregistré");
      onClipCreated?.(clip);
      exitClipMode();
    } catch (err) {
      toast("error", `Échec du clip : ${err}`);
    } finally {
      setSaving(false);
    }
  }, [range, saving, replay.id, onClipCreated, exitClipMode, toast]);

  // Keyboard shortcuts
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const el = v();
      const target = e.target as HTMLElement;
      if (!el || target.closest("input, select, textarea")) return;
      if (e.ctrlKey || e.altKey || e.metaKey) return;
      const frame = 1 / 60;

      switch (e.key) {
        case " ":
        case "k":
          e.preventDefault();
          togglePlay();
          break;
        case "ArrowLeft":
          e.preventDefault();
          seekTo(el.currentTime - (e.shiftKey ? 1 : 5));
          break;
        case "ArrowRight":
          e.preventDefault();
          seekTo(el.currentTime + (e.shiftKey ? 1 : 5));
          break;
        case "j":
          seekTo(el.currentTime - 10);
          break;
        case "l":
          seekTo(el.currentTime + 10);
          break;
        case ",":
          el.pause();
          seekTo(el.currentTime - frame);
          break;
        case ".":
          el.pause();
          seekTo(el.currentTime + frame);
          break;
        case "ArrowUp":
          e.preventDefault();
          el.volume = Math.min(1, el.volume + 0.1);
          break;
        case "ArrowDown":
          e.preventDefault();
          el.volume = Math.max(0, el.volume - 0.1);
          break;
        case "f":
          toggleFullscreen();
          break;
        case "m":
          el.muted = !el.muted;
          break;
        case "c":
          clipMode ? exitClipMode() : enterClipMode();
          break;
        case "[":
          if (clipMode) setRange(([, b]) => [Math.min(el.currentTime, b - 1), b]);
          break;
        case "]":
          if (clipMode) setRange(([a]) => [a, Math.max(el.currentTime, a + 1)]);
          break;
        case "Enter":
          if (clipMode) saveClip();
          break;
        case "Escape":
          if (document.fullscreenElement) return;
          if (clipMode) exitClipMode();
          else onEscape?.();
          break;
        default:
          return;
      }
      pokeControls();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [clipMode, togglePlay, seekTo, toggleFullscreen, enterClipMode, exitClipMode, saveClip, onEscape, pokeControls]);

  useEffect(() => {
    const onFs = () => setFullscreen(!!document.fullscreenElement);
    document.addEventListener("fullscreenchange", onFs);
    return () => document.removeEventListener("fullscreenchange", onFs);
  }, []);

  // Loop the selected range while previewing it.
  useEffect(() => {
    if (!clipMode || !loopRange) return;
    if (time >= range[1] || time < range[0] - 0.25) seekTo(range[0]);
  }, [time, clipMode, loopRange, range, seekTo]);

  // ── Timeline dragging ─────────────────────────────────────────────────────
  const timeAt = (clientX: number) => {
    const rect = trackRef.current?.getBoundingClientRect();
    if (!rect || !duration) return 0;
    return Math.min(Math.max(0, (clientX - rect.left) / rect.width), 1) * duration;
  };

  useEffect(() => {
    if (!drag) return;
    function move(e: PointerEvent) {
      const t = timeAt(e.clientX);
      if (drag === "seek") seekTo(t);
      else if (drag === "start") setRange(([, b]) => [Math.min(t, b - 1), b]);
      else if (drag === "end") setRange(([a]) => [a, Math.max(t, a + 1)]);
    }
    function up() {
      setDrag(null);
    }
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  }, [drag, duration]);

  const pct = (t: number) => (duration > 0 ? (t / duration) * 100 : 0);
  const clipLen = range[1] - range[0];

  return (
    <div className="flex flex-col gap-3">
      <div
        ref={wrapRef}
        className={`relative bg-black overflow-hidden group/player ${fullscreen ? "" : "rounded-xl border border-line"} ${
          controlsVisible ? "" : "cursor-none"
        }`}
        onMouseMove={pokeControls}
        onMouseLeave={() => playing && setControlsVisible(false)}
      >
        <video
          ref={videoRef}
          src={src}
          className={`w-full bg-black ${fullscreen ? "h-screen" : "aspect-video"}`}
          onClick={togglePlay}
          onDoubleClick={toggleFullscreen}
          onPlay={() => {
            setPlaying(true);
            pokeControls();
          }}
          onPause={() => {
            setPlaying(false);
            setControlsVisible(true);
          }}
          onTimeUpdate={(e) => !drag && setTime(e.currentTarget.currentTime)}
          onLoadedMetadata={(e) => {
            const d = e.currentTarget.duration;
            if (Number.isFinite(d)) setDuration(d);
            e.currentTarget.volume = volume;
          }}
          onVolumeChange={(e) => {
            setVolume(e.currentTarget.volume);
            setMuted(e.currentTarget.muted);
            try {
              localStorage.setItem(VOLUME_KEY, String(e.currentTarget.volume));
            } catch {}
          }}
          onError={() => setError(true)}
          autoPlay
        />

        {error && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 bg-black/80 text-center p-6">
            <p className="font-semibold">Impossible de lire cette vidéo</p>
            <p className="text-sm text-fg-muted">Le fichier a peut-être été déplacé ou supprimé.</p>
          </div>
        )}

        {!playing && !error && (
          <button onClick={togglePlay}
            className="absolute inset-0 m-auto w-16 h-16 rounded-full bg-black/50 backdrop-blur border border-white/15 flex items-center justify-center hover:scale-105 transition-transform">
            <Play size={26} className="text-white ml-1" fill="currentColor" />
          </button>
        )}

        {/* Controls overlay */}
        <div
          className={`absolute inset-x-0 bottom-0 px-4 pb-3 pt-10 bg-gradient-to-t from-black/90 via-black/50 to-transparent transition-opacity duration-200 ${
            controlsVisible || clipMode ? "opacity-100" : "opacity-0 pointer-events-none"
          }`}
        >
          {/* Timeline */}
          <div
            ref={trackRef}
            className="relative h-5 flex items-center cursor-pointer group/track"
            onPointerDown={(e) => {
              if (e.button !== 0) return;
              seekTo(timeAt(e.clientX));
              setDrag("seek");
            }}
            onPointerMove={(e) => {
              const rect = trackRef.current!.getBoundingClientRect();
              setHover({ x: e.clientX - rect.left, t: timeAt(e.clientX) });
            }}
            onPointerLeave={() => setHover(null)}
          >
            <div className="relative w-full h-1 group-hover/track:h-1.5 transition-all rounded-full bg-white/20 overflow-hidden">
              <div className="absolute inset-y-0 left-0 bg-accent" style={{ width: `${pct(time)}%` }} />
              {clipMode && (
                <div className="absolute inset-y-0 bg-white/35"
                  style={{ left: `${pct(range[0])}%`, width: `${pct(clipLen)}%` }} />
              )}
            </div>

            {markers.map((m, i) => (
              <div key={i} title={m.label}
                className="absolute top-1/2 -translate-y-1/2 w-1 h-3 rounded-full pointer-events-auto"
                style={{ left: `${pct(m.time)}%`, background: m.color ?? "#f5b544" }} />
            ))}

            {/* Playhead */}
            <div className="absolute top-1/2 -translate-y-1/2 -translate-x-1/2 w-3 h-3 rounded-full bg-white shadow pointer-events-none scale-0 group-hover/track:scale-100 transition-transform"
              style={{ left: `${pct(time)}%` }} />

            {clipMode && (
              <>
                {(["start", "end"] as const).map((which) => (
                  <div key={which}
                    onPointerDown={(e) => {
                      e.stopPropagation();
                      setDrag(which);
                    }}
                    className="absolute top-1/2 -translate-y-1/2 -translate-x-1/2 w-3 h-6 rounded-[4px] bg-white border-2 border-accent cursor-ew-resize shadow-lg z-10"
                    style={{ left: `${pct(which === "start" ? range[0] : range[1])}%` }} />
                ))}
              </>
            )}

            {hover && duration > 0 && (
              <div className="absolute -top-7 -translate-x-1/2 px-1.5 py-0.5 rounded bg-black/90 text-[11px] text-white tabular pointer-events-none"
                style={{ left: hover.x }}>
                {formatTimecode(hover.t)}
              </div>
            )}
          </div>

          {/* Buttons */}
          <div className="flex items-center gap-1 mt-1.5 text-white">
            <button className="player-btn" onClick={togglePlay} title="Lecture / pause (Espace)">
              {playing ? <Pause size={20} fill="currentColor" /> : <Play size={20} fill="currentColor" />}
            </button>
            <button className="player-btn" onClick={() => seekTo(time - 10)} title="-10 s (J)">
              <RotateCcw size={18} />
            </button>
            <button className="player-btn" onClick={() => seekTo(time + 10)} title="+10 s (L)">
              <RotateCw size={18} />
            </button>

            <div className="flex items-center gap-1 group/vol">
              <button className="player-btn" onClick={() => v() && (v()!.muted = !muted)} title="Muet (M)">
                {muted || volume === 0 ? <VolumeX size={18} /> : <Volume2 size={18} />}
              </button>
              <input type="range" min={0} max={1} step={0.02} value={muted ? 0 : volume}
                onChange={(e) => {
                  const el = v();
                  if (!el) return;
                  el.volume = parseFloat(e.target.value);
                  el.muted = el.volume === 0;
                }}
                className="w-0 opacity-0 group-hover/vol:w-20 group-hover/vol:opacity-100 transition-all accent-white h-1 cursor-pointer" />
            </div>

            <span className="ml-2 text-xs tabular text-white/80">
              {formatTimecode(time)} <span className="text-white/40">/ {formatTimecode(duration)}</span>
            </span>

            <div className="flex-1" />

            <div className="flex items-center rounded-lg bg-white/10 p-0.5">
              {SPEEDS.map((s) => (
                <button key={s}
                  onClick={() => {
                    const el = v();
                    if (el) el.playbackRate = s;
                    setRate(s);
                  }}
                  className={`h-6 px-1.5 rounded-md text-[11px] font-semibold tabular transition-colors ${
                    rate === s ? "bg-white text-black" : "text-white/70 hover:text-white"
                  }`}>
                  {s}×
                </button>
              ))}
            </div>

            <button onClick={clipMode ? exitClipMode : enterClipMode} title="Créer un clip (C)"
              className={`ml-2 h-8 px-3 rounded-lg flex items-center gap-1.5 text-xs font-semibold transition-colors ${
                clipMode ? "bg-accent text-white" : "bg-white/10 hover:bg-white/20"
              }`}>
              <Scissors size={14} /> Clip
            </button>
            <button className="player-btn" onClick={toggleFullscreen} title="Plein écran (F)">
              {fullscreen ? <Minimize size={18} /> : <Maximize size={18} />}
            </button>
          </div>
        </div>
      </div>

      {/* Clip editor */}
      {clipMode && (
        <div className="card p-3 flex items-center gap-3 animate-slide-up">
          <div className="w-9 h-9 rounded-lg bg-accent-soft text-accent flex items-center justify-center">
            <Scissors size={17} />
          </div>
          <div className="min-w-0">
            <p className="text-sm font-semibold">Nouveau clip · <span className="tabular">{formatTimecode(clipLen)}</span></p>
            <p className="text-xs text-fg-subtle tabular">
              {formatTimecode(range[0])} → {formatTimecode(range[1])}
              <span className="ml-2 hidden lg:inline">Glisse les poignées, ou [ et ] pour caler sur la lecture</span>
            </p>
          </div>
          <div className="flex-1" />
          <button className={`btn-ghost ${loopRange ? "!text-accent" : ""}`} onClick={() => {
            setLoopRange((l) => !l);
            seekTo(range[0]);
            v()?.play().catch(() => {});
          }}>
            <Repeat size={15} /> Aperçu
          </button>
          <button className="btn-ghost" onClick={exitClipMode}>
            <X size={15} /> Annuler
          </button>
          <button className="btn-primary" disabled={clipLen < 1 || saving} onClick={saveClip}>
            {saving ? <Loader2 size={15} className="animate-spin" /> : <Check size={15} />}
            {saving ? "Création…" : "Enregistrer le clip"}
          </button>
        </div>
      )}
    </div>
  );
}
