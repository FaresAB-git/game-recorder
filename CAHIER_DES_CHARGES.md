# Cahier des charges — GameRecorder (LoL + Valorant)

## 1. Objectif

Application de bureau Windows, légère, qui enregistre automatiquement mes parties de **League of Legends** et **Valorant** (vidéo + son du jeu), stocke les replays dans un dossier, supprime automatiquement les plus anciens au-delà d'une limite, et propose une belle interface pour revoir les derniers replays (inspirée d'Outplayed / Medal).

Pas de détection de kills, pas de highlights, pas d'API Riot en ligne. On a seulement besoin de savoir **quand une partie commence et quand elle se termine**.

## 2. Périmètre

**Dans le périmètre**
- Détection automatique début / fin de partie (LoL et Valorant)
- Enregistrement vidéo + audio système (+ micro en option)
- Bibliothèque de replays avec vignettes
- Rotation automatique (suppression des plus anciens)
- Ouverture automatique de l'app à la fin de chaque partie
- Lecteur vidéo intégré
- Paramètres simples
- Icône dans la barre des tâches (system tray), lancement au démarrage de Windows

**Hors périmètre (v1)**
- Détection de kills / événements, clips automatiques
- Montage, découpe, partage, upload
- Comptes utilisateurs, cloud
- macOS / Linux

## 3. Stack technique recommandée

