import { useEffect } from "react";
import { MemoryRouter, Routes, Route, Navigate } from "react-router-dom";
import type { UnlistenFn } from "@tauri-apps/api/event";
import TitleBar from "./components/TitleBar";
import Sidebar from "./components/Sidebar";
import Toasts from "./components/Toasts";
import Library from "./pages/Library";
import Clips from "./pages/Clips";
import ReplayView from "./pages/ReplayView";
import Settings from "./pages/Settings";
import { useStore } from "./store";
import * as api from "./lib/tauri";
import { GAME_NAMES } from "./lib/format";

export default function App() {
  useEffect(() => {
    const s = useStore.getState();

    Promise.all([api.getConfig(), api.getReplays(), api.getRecordingStatus()])
      .then(([config, replays, status]) => {
        s.setConfig(config);
        s.setReplays(replays);
        s.setRecordingStatus(status);
      })
      .catch((err) => {
        console.error("Init error:", err);
        s.toast("error", "Impossible de charger la bibliothèque");
      })
      .finally(() => s.setLoading(false));
    s.refreshStats();
    // Listing audio devices can be slow — don't block the first render on it.
    api.getAudioDevices().then(s.setAudioDevices).catch(() => {});

    const unlisteners: Promise<UnlistenFn>[] = [
      api.onRecordingStatusChanged((status) => useStore.getState().setRecordingStatus(status)),
      api.onNewReplay((replay) => {
        const st = useStore.getState();
        st.upsertReplay(replay);
        st.refreshStats();
        st.toast("success", `Partie ${GAME_NAMES[replay.game]} enregistrée`);
      }),
      api.onReplaysDeleted((ids) => {
        const st = useStore.getState();
        st.removeReplays(ids);
        st.refreshStats();
      }),
    ];

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  return (
    <MemoryRouter>
      <div className="flex flex-col h-screen bg-bg-base text-fg overflow-hidden">
        <TitleBar />
        <div className="flex flex-1 min-h-0">
          <Sidebar />
          <main className="flex-1 min-w-0 overflow-hidden">
            <Routes>
              <Route path="/" element={<Library />} />
              <Route path="/clips" element={<Clips />} />
              <Route path="/replay/:id" element={<ReplayView />} />
              <Route path="/settings" element={<Settings />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </main>
        </div>
        <Toasts />
      </div>
    </MemoryRouter>
  );
}
