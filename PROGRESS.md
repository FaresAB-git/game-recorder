# GameRecorder — Suivi de développement

## Architecture choisie

**Stack:**
- Tauri 2 (Rust backend) + React 18 + TypeScript + Vite + Tailwind CSS
- SQLite (rusqlite bundled) pour la bibliothèque de replays
- FFmpeg sidecar embarqué pour l'enregistrement et la génération de vignettes

**Moteur d'enregistrement (choix vs libobs):**
Les bindings Rust pour libobs ne sont pas assez matures pour une intégration fiable.
Approche retenue : **FFmpeg subprocess** avec :
- Vidéo : `gdigrab` (capture fenêtre/écran via GDI Windows) ou `ddagrab` (DXGI, plus performant)
- Audio : WASAPI loopback system audio (`-f wasapi -loopback 1`)
- Encodage : h264_nvenc → h264_amf → h264_qsv → libx264 (auto-détecté au démarrage)
- Sortie : MKV (résistant aux crashs) → remux MP4 sans ré-encodage à la fin
- FFmpeg doit être placé dans `src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe`
  ou être disponible dans le PATH système (détection automatique)

**Détection des parties:**
- LoL : polling `sysinfo` sur `League of Legends.exe` toutes les 2s
- Valorant : lecture du lockfile Riot + appel API locale `https://127.0.0.1:{port}/chat/v4/presences`
  Fallback : parsing du log `ShooterGame.log`

## Prérequis installation (à faire avant de compiler)

1. **Rust + Cargo** : https://rustup.rs/
   - Cible requise : `x86_64-pc-windows-msvc`
   - VS Build Tools 2022 avec composant "Développement Desktop C++"
2. **Node.js 18+** : https://nodejs.org/ (déjà installé : v22.11.0)
3. **WebView2** : généralement déjà présent sur Windows 10 récent
4. **FFmpeg** (développement) : placer `ffmpeg.exe` dans le PATH ou dans
   `src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe`

## Commandes de développement

```bash
# Installer les dépendances npm
npm install

# Lancer en mode développement
npm run tauri dev

# Build de production
npm run tauri build
```

## État d'avancement

### Session 1 — 2026-10-12

#### Complété
- [x] Cahier des charges lu et analysé
- [x] Architecture définie
- [x] Fichier PROGRESS.md créé
- [x] `package.json` (React, Tauri, Tailwind, Zustand, React Router)
- [x] `vite.config.ts`
- [x] `tsconfig.json` + `tsconfig.node.json`
- [x] `tailwind.config.js` + `postcss.config.js`
- [x] `index.html`
- [x] `src/types.ts` — types partagés (Replay, AppConfig, RecordingStatus...)
- [x] `src/store.ts` — Zustand store global
- [x] `src/lib/tauri.ts` — wrappers des commandes Tauri
- [x] `src/App.tsx` — routing React Router
- [x] `src/index.css` — thème sombre
- [x] `src/main.tsx`
- [x] `src/pages/Replays.tsx` — grille de replays, filtres, tri
- [x] `src/pages/Settings.tsx` — tous les paramètres configurables
- [x] `src/components/Sidebar.tsx` — navigation + statut en direct
- [x] `src/components/ReplayCard.tsx` — carte avec vignette, badges, actions
- [x] `src/components/VideoPlayer.tsx` — lecteur custom (contrôles, raccourcis clavier)
- [x] `src/components/StatusBar.tsx` — barre de statut + stockage
- [x] `src-tauri/Cargo.toml`
- [x] `src-tauri/build.rs`
- [x] `src-tauri/tauri.conf.json`
- [x] `src-tauri/src/main.rs`
- [x] `src-tauri/src/lib.rs`
- [x] `src-tauri/src/config.rs` — chargement/sauvegarde JSON config
- [x] `src-tauri/src/state.rs` — AppState partagé (db, config, status)
- [x] `src-tauri/src/tray.rs` — icône tray, menu contextuel
- [x] `src-tauri/src/commands.rs` — toutes les commandes Tauri invokables
- [x] `src-tauri/src/library/mod.rs` — gestion SQLite, rotation, favoris
- [x] `src-tauri/src/library/models.rs` — structs Replay
- [x] `src-tauri/src/detection/mod.rs` — orchestrateur détection
- [x] `src-tauri/src/detection/lol.rs` — watcher processus LoL
- [x] `src-tauri/src/detection/valorant.rs` — lockfile + API locale Riot
- [x] `src-tauri/src/recording/mod.rs`
- [x] `src-tauri/src/recording/engine.rs` — orchestrateur enregistrement
- [x] `src-tauri/src/recording/ffmpeg.rs` — wrapper FFmpeg subprocess

