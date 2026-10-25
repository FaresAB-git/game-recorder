import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Replay, AppConfig, RecordingStatus, LibraryStats } from "../types";

// ── Config ────────────────────────────────────────────────────────────────────

export const getConfig = () => invoke<AppConfig>("get_config");
/** Returns the config as saved (values clamped by the backend). */
export const saveConfig = (config: AppConfig) => invoke<AppConfig>("save_config", { config });

// ── Library ───────────────────────────────────────────────────────────────────

export const getReplays = () => invoke<Replay[]>("get_replays");
export const deleteReplay = (id: string) => invoke<void>("delete_replay", { id });
export const toggleFavorite = (id: string) => invoke<boolean>("toggle_favorite", { id });
export const markSeen = (id?: string) => invoke<void>("mark_seen", { id: id ?? null });
export const openReplayInExplorer = (id: string) => invoke<void>("open_replay_in_explorer", { id });
export const openOutputFolder = () => invoke<void>("open_output_folder");
export const createClip = (id: string, startSec: number, endSec: number) =>
  invoke<Replay>("create_clip", { id, startSec, endSec });
export const getLibraryStats = () => invoke<LibraryStats>("get_library_stats");

// ── Recording / system ────────────────────────────────────────────────────────

export const getRecordingStatus = () => invoke<RecordingStatus>("get_recording_status");
export const getAudioDevices = () => invoke<string[]>("get_audio_devices");
export const pickOutputDirectory = () => invoke<string | null>("pick_output_directory");

// ── Events ────────────────────────────────────────────────────────────────────

export const onNewReplay = (cb: (replay: Replay) => void) =>
  listen<Replay>("new_replay", (e) => cb(e.payload));

export const onReplaysDeleted = (cb: (ids: string[]) => void) =>
  listen<string[]>("replays_deleted", (e) => cb(e.payload));

export const onRecordingStatusChanged = (cb: (status: RecordingStatus) => void) =>
  listen<RecordingStatus>("recording_status_changed", (e) => cb(e.payload));
