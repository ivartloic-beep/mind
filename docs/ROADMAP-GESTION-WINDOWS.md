# Feuille de route — Gestion Windows + MIND

## Décisions

- Migration **hybride** (pas de full rewrite React d’un coup).
- Tâches : **fusion**, modèle **gestion** (`personal_tasks` / fiche tâche).
- Pas de messagerie ni mails dans le parcours Windows.
- Événementiel → **Production** (labels UX d’abord).
- Multi-utilisateur conservé (API users / permissions).

## Phases

### Phase 1 — Shell (livré)

- Front gestion embarqué (`public/gestion/`) dans Tauri.
- Fenêtre principale = app gestion.
- Nav : masquer Messagerie / Mails ; renommer Production.
- `API_URL` configurable (serveur PHP existant).
- MIND inchangé en overlay : tray, panneau, capture, timer, raccourcis.

### Phase 2 — Tâches unifiées (livré)

- Modèle Task MIND = schéma gestion (`todo` / `in_progress` / `done`, priority medium, description, category, assignedTo, documents, activities…).
- Si session Gestion (URL + token) : capture / panneau / bibliothèque CRUD → `personal_tasks.php`.
- Sinon : SQLite local (même schéma élargi).
- Rappels MIND (`reminder`) restent locaux (overlay).
- Login depuis Paramètres panneau, ou session reprise depuis la WebView Gestion.
- Import one-shot : bouton « Importer tâches locales ».
- Fiche tâche riche (docs / activités / assignation) via UI Gestion.
- Capture tâche : mêmes champs Gestion (description, priorité, échéance, statut, notes, projet → category).
- Projets MIND ↔ `projects.php` + `work_projects.php` (liste / création / rename / delete + sync au login).
- Notes / idées MIND ↔ bureau Gestion (`workspace.php`, types `page` / `idea`, visibility `personal`).

### Phase 3 — Durcissement (en cours)

- Cache SQLite si l’API `personal_tasks` / `work_projects` est injoignable (lecture + écriture locale, `lastError` en prefs).
- Réécriture React **par module** seulement si le WebView limite (Bureau, Production…).

## Config

Fichier local : `{app_data}/gestion-prefs.json`

```json
{
  "apiUrl": "https://votre-domaine.tld/chemin/api",
  "authToken": "…",
  "userId": 1,
  "userName": "Loïc",
  "tasksMigratedAt": "2026-…"
}
```

Aussi : Paramètres MIND → section Gestion, ou `localStorage.mind_gestion_api_url` / `authToken` dans la WebView.