#### Session 2 — 2026-10-12 (même session)

**Compilation réussie** — `cargo check` passe sans erreurs après corrections :
- [x] Icônes placeholder générées via FFmpeg (32x32, 128x128, 128x128@2x, icon.ico)
- [x] `tauri.conf.json` — retiré `icon.icns` (macOS uniquement)
- [x] `detection/mod.rs` — ajout `use tauri::{Emitter, Manager}` (requis pour `.emit()` et `.state()`)
- [x] `detection/valorant.rs` — fix syntaxe `base64::Engine` (`use base64::Engine as _`)
- [x] `commands.rs` — fix `pick_folder` API (utilise closure + oneshot channel)
- [x] `commands.rs` — fix `FilePath` enum conversion
- [x] `tray.rs` — retiré `Runtime` inutilisé
- [x] `recording/engine.rs` — retiré variable shadowing `mp4_filename`
- [x] `state.rs` — retiré `Arc` inutilisé
- [x] `npm install` terminé (146 packages)
- [x] `npm run tauri dev` lancé (fenêtre PowerShell ouverte)

#### À faire (prochaine session / tests)
- [ ] Vérifier que l'app s'ouvre correctement (fenêtre tray)
- [ ] Tester la détection LoL (processus) en lançant le client
- [ ] Tester la détection Valorant (lockfile + API locale)
- [ ] Tester l'enregistrement FFmpeg (lancer une vraie partie)
- [ ] Ajuster les paramètres FFmpeg si nécessaire (encodeur, audio)
- [ ] Vérifier que la bibliothèque s'affiche correctement
- [ ] Tester le lecteur vidéo intégré
- [ ] Implémenter le `StorageBar` manquant dans Sidebar (déjà dans StatusBar)
- [ ] Icônes définitives (remplacer les placeholders)
- [ ] Installeur final avec `npm run tauri build`

## Décisions techniques importantes

1. **gdigrab vs ddagrab** : `gdigrab` est utilisé pour la compatibilité maximale.
   `ddagrab` (DXGI) est plus performant mais nécessite une config FFmpeg avec filtres spéciaux.
   À switcher vers `ddagrab` si les performances ne sont pas satisfaisantes.

2. **Audio par application vs loopback** : Pour l'instant, loopback audio system entier.
   Pour un audio par application (éviter Discord/Spotify), il faudrait implémenter WASAPI
   application capture via les Windows APIs directement en Rust (complexe, future version).

