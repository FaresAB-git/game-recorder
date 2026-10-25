# FFmpeg binaries

Ces fichiers ne sont pas inclus dans le dépôt (trop lourds).

Télécharge un build **statique** BtbN (`ffmpeg-master-latest-win64-gpl`, pas `-shared`) :
https://github.com/BtbN/FFmpeg-builds/releases

Copie dans ce dossier :
- `bin/ffmpeg.exe`  → `ffmpeg-x86_64-pc-windows-msvc.exe`
- `bin/ffprobe.exe` → `ffprobe-x86_64-pc-windows-msvc.exe`

C'est tout : aucune DLL n'est nécessaire avec un build statique.

## Pourquoi ce nommage ?

Tauri `externalBin` exige le triplet de target Rust dans le nom du fichier.
À l'installation, ils sont copiés à côté de l'exe sous le nom `ffmpeg.exe` / `ffprobe.exe`.

`build.rs` patche `ffmpeg` en sous-système GUI pour qu'aucune console ne
s'ouvre jamais pendant une partie (en plus du flag `CREATE_NO_WINDOW`).
