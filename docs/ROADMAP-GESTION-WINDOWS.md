# Feuille de route — Gestion Windows + MIND

## Décisions

- Migration **hybride** (pas de full rewrite React d’un coup).
- Tâches : **fusion**, modèle **gestion** (`personal_tasks` / fiche tâche).
- Pas de messagerie ni mails dans le parcours Windows.
- Événementiel → **Production** (labels UX d’abord).
- Multi-utilisateur conservé (API users / permissions).

## Phases

### Phase 1 — Shell (en cours)

- Front gestion embarqué (`public/gestion/`) dans Tauri.
- Fenêtre principale = app gestion.
- Nav : masquer Messagerie / Mails ; renommer Production.
- `API_URL` configurable (serveur PHP existant).
- MIND inchangé en overlay : tray, panneau, capture, timer, raccourcis.

### Phase 2 — Tâches unifiées

- Capture / panneau MIND → CRUD `personal_tasks` (schéma gestion).
- Import one-shot des tâches SQLite MIND actuelles.
- Fiche tâche (assignation, docs, activités) via UI gestion ou pont.

### Phase 3 — Durcissement

- Offline / sync ciblée si besoin.
- Réécriture React **par module** seulement si le WebView limite (Bureau, Production…).

## Config

Fichier local : `{app_data}/gestion-prefs.json`

```json
{ "apiUrl": "https://votre-domaine.tld/chemin/api" }
```

Aussi : Paramètres MIND → section Gestion, ou `localStorage.mind_gestion_api_url` dans la WebView.