- **Tauri 2** (backend Rust) + **React + TypeScript + Vite + Tailwind** pour le front
- **libobs** (le moteur d'OBS Studio) **embarqué** dans l'app pour la capture vidéo/audio et l'encodage
  - L'utilisateur n'installe **pas** OBS : les DLL, plugins et fichiers data nécessaires sont livrés avec l'installeur
  - Bindings Rust : évaluer `libobs-rs` / `libobs-wrapper` ; s'ils ne sont pas assez matures, écrire un petit wrapper FFI sur l'API C de libobs
  - Licence GPL-2.0 → OK pour un usage personnel
- **FFmpeg** (CLI) embarqué en sidecar uniquement pour le remux MKV → MP4 et la génération des vignettes
- Stockage des métadonnées : un fichier `library.json` (ou SQLite) dans le dossier de l'app

## 4. Détection des parties (sans API en ligne)

Un service tourne en arrière-plan et vérifie l'état toutes les ~2 secondes.

### League of Legends
- Début de partie = apparition du processus **`League of Legends.exe`** (le processus du jeu, pas le client `LeagueClient.exe`). Il démarre à l'écran de chargement.
- Fin de partie = disparition de ce processus.
- Le client LoL et la sélection des champions ne doivent **pas** déclencher d'enregistrement.

### Valorant
- Le processus `VALORANT-Win64-Shipping.exe` tourne en permanence (menus inclus), donc il ne suffit pas.
- Méthode principale : l'**API locale du client Riot** (tourne sur la machine, aucune clé ni requête internet) :
  - lire le `lockfile` dans `%LOCALAPPDATA%\Riot Games\Riot Client\Config\lockfile` (port + mot de passe)
  - interroger `https://127.0.0.1:{port}/chat/v4/presences` (auth basique `riot:{password}`, certificat auto-signé à accepter)
  - décoder la présence du joueur et lire l'état de session (`sessionLoopState` : `MENUS` / `PREGAME` / `INGAME`)
  - début = passage à `INGAME`, fin = sortie de `INGAME`
  - ⚠️ le format exact du JSON de présence a évolué selon les versions : à vérifier et coder de façon défensive
- Méthode de secours : parser le log `%LOCALAPPDATA%\VALORANT\Saved\Logs\ShooterGame.log` pour repérer le chargement et la fin de match.
- Dernier recours (mode dégradé) : enregistrer toute la durée de vie du processus.

### Règles communes
- Si une partie dure moins de **X minutes** (réglable, défaut 3 min : remake, crash, dodge), l'enregistrement est supprimé.
- Si le jeu crashe, l'enregistrement est arrêté proprement et conservé.

## 5. Enregistrement

Tout l'enregistrement passe par **libobs** embarqué, initialisé au démarrage de l'app (scène + sources préparées à l'avance pour démarrer l'enregistrement instantanément).

- **Vidéo** : source **« Capture de jeu »** d'OBS (`game_capture`) ciblant la fenêtre du jeu (`League of Legends.exe` / `VALORANT-Win64-Shipping.exe`).
  - C'est le hook officiel d'OBS, signé et compatible avec Vanguard. **Interdit** d'utiliser tout autre hook ou injection maison.
  - Fallback si la capture de jeu échoue : capture de fenêtre (`window_capture`, mode Windows Graphics Capture), puis capture d'écran (`monitor_capture`).
- **Audio** :
  - par défaut, **son du jeu uniquement** via la capture audio d'application d'OBS (`wasapi_process_output_capture`, Windows 10 2004+), pour ne pas enregistrer Discord / Spotify
  - option : tout le son système (`wasapi_output_capture`)
  - option : micro (`wasapi_input_capture`) sur une piste audio séparée, désactivable
- **Encodage** : encodeur matériel choisi automatiquement : NVENC (NVIDIA) → AMF (AMD) → QuickSync (Intel) → fallback x264. Mode CQP/CQ ou bitrate réglable.
- Écrire en **MKV** (résiste aux crashs), puis remuxer en **MP4 H.264/AAC** à la fin avec FFmpeg (sans ré-encodage) pour la lecture dans l'app.
- **Performances** : aucune baisse de FPS notable. Encodage GPU obligatoire si dispo, pas de prévisualisation ni de rendu inutile côté libobs, rien qui tourne en continu hors enregistrement.
- Nommage : `{jeu}_{AAAA-MM-JJ}_{HH-mm-ss}.mp4` (ex. `LoL_2026-10-12_20-14-03.mp4`).
- Après chaque enregistrement : génération d'une vignette (frame à ~30 % de la vidéo) via FFmpeg.

## 6. Stockage et rotation

- Dossier de sortie configurable (défaut : `Vidéos\GameRecorder`).
- Limite configurable en **Go** (défaut 50 Go) et/ou en **nombre de replays** (défaut 30).
- Après chaque nouvel enregistrement : tant que la limite est dépassée, supprimer le replay **le plus ancien** (vidéo + vignette + entrée dans l'index).
- Les replays marqués **favoris** ne sont jamais supprimés automatiquement.
- Au démarrage, resynchroniser l'index avec le contenu réel du dossier (fichiers supprimés à la main, etc.).
- Alerte si l'espace disque libre passe sous ~5 Go.

## 7. Comportement de l'application

- Démarre minimisée dans le tray (option : lancement avec Windows).
- Fermer la fenêtre = réduire dans le tray (quitter via le menu du tray).
- **À la fin de chaque partie** : la fenêtre s'ouvre et met en avant le nouveau replay (badge « Nouveau »).
  - Ne jamais ouvrir la fenêtre pendant qu'une partie est en cours.
  - Option pour désactiver cette ouverture automatique.
- Notification Windows discrète au début de l'enregistrement (désactivable).

## 8. Interface (front)

Style : thème sombre, moderne, type Medal / Outplayed. Couleur d'accent par jeu (ex. bleu/or pour LoL, rouge pour Valorant) via des badges texte « LoL » / « VALO ».

### Barre latérale
- Replays
- Paramètres
- En bas : statut en direct (« En attente d'une partie » / point rouge « Enregistrement — LoL — 12:34 ») et jauge de stockage (ex. 32 / 50 Go).

### Page Replays
- Grille de cartes : vignette, badge du jeu, date relative (« il y a 2 h »), durée, taille.
- Survol de la carte : léger zoom / aperçu.
- Filtres : Tous / LoL / Valorant / Favoris. Tri par date (défaut récent → ancien).
- Actions rapides sur la carte : favori ⭐, ouvrir dans l'explorateur, supprimer (avec confirmation).

### Lecteur
- S'ouvre au clic sur une carte (vue dédiée ou modal plein écran).
- Contrôles personnalisés : lecture/pause, barre de progression, volume, plein écran, vitesse (0,25× à 2×).
- Raccourcis : `Espace` pause, `←/→` ±5 s, `J/L` ±10 s, `,`/`.` image par image, `F` plein écran, `Échap` fermer.
- Infos : jeu, date, durée, taille, chemin du fichier.

### Paramètres
- Dossier de sortie
- Limite de stockage (Go) et nombre max de replays
- Qualité : résolution (native / 1080p / 720p), FPS (30 / 60), débit (bitrate), encodeur (auto / choix manuel)
- Audio : son du jeu oui/non, micro oui/non + choix du périphérique
- Durée minimale d'une partie conservée
- Jeux activés (LoL, Valorant)
- Lancer au démarrage de Windows, ouvrir à la fin des parties, notifications

## 9. Exigences non fonctionnelles

- Consommation en attente quasi nulle (CPU < 1 %, RAM raisonnable).
- Aucune perte de vidéo en cas de crash du jeu ou de l'app (grâce au MKV).
- Lecture fluide des replays dans l'app (lecture via le protocole d'assets Tauri, pas de chargement du fichier entier en mémoire).
- Logs de l'app dans un fichier pour le débogage.
- Installeur Windows (`.msi` ou `.exe` via Tauri bundler) qui inclut libobs (DLL, plugins, data) et FFmpeg. La taille de l'installeur n'est pas une contrainte.
- Aucune dépendance externe à installer par l'utilisateur (pas d'OBS, pas de FFmpeg système).

## 10. Étapes de développement suggérées

1. Squelette Tauri + React, tray, fenêtre, page Paramètres avec sauvegarde de la config.
2. Service de détection LoL (processus) + Valorant (API locale / logs), avec logs clairs des changements d'état.
3. Moteur d'enregistrement : intégration libobs (init, capture de jeu, audio de l'application, encodeur matériel), MKV → MP4. Commencer par un prototype minimal qui enregistre 30 s d'une fenêtre pour valider l'intégration avant le reste.
4. Index des replays, vignettes, rotation, favoris.
5. Interface Replays + lecteur.
6. Ouverture auto en fin de partie, notifications, lancement au démarrage, installeur.

## 11. Critères d'acceptation

- Je lance une partie LoL → l'enregistrement démarre au chargement, s'arrête à la fin, la fenêtre s'ouvre sur le nouveau replay.
- Idem pour une partie Valorant ; rester dans les menus Valorant n'enregistre rien.
- Le replay se lit dans l'app avec le son du jeu (et sans le son de Discord / Spotify par défaut).
- L'app fonctionne sur un PC où OBS n'est pas installé.
- Quand la limite est dépassée, le plus ancien replay non favori est supprimé automatiquement.
- Aucune baisse de FPS notable en jeu avec l'encodage GPU.
