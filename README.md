# Ma Tête

App Windows locale-first : capture rapide, panneau latéral, post-it natifs.  
Stack : **Tauri 2** + **React** + **TypeScript** + **Vite**.

Nom provisoire : **Ma Tête**.

## État actuel

- **Étape 1** : squelette multi-fenêtres (`main` / `panel` / `capture` / `postit`)
- **Étape 2** : stockage SQLite (`rusqlite`), modèles, CRUD Tauri, event `data-changed`
- **Étape 3** : panneau latéral droit (~400 px), poignée slide, always-on-top mémorisé
- **Étape 4** : capture rapide (fenêtre légère, Task/Note/Idée, depuis le panneau)
- **Étape 5** : tâches dans le panneau (liste, cocher/décocher, terminées)
- **Étape 6** : notes dans le panneau (créer / éditer / lister)
- **Étape 7** : projets optionnels + filtres Tout / Sans projet / Projet
- **Étape 8** : post-its fenêtres natives (`postit-{id}`), persistés, liés à Note
- **Étape 9** : minuteur 25/5 (démarrer / pause / notif Windows)
- **Étape 10** : rappels tâche (🔔, notif Windows, snooze +10 min / +1 h / demain)
- **Étape 11** : raccourcis globaux CTRL+ALT+N (capture) / CTRL+ALT+Espace (panneau) / CTRL+ALT+B (bibliothèque) — liste aussi dans Paramètres
- **Étape 12** : tray + menu, autostart ON, single-instance, boot sans flash
- **UX** : capture fermable (✕/Échap) · post-its autonomes (CTRL+ALT+P, croix=supprimer, CTRL+ALT+H masquer/réafficher) · panneau compact + Bibliothèque (réouvrable) · minuteur durée libre (+5 min)
- **Cloud** : sync optionnelle vers `https://mind.louetline.fr` (Réglages → token + Synchroniser)

V1 desktop + polish UX. Test réel Windows recommandé via installateur **NSIS**.

## Prérequis

- Node.js 20+ et npm
- [Rust](https://rustup.rs/) (stable, ≥ 1.90 recommandé pour Tauri 2 récent)
- Sur **Windows** : [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)
- Sur **Linux** (dev / CI) : `libwebkit2gtk-4.1-dev`, `librsvg2-dev`, `patchelf` — [prerequisites Tauri](https://tauri.app/start/prerequisites/)

Cible produit : **Windows** (installeur **NSIS**).

## Usage quotidien (sans terminal)

`npm run tauri dev` ouvre toujours un terminal — c’est le mode développement.
Pour une app normale (icône Bureau, pas de console) :

```powershell
cd mind
git pull origin main
npm install
npm run desktop:build
```

Puis lance l’installeur généré :

`src-tauri\target\release\bundle\nsis\Ma Tête_*_x64-setup.exe`

- Coche / accepte le raccourci **Bureau** (aussi créé automatiquement).
- Démarre via l’icône **Ma Tête** sur le Bureau (pas de fenêtre noire).
- Arrêt : icône tray (barre des tâches) → **Quitter**.
- Redémarrage : recliquer l’icône Bureau.

## Lancer en développement

```bash
npm install
npm run desktop:dev
```

Équivalents :

```bash
npm run dev
npm run build
cd src-tauri && cargo check
cd src-tauri && cargo test   # persistance SQLite (reopen)
```

DB locale : `{app_data_dir}/ma-tete.db`.  
Prefs sync : `{app_data_dir}/sync-prefs.json` (URL + token, hors git).

### Sync cloud

1. Panneau → **⚙ Paramètres** → Cloud → **Configurer** → coller le Bearer token  
2. **Tester la connexion** puis **Synchroniser**  
3. Si « Synchronisation » cochée : sync auto toutes les 5 min  

### Raccourcis Bureau (Windows)

```powershell
cd mind
git pull origin main
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\create-desktop-shortcuts.ps1
```

- **Mettre a jour MIND** — `git pull` + `npm install`  
- **Ouvrir MIND** — lance `desktop:dev` (code Git à jour)  
- **Arreter MIND** — ferme toutes les instances

## Entries frontend

| Entry | Fichier | Rôle |
|---|---|---|
| `main` | `index.html` → `src/main.tsx` | Host |
| `panel` | `panel.html` → `src/panel.tsx` | Panneau |
| `capture` | `capture.html` → `src/capture.tsx` | Capture |
| `postit` | `postit.html` → `src/postit.tsx` | Post-it |

## Architecture

Référence store : `docs/ma-tete-v1-architecture.md`.
