import { CheckCircle2, AlertCircle, Info, X } from "lucide-react";
import { useStore } from "../store";

const ICONS = {
  success: <CheckCircle2 size={18} className="text-success shrink-0" />,
  error: <AlertCircle size={18} className="text-danger shrink-0" />,
  info: <Info size={18} className="text-accent shrink-0" />,
};

export default function Toasts() {
  const toasts = useStore((s) => s.toasts);
  const dismiss = useStore((s) => s.dismissToast);

  return (
    <div className="fixed bottom-5 right-5 z-[100] flex flex-col gap-2 w-80">
      {toasts.map((t) => (
        <div key={t.id} className="flex items-start gap-3 p-3 rounded-xl bg-bg-elevated border border-line-strong shadow-card animate-slide-up">
          {ICONS[t.kind]}
          <p className="flex-1 text-sm text-fg leading-snug break-words">{t.message}</p>
          <button className="text-fg-subtle hover:text-fg" onClick={() => dismiss(t.id)}>
            <X size={15} />
          </button>
        </div>
      ))}
    </div>
  );
}
