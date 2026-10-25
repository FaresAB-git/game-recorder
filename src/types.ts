export type GameType = "lol" | "valorant";

export interface Replay {
  id: string;
  game: GameType;
  date: string; // ISO 8601
  duration: number; // seconds
  size: number; // bytes
  path: string;
  thumbnailPath?: string | null;
  favorite: boolean;
  isNew: boolean;
  isClip: boolean;
  /** For clips: id of the game recording they were cut from. */
  sourceId?: string | null;
}

export interface AppConfig {
  outputDir: string;
  maxStorageGb: number;
  maxReplays: number;
  minGameDurationMinutes: number;
  resolution: "native" | "1080p" | "720p";
  fps: 30 | 60;
  bitrate: number; // kbps
  encoder: "auto" | "nvenc" | "amf" | "qsv" | "x264";
  captureGameAudio: boolean;
  captureMic: boolean;
  micDevice?: string | null;
  enableLol: boolean;
  enableValorant: boolean;
  launchAtStartup: boolean;
  startMinimized: boolean;
  autoOpenAfterGame: boolean;
  showNotifications: boolean;
}

export const defaultConfig: AppConfig = {
  outputDir: "",
  maxStorageGb: 50,
  maxReplays: 30,
  minGameDurationMinutes: 3,
  resolution: "native",
  fps: 60,
  bitrate: 20000,
  encoder: "auto",
  captureGameAudio: true,
  captureMic: false,
  enableLol: true,
  enableValorant: true,
  launchAtStartup: false,
  startMinimized: false,
  autoOpenAfterGame: true,
  showNotifications: true,
};

export interface RecordingStatus {
  isRecording: boolean;
  isProcessing: boolean;
  game?: GameType | null;
  startedAt?: string | null;
}

export interface LibraryStats {
  storage: { usedBytes: number; maxBytes: number; freeBytes: number };
  totalReplays: number;
}

export type FilterType = "all" | GameType | "favorites";
export type SortType = "date-desc" | "date-asc" | "duration" | "size";
