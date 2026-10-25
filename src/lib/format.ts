import { format, isToday, isYesterday, parseISO, formatDistanceToNowStrict } from "date-fns";
import { fr } from "date-fns/locale";
import type { GameType } from "../types";

export function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
  return `${m}:${String(sec).padStart(2, "0")}`;
}

/** Player timestamp, with tenths when under an hour. */
export function formatTimecode(seconds: number): string {
  if (!Number.isFinite(seconds)) return "0:00";
  const base = formatDuration(seconds);
  if (seconds >= 3600) return base;
  return `${base}.${Math.floor((seconds % 1) * 10)}`;
}

/** "1h 24min", "38 min" — for totals. */
export function formatLongDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.round((seconds % 3600) / 60);
  if (h === 0) return `${m} min`;
  return `${h}h ${String(m).padStart(2, "0")}`;
}

export function formatSize(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} Go`;
  if (bytes >= 1024 ** 2) return `${Math.round(bytes / 1024 ** 2)} Mo`;
  return `${Math.max(1, Math.round(bytes / 1024))} Ko`;
}

export function formatRelative(iso: string): string {
  return formatDistanceToNowStrict(parseISO(iso), { addSuffix: true, locale: fr });
}

export function formatDateTime(iso: string): string {
  const s = format(parseISO(iso), "EEEE d MMMM · HH:mm", { locale: fr });
  return s.charAt(0).toUpperCase() + s.slice(1);
}

export function formatTime(iso: string): string {
  return format(parseISO(iso), "HH:mm");
}

/** Section title for grouping by day. */
export function dayLabel(iso: string): string {
  const d = parseISO(iso);
  if (isToday(d)) return "Aujourd'hui";
  if (isYesterday(d)) return "Hier";
  const label = format(d, "EEEE d MMMM", { locale: fr });
  return label.charAt(0).toUpperCase() + label.slice(1);
}

export function dayKey(iso: string): string {
  return format(parseISO(iso), "yyyy-MM-dd");
}

export const GAME_NAMES: Record<GameType, string> = {
  lol: "League of Legends",
  valorant: "Valorant",
};

export const GAME_SHORT: Record<GameType, string> = {
  lol: "LoL",
  valorant: "Valorant",
};
