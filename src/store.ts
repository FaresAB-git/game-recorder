import { create } from "zustand";
import type { Replay, AppConfig, RecordingStatus, LibraryStats, FilterType, SortType } from "./types";
import { defaultConfig } from "./types";
import * as api from "./lib/tauri";

export interface Toast {
  id: number;
  kind: "success" | "error" | "info";
  message: string;
}

interface AppStore {
  replays: Replay[];
  config: AppConfig;
  recordingStatus: RecordingStatus;
  libraryStats: LibraryStats | null;
  audioDevices: string[];
  isLoading: boolean;

  filter: FilterType;
  sort: SortType;
  toasts: Toast[];

  setReplays: (replays: Replay[]) => void;
  upsertReplay: (replay: Replay) => void;
  updateReplay: (id: string, patch: Partial<Replay>) => void;
  removeReplays: (ids: string[]) => void;
  setConfig: (config: AppConfig) => void;
  setRecordingStatus: (status: RecordingStatus) => void;
  setAudioDevices: (devices: string[]) => void;
  setLoading: (loading: boolean) => void;
  setFilter: (filter: FilterType) => void;
  setSort: (sort: SortType) => void;
  refreshStats: () => Promise<void>;
  markSeen: (id: string) => void;
  toast: (kind: Toast["kind"], message: string) => void;
  dismissToast: (id: number) => void;
}

let toastId = 0;

export const useStore = create<AppStore>((set, get) => ({
  replays: [],
  config: defaultConfig,
  recordingStatus: { isRecording: false, isProcessing: false },
  libraryStats: null,
  audioDevices: [],
  isLoading: true,
  filter: "all",
  sort: "date-desc",
  toasts: [],

  setReplays: (replays) => set({ replays }),
  upsertReplay: (replay) =>
    set((s) => ({ replays: [replay, ...s.replays.filter((r) => r.id !== replay.id)] })),
  updateReplay: (id, patch) =>
    set((s) => ({ replays: s.replays.map((r) => (r.id === id ? { ...r, ...patch } : r)) })),
  removeReplays: (ids) => set((s) => ({ replays: s.replays.filter((r) => !ids.includes(r.id)) })),
  setConfig: (config) => set({ config }),
  setRecordingStatus: (recordingStatus) => set({ recordingStatus }),
  setAudioDevices: (audioDevices) => set({ audioDevices }),
  setLoading: (isLoading) => set({ isLoading }),
  setFilter: (filter) => set({ filter }),
  setSort: (sort) => set({ sort }),

  refreshStats: async () => {
    try {
      set({ libraryStats: await api.getLibraryStats() });
    } catch (e) {
      console.error("stats", e);
    }
  },

  markSeen: (id) => {
    const r = get().replays.find((x) => x.id === id);
    if (!r?.isNew) return;
    get().updateReplay(id, { isNew: false });
    api.markSeen(id).catch(() => {});
  },

  toast: (kind, message) => {
    const id = ++toastId;
    set((s) => ({ toasts: [...s.toasts, { id, kind, message }] }));
    setTimeout(() => get().dismissToast(id), kind === "error" ? 6000 : 3500);
  },
  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));
