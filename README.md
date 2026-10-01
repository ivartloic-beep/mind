# Ma Tête

App Windows locale-first : capture rapide, panneau latéral, post-it natifs.  
Stack : **Tauri 2** + **React** + **TypeScript** + **Vite**.

Nom provisoire : **Ma Tête**. Étape 1 = squelette multi-fenêtres uniquement (pas de storage, tray, raccourcis ni features métier).

## Prérequis

- Node.js 20+ et npm
- [Rust](https://rustup.rs/) (stable)
- Sur **Windows** : [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (généralement déjà présent)
- Sur **Linux** (dev / CI) : `libwebkit2gtk-4.1-dev`, `librsvg2-dev`, `patchelf` — voir [prerequisites Tauri](https://tauri.app/start/prerequisites/)

Cible produit : **Windows** uniquement (installeur **NSIS**).

## Lancer

```bash
npm install
npm run tauri dev
```

Équivalents utiles :

```bash
# Frontend seul (multi-entry Vite)
npm run dev
npm run build

# Vérifier le backend Rust sans lancer l'UI
cd src-tauri && cargo check
```

Au démarrage étape 1 : la fenêtre **panel** (~400 px) s’affiche ; `main`, `capture` et `postit` sont définies mais masquées (stubs).

## Entries frontend

| Entry | Fichier | Rôle |
|---|---|---|
| `main` | `index.html` → `src/main.tsx` | Host (tray/raccourcis plus tard) |
| `panel` | `panel.html` → `src/panel.tsx` | Panneau latéral |
| `capture` | `capture.html` → `src/capture.tsx` | Capture rapide |
| `postit` | `postit.html` → `src/postit.tsx` | Post-it natif |

## Architecture

Voir le document de référence dans le store projet : `docs/ma-tete-v1-architecture.md`.

## État compile sur cette VM

Environnement Linux (pas Windows). Vérifications locales :

- `npm run build` — frontend multi-entry
- `cargo check` dans `src-tauri` — backend Tauri

`tauri build` Windows / NSIS nécessite une machine Windows + WebView2.
