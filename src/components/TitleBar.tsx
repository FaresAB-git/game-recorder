import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, Copy, X } from "lucide-react";
import RecordingPill from "./RecordingPill";

/** Custom title bar (the native one is disabled in tauri.conf.json). */
export default function TitleBar() {
  const win = getCurrentWindow();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    win.isMaximized().then(setMaximized).catch(() => {});
    const un = win.onResized(() => win.isMaximized().then(setMaximized).catch(() => {}));
    return () => {
      un.then((f) => f());
    };
  }, []);

  return (
    <header
      data-tauri-drag-region
      className="h-10 shrink-0 flex items-center justify-between bg-bg-surface border-b border-line pl-4"
      onDoubleClick={(e) => {
        if (e.target === e.currentTarget) win.toggleMaximize();
      }}
    >
      <div data-tauri-drag-region className="flex items-center gap-2 pointer-events-none">
        <div className="w-5 h-5 rounded-md bg-accent flex items-center justify-center">
          <span className="w-2 h-2 rounded-full bg-white" />
        </div>
        <span className="text-[13px] font-semibold tracking-wide">GameRecorder</span>
      </div>

      <div data-tauri-drag-region className="flex-1 flex justify-center">
        <RecordingPill />
      </div>

      <div className="flex h-full">
        <button className="w-12 h-full flex items-center justify-center text-fg-muted hover:bg-bg-hover hover:text-fg transition-colors"
          onClick={() => win.minimize()} title="Réduire">
          <Minus size={16} />
        </button>
        <button className="w-12 h-full flex items-center justify-center text-fg-muted hover:bg-bg-hover hover:text-fg transition-colors"
          onClick={() => win.toggleMaximize()} title={maximized ? "Restaurer" : "Agrandir"}>
          {maximized ? <Copy size={13} className="-scale-x-100" /> : <Square size={13} />}
        </button>
        <button className="w-12 h-full flex items-center justify-center text-fg-muted hover:bg-[#e81123] hover:text-white transition-colors"
          onClick={() => win.hide()} title="Fermer (reste dans la barre des tâches)">
          <X size={17} />
        </button>
      </div>
    </header>
  );
}
