import { useEffect, useState } from "react";
import { FolderOpen, HardDrive, Video, Mic, Gamepad2, Power, Loader2 } from "lucide-react";
import { useStore } from "../store";
import * as api from "../lib/tauri";
import { GameMark } from "../components/GameBadge";
import type { AppConfig } from "../types";

function Section({ icon, title, children }: { icon: React.ReactNode; title: string; children: React.ReactNode }) {
  return (
    <section className="space-y-2.5">
      <h2 className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-fg-subtle px-1">
        {icon} {title}
      </h2>
      <div className="card divide-y divide-line">{children}</div>
    </section>
  );
}

function Row({ label, description, children }: { label: React.ReactNode; description?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-6 px-5 py-4">
      <div className="min-w-0">
        <div className="text-sm font-medium">{label}</div>
        {description && <p className="text-xs text-fg-subtle mt-0.5">{description}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function Toggle({ checked, onChange, disabled }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors disabled:opacity-40 ${
        checked ? "bg-accent" : "bg-bg-hover"
      }`}
    >
      <span className={`inline-block h-[18px] w-[18px] rounded-full bg-white shadow transition-transform ${
        checked ? "translate-x-[22px]" : "translate-x-[3px]"
      }`} />
    </button>
  );
}

function Segmented<T extends string | number>({ value, options, onChange }: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
}) {
  return (
    <div className="flex p-0.5 rounded-lg bg-bg-surface border border-line">
      {options.map((o) => (
        <button key={String(o.value)} onClick={() => onChange(o.value)}
          className={`h-8 px-3 rounded-md text-sm font-medium transition-colors ${
            value === o.value ? "bg-bg-hover text-fg" : "text-fg-muted hover:text-fg"
          }`}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

function NumberField({ value, onChange, min, max, step = 1, suffix }: {
  value: number; onChange: (v: number) => void; min: number; max: number; step?: number; suffix?: string;
}) {
  return (
    <div className="flex items-center gap-2">
      <input type="number" className="input w-24 text-right tabular" value={value} min={min} max={max} step={step}
        onChange={(e) => onChange(e.target.value === "" ? min : Number(e.target.value))}
        onBlur={(e) => onChange(Math.min(max, Math.max(min, Number(e.target.value) || min)))} />
      {suffix && <span className="text-sm text-fg-subtle w-8">{suffix}</span>}
    </div>
  );
}

const BITRATES = [
  { value: 8000, label: "8 Mb/s" },
  { value: 15000, label: "15 Mb/s" },
  { value: 20000, label: "20 Mb/s" },
  { value: 35000, label: "35 Mb/s" },
  { value: 50000, label: "50 Mb/s" },
];

export default function Settings() {
  const { config, setConfig, audioDevices, toast, refreshStats } = useStore();
  const [local, setLocal] = useState<AppConfig>(config);
  const [saving, setSaving] = useState(false);

  useEffect(() => setLocal(config), [config]);

  const set = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) => setLocal((p) => ({ ...p, [key]: value }));
  const dirty = JSON.stringify(local) !== JSON.stringify(config);

  async function save() {
    setSaving(true);
    try {
      const saved = await api.saveConfig(local);
      setConfig(saved);
      refreshStats();
      toast("success", "Paramètres enregistrés");
    } catch (e) {
      toast("error", `Erreur : ${e}`);
    } finally {
      setSaving(false);
    }
  }

  async function pickDir() {
    const dir = await api.pickOutputDirectory();
    if (dir) set("outputDir", dir);
  }

  const bitrateKnown = BITRATES.some((b) => b.value === local.bitrate);

  return (
    <div className="relative h-full flex flex-col">
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-3xl mx-auto px-8 py-7 space-y-7 pb-28">
          <div>
            <h1 className="text-2xl font-bold">Paramètres</h1>
            <p className="text-sm text-fg-muted mt-1">Les changements s'appliquent à la prochaine partie.</p>
          </div>

          <Section icon={<Gamepad2 size={14} />} title="Jeux">
            <Row label={<span className="flex items-center gap-2"><GameMark game="lol" size={18} /> League of Legends</span>}
              description="Démarre quand la partie se charge, s'arrête à l'écran de fin.">
              <Toggle checked={local.enableLol} onChange={(v) => set("enableLol", v)} />
            </Row>
            <Row label={<span className="flex items-center gap-2"><GameMark game="valorant" size={18} /> Valorant</span>}
              description="Seulement en match (pas dans les menus ni le range).">
              <Toggle checked={local.enableValorant} onChange={(v) => set("enableValorant", v)} />
            </Row>
            <Row label="Durée minimale d'une partie" description="Les parties plus courtes (remake, dodge) ne sont pas gardées.">
              <NumberField value={local.minGameDurationMinutes} min={0} max={60} suffix="min"
                onChange={(v) => set("minGameDurationMinutes", v)} />
            </Row>
          </Section>

          <Section icon={<Video size={14} />} title="Vidéo">
            <Row label="Résolution" description="Native = celle de ton écran.">
              <Segmented value={local.resolution} onChange={(v) => set("resolution", v)}
                options={[{ value: "native", label: "Native" }, { value: "1080p", label: "1080p" }, { value: "720p", label: "720p" }]} />
            </Row>
            <Row label="Images par seconde">
              <Segmented value={local.fps} onChange={(v) => set("fps", v)}
                options={[{ value: 30, label: "30 FPS" }, { value: 60, label: "60 FPS" }]} />
            </Row>
            <Row label="Qualité (débit)" description="Plus haut = plus beau, mais fichiers plus lourds (~9 Go/h à 20 Mb/s).">
              <select className="input" value={local.bitrate} onChange={(e) => set("bitrate", Number(e.target.value))}>
                {!bitrateKnown && <option value={local.bitrate}>{Math.round(local.bitrate / 1000)} Mb/s</option>}
                {BITRATES.map((b) => <option key={b.value} value={b.value}>{b.label}</option>)}
              </select>
            </Row>
            <Row label="Encodeur" description="Automatique choisit ta carte graphique (NVIDIA → AMD → Intel), sinon le processeur.">
              <select className="input" value={local.encoder} onChange={(e) => set("encoder", e.target.value as AppConfig["encoder"])}>
                <option value="auto">Automatique</option>
                <option value="nvenc">NVIDIA NVENC</option>
                <option value="amf">AMD AMF</option>
                <option value="qsv">Intel Quick Sync</option>
                <option value="x264">Processeur (x264)</option>
              </select>
            </Row>
          </Section>

          <Section icon={<Mic size={14} />} title="Audio">
            <Row label="Son du PC" description="Tout ce que tu entends (jeu, Discord, musique…).">
              <Toggle checked={local.captureGameAudio} onChange={(v) => set("captureGameAudio", v)} />
            </Row>
            <Row label="Microphone" description="Ta voix, mixée avec le son du PC.">
              <Toggle checked={local.captureMic} onChange={(v) => set("captureMic", v)} />
            </Row>
            {local.captureMic && (
              <Row label="Micro utilisé">
                <select className="input max-w-[260px]" value={local.micDevice ?? ""}
                  onChange={(e) => set("micDevice", e.target.value || null)}>
                  <option value="">Micro par défaut de Windows</option>
                  {audioDevices.map((d) => <option key={d} value={d}>{d}</option>)}
                </select>
              </Row>
            )}
          </Section>

          <Section icon={<HardDrive size={14} />} title="Stockage">
            <Row label="Dossier des vidéos" description={<span className="font-mono break-all">{local.outputDir || "Non configuré"}</span>}>
              <div className="flex gap-2">
                <button className="icon-btn border border-line" title="Ouvrir" onClick={() => api.openOutputFolder()}>
                  <FolderOpen size={16} />
                </button>
                <button className="btn-outline" onClick={pickDir}>Changer…</button>
              </div>
            </Row>
            <Row label="Espace maximum" description="Les parties les plus anciennes sont supprimées au-delà (jamais les favoris ni les clips).">
              <NumberField value={local.maxStorageGb} min={1} max={10000} suffix="Go" onChange={(v) => set("maxStorageGb", v)} />
            </Row>
            <Row label="Nombre maximum de parties">
              <NumberField value={local.maxReplays} min={1} max={10000} onChange={(v) => set("maxReplays", v)} />
            </Row>
          </Section>

          <Section icon={<Power size={14} />} title="Démarrage & comportement">
            <Row label="Lancer avec Windows" description="Uniquement avec la version installée (pas en mode dev).">
              <Toggle checked={local.launchAtStartup} onChange={(v) => set("launchAtStartup", v)} />
            </Row>
            <Row label="Démarrer réduit" description="Au démarrage de Windows, rester dans la barre des tâches sans ouvrir la fenêtre.">
              <Toggle checked={local.startMinimized} disabled={!local.launchAtStartup} onChange={(v) => set("startMinimized", v)} />
            </Row>
            <Row label="Ouvrir l'app à la fin d'une partie">
              <Toggle checked={local.autoOpenAfterGame} onChange={(v) => set("autoOpenAfterGame", v)} />
            </Row>
            <Row label="Notifications Windows" description="Quand l'enregistrement démarre ou échoue.">
              <Toggle checked={local.showNotifications} onChange={(v) => set("showNotifications", v)} />
            </Row>
          </Section>
        </div>
      </div>

      {dirty && (
        <div className="absolute bottom-6 left-1/2 -translate-x-1/2 z-40 animate-slide-up">
          <div className="flex items-center gap-4 pl-5 pr-2 py-2 rounded-xl bg-bg-elevated border border-line-strong shadow-card">
            <span className="text-sm whitespace-nowrap">Modifications non enregistrées</span>
            <button className="btn-ghost" onClick={() => setLocal(config)} disabled={saving}>Annuler</button>
            <button className="btn-primary" onClick={save} disabled={saving}>
              {saving && <Loader2 size={15} className="animate-spin" />}
              Enregistrer
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
