# GameRecorder

Enregistre automatiquement tes parties de **League of Legends** et **Valorant**, et te permet de les revoir et d'en couper les meilleurs moments, à la manière d'Outplayed ou Medal.

- **Automatique** : l'enregistrement démarre quand la partie se lance et s'arrête à la fin, sans rien toucher.
- **Léger** : capture et encodage sur la carte graphique (NVIDIA NVENC, AMD AMF ou Intel Quick Sync), processeur en secours.
- **Son du PC + micro** : son du jeu et voix, synchronisés avec la vidéo.
- **Clips** : appuie sur `C` pendant la lecture, ajuste le début et la fin, enregistre.
- **Rangement automatique** : les plus vieilles parties sont supprimées au-delà de la limite choisie (jamais les favoris ni les clips).
- **Compatible Vanguard** : capture de l'écran via l'API Windows Desktop Duplication, sans injection dans le jeu.

## Installation

1. Va dans [**Releases**](../../releases/latest) et télécharge `GameRecorder_x.y.z_x64-setup.exe`.
2. Lance-le. Windows peut afficher **« Windows a protégé votre PC »** parce que l'app n'est pas signée : clique sur **Informations complémentaires**, puis **Exécuter quand même**.
3. C'est tout. GameRecorder se lance dans la barre des tâches et attend ta prochaine partie.

Configuration requise : Windows 10 ou 11 (64 bits). Aucune autre installation nécessaire (FFmpeg est inclus).

Les vidéos sont enregistrées dans `Vidéos\GameRecorder` (modifiable dans les paramètres).

## Utilisation

| Action | Raccourci dans le lecteur |
|---|---|
| Lecture / pause | `Espace` |
| Reculer / avancer de 5 s | `←` `→` |
| Reculer / avancer de 10 s | `J` `L` |
| Image par image | `,` `.` |
| Créer un clip | `C`, puis `[` `]` pour caler le début / la fin, `Entrée` pour enregistrer |
| Plein écran | `F` |
| Muet | `M` |

Fermer la fenêtre garde l'app active dans la barre des tâches. Pour quitter : clic droit sur l'icône → **Quitter GameRecorder** (une partie en cours est sauvegardée avant de fermer).

En cas de problème, le journal se trouve dans `%LOCALAPPDATA%\com.gamerecorder.app\logs\gamerecorder.log`.

## Développement

Stack : [Tauri 2](https://tauri.app) (Rust) + React + TypeScript + Tailwind, SQLite pour la bibliothèque, FFmpeg pour la capture et l'encodage.

Prérequis : [Rust](https://rustup.rs), [Node.js 18+](https://nodejs.org), Visual Studio Build Tools (C++).

```bash
npm install
```

Place les binaires FFmpeg dans `src-tauri/binaries/` (voir [src-tauri/binaries/README.md](src-tauri/binaries/README.md)), puis :

```bash
npm run tauri dev
```

Pour produire l'installeur (`src-tauri/target/release/bundle/nsis/`) :

```bash
npm run tauri build
```

## Feuille de route

- Événements de partie (kills, morts, objectifs) sur la timeline via l'API Riot
- Clips automatiques des meilleurs moments

## Licences

Le code de GameRecorder est fourni tel quel, pour un usage personnel.

L'installeur embarque **[FFmpeg](https://ffmpeg.org)**, distribué sous licence **GNU GPL v3**. Les binaires utilisés sont les builds statiques de [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds), dont le code source est disponible sur ce dépôt et sur [ffmpeg.org](https://ffmpeg.org/download.html).

GameRecorder n'est ni affilié à Riot Games ni approuvé par Riot Games. League of Legends et Valorant sont des marques de Riot Games, Inc.