3. **Vanguard et capture écran** : `gdigrab` utilise GDI (pas d'injection DLL).
   Windows Graphics Capture API (`WGC`) est encore plus sûr. Si Vanguard bloque `gdigrab`,
   implémenter WGC via le crate `windows` avec feature `Win32_Graphics_Capture`.

4. **Fenêtre au démarrage** : cachée dans le tray. La fenêtre principale s'affiche uniquement
   à la fin d'une partie ou au clic sur l'icône tray.

## Structure des fichiers

```
recordApp/
├── CAHIER_DES_CHARGES.md
├── PROGRESS.md                    <- ce fichier
├── package.json
├── vite.config.ts
├── tsconfig.json
├── tsconfig.node.json
├── tailwind.config.js
├── postcss.config.js
├── index.html
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── index.css
│   ├── types.ts
│   ├── store.ts
│   ├── lib/
│   │   └── tauri.ts
│   ├── pages/
│   │   ├── Replays.tsx
│   │   └── Settings.tsx
│   └── components/
│       ├── Sidebar.tsx
│       ├── ReplayCard.tsx
│       ├── VideoPlayer.tsx
│       └── StatusBar.tsx
└── src-tauri/
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    ├── icons/                     <- générer avec: npm run tauri icon
    └── src/
        ├── main.rs
        ├── lib.rs
        ├── config.rs
        ├── state.rs
        ├── tray.rs
        ├── commands.rs
        ├── library/
        │   ├── mod.rs
        │   └── models.rs
        ├── detection/
        │   ├── mod.rs
        │   ├── lol.rs
        │   └── valorant.rs
        └── recording/
            ├── mod.rs
            ├── engine.rs
            └── ffmpeg.rs
```

### Session 4 — 2026-10-25 — Désync audio, autostart, refonte UI, nettoyage

#### Désynchronisation audio (corrigé)
Causes : (1) WASAPI loopback n'envoie aucun paquet pendant les silences → WAV plus court
que la vidéo, le son prenait de l'avance au fil de la partie ; (2) l'audio démarrait ~2 s
après la vidéo ; (3) après un alt-tab, le trou vidéo n'existait pas côté audio.
Fix :
- `audio.rs` : comble les trous avec du silence grâce au timestamp QPC de chaque paquet,
  et pad jusqu'à l'heure d'arrêt. Le WAV couvre exactement [start, stop] en temps réel.
- `engine.rs` : l'audio démarre avant la vidéo ; chaque segment vidéo mémorise l'instant
  où il s'arrête ; début du segment = fin − durée (ffprobe).
- `ffmpeg.rs::mux_av` : découpe chaque piste audio en tranches alignées sur les segments
  (atrim/adelay/apad), concat, mix jeu+micro (amix), directement en MP4.
- Micro maintenant réellement enregistré (avant : réglage sans effet).

#### Démarrage Windows (corrigé)
L'entrée Run pointait vers `target\debug\game-recorder.exe` (build debug = console).
- Les builds debug ne s'enregistrent plus (et suppriment l'ancienne entrée).
- La version release ré-enregistre son chemin à chaque lancement.
- La fenêtre s'ouvre au démarrage (option « Démarrer réduit » pour rester dans le tray).

#### Bugs corrigés
- Config : `#[serde(default)]` (un champ ajouté réinitialisait toute la config).
- Valorant : filtrage sur son propre PUUID (un ami en game déclenchait l'enregistrement),
  nouveau format `matchPresenceData`, anti-coupure (3 lectures avant d'arrêter),
  lecture du log limitée aux 64 derniers Ko (avant : fichier entier + panic UTF-8 possible).
- Test d'encodeur en 64x64 refusé par NVENC → fallback CPU silencieux ; maintenant 256x256.
- Réglage « débit » ignoré → appliqué ; GOP 1 s pour des clips précis.
- Bundle : DLL + ffmpeg-audio inutiles (build statique) retirés ; ffprobe ajouté.
- Quitter pendant une partie la sauvegarde au lieu de tuer FFmpeg.
- Suppression : échoue proprement si le fichier est ouvert ; badge « Nouveau » persistant.
- Espace libre mesuré sur le disque du dossier de sortie ; stats rafraîchies.
- Explorer `/select` cassé avec les espaces ; mutex DB empoisonné → plus de panic.

#### Code mort supprimé
`wgc.rs` + dépendance `windows-capture`, `lol.rs`, `list_audio_input_devices`,
plugin shell, plugins JS inutilisés, events jamais émis.

#### UI
Barre de titre custom, sidebar, page Parties (dernière partie en vedette, stats, groupes
par jour), page de replay (lecteur + éditeur de clip à poignées, clips de la partie),
page Clips, paramètres avec barre « non enregistré », toasts. Icônes : lucide-react.
Emplacement prévu pour les événements Riot (kills) : `TimelineMarker` dans VideoPlayer.
