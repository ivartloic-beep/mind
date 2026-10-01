# Ma Tête

App Windows locale-first : capture rapide, panneau latéral, post-it natifs.  
Stack : **Tauri 2** + **React** + **TypeScript** + **Vite**.

Nom provisoire : **Ma Tête**.

## État actuel

- **Étape 1** : squelette multi-fenêtres (`main` / `panel` / `capture` / `postit`)
- **Étape 2** : stockage SQLite (`rusqlite`), modèles, CRUD Tauri, event `data-changed`
- **Étape 3** : panneau latéral droit (~400 px), poignée slide, always-on-top mémorisé
- **Étape 4** : capture rapide (fenêtre légère, Task/Note/Idée, depuis le panneau)

Pas encore : CRUD tâches avancé UI, tray, raccourcis globaux, post-it (étapes 5+).

## Prérequis

- Node.js 20+ et npm
- [Rust](https://rustup.rs/) (stable, ≥ 1.90 recommandé pour Tauri 2 récent)
- Sur **Windows** : [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)
- Sur **Linux** (dev / CI) : `libwebkit2gtk-4.1-dev`, `librsvg2-dev`, `patchelf` — [prerequisites Tauri](https://tauri.app/start/prerequisites/)

Cible produit : **Windows** (installeur **NSIS**).

## Lancer

```bash
npm install
npm run tauri dev
```

Équivalents :

```bash
npm run dev
npm run build
cd src-tauri && cargo check
cd src-tauri && cargo test   # persistance SQLite (reopen)
```

DB locale : `{app_data_dir}/ma-tete.db`.

## Entries frontend

| Entry | Fichier | Rôle |
|---|---|---|
| `main` | `index.html` → `src/main.tsx` | Host |
| `panel` | `panel.html` → `src/panel.tsx` | Panneau |
| `capture` | `capture.html` → `src/capture.tsx` | Capture |
| `postit` | `postit.html` → `src/postit.tsx` | Post-it |

## Architecture

Référence store : `docs/ma-tete-v1-architecture.md`.
