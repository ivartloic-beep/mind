---
cursor:
  subagentId: "bc-9065af54-d128-54bd-bcd2-361cf0996e1b"
---

# Prompt à coller dans Cursor sur le PC serveur

Objectif : mettre en ligne **gestion-v2** (PHP + MySQL) pour que l’app Windows MIND puisse appeler l’API (`login.php`, `personal_tasks.php`, etc.).

Copier-coller le bloc ci-dessous tel quel dans un agent Cursor **sur le PC serveur** (là où Docker + Cloudflare Tunnel existent déjà, à côté de mind-api).

---

```
Déploie gestion-v2 (PHP/MySQL) sur cette machine pour servir l’API à l’app Windows MIND.

## Contexte

- Repo source : https://github.com/ivartloic-beep/gestion-v2
- Sur mon PC Windows j’ai seulement le dossier de l’app ; il n’y a pas encore d’API en ligne.
- J’ai déjà Docker + Cloudflare Tunnel pour d’autres apps (dont éventuellement mind-api sur un port local type 8787).
- L’app Windows MIND (Tauri) a besoin d’une URL absolue du type :
  https://gestion.MONDOMAINE.tld/api
  (sans slash final) pour login + personal_tasks + le reste.

## Objectif

1. Stack Docker isolé qui sert :
   - le front gestion (optionnel mais utile pour tester dans le navigateur)
   - surtout le dossier `api/` en HTTP(S) via tunnel
2. MySQL avec volumes persistants
3. `config.php` généré depuis `api/config.example.php` avec secrets forts
4. Un premier utilisateur admin utilisable pour se connecter depuis MIND
5. Hostname Cloudflare Tunnel pointant vers le service PHP local, sans casser les tunnels existants
6. README clair : URL API à coller dans MIND (Paramètres → Gestion)

## Stack à créer

Dossier dédié, ex. `~/apps/gestion-v2` (ou à côté de mes autres stacks).

### docker-compose.yml

Services :
- `db` : `mysql:8` (ou mariadb:11)
  - volume persistant
  - user/db/password via `.env`
  - **non exposé** sur Internet (réseau Docker seulement ; pas de publish 3306 public)
- `web` : image PHP + Apache (ex. `php:8.2-apache`) avec extensions PDO MySQL, json, mbstring, gd si besoin
  - monte le code gestion-v2
  - DocumentRoot = racine du projet (pour servir `index.html` + `api/`)
  - écoute en local seulement, ex. `127.0.0.1:8088:80` (choisis un port libre)
  - active `mod_headers` / `mod_rewrite` si utile
  - passe correctement le header `Authorization` vers PHP (souvent nécessaire : `CGIPassAuth On` ou `SetEnvIf Authorization "(.*)" HTTP_AUTHORIZATION=$1` dans Apache) — critique pour `verifyToken()`

Réseau Docker dédié `gestion`.

### Code

- Clone ou copie https://github.com/ivartloic-beep/gestion-v2 dans ce dossier
- Crée `api/config.php` depuis `api/config.example.php` :
  - DB_HOST = nom du service docker `db`
  - DB_NAME / DB_USER / DB_PASS depuis `.env`
  - SESSION_LIFETIME OK
  - VAPID / GROQ / MAIL_* : placeholders OK pour V1 (pas bloquant pour login + tâches)
- CORS : assure-toi que les réponses API acceptent les appels depuis l’app desktop Tauri (Origin variable). Si le code n’a pas de CORS global, ajoute un petit bootstrap ou vhost qui envoie :
  - `Access-Control-Allow-Origin: *` (ou liste)
  - `Access-Control-Allow-Headers: Content-Type, Authorization, X-Auth-Token`
  - `Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS`
  - réponse 204 aux preflight OPTIONS

### Base de données / premier user

- Au premier boot, crée le schéma minimal nécessaire si le repo ne l’auto-crée pas entièrement :
  - au minimum tables `users` + sessions (comme attendu par `login.php` / `verifyToken`)
  - les tables métier (`personal_tasks`, etc.) sont souvent créées à la volée par les endpoints — laisse-les s’auto-créer quand c’est déjà le cas dans le code
- Crée **un utilisateur admin** (username + password hash `password_hash` PHP) documenté dans le README
- Script one-shot acceptable : `scripts/bootstrap-admin.php` ou SQL + bcrypt généré

### Cloudflare Tunnel

- Inspecte d’abord ma config cloudflared / compose existante
- Ajoute une Public Hostname, ex. `gestion.MONDOMAINE.tld` → `http://127.0.0.1:8088`
- Diff minimal, ne casse pas mind-api ni les autres apps
- L’URL API pour MIND sera alors : `https://gestion.MONDOMAINE.tld/api`

### Fichiers livrables

- `docker-compose.yml`
- `.env.example` + `.env` local (secrets forts, hors git)
- `README.md` avec :
  - `docker compose up -d`
  - health : `curl -sS http://127.0.0.1:8088/api/login.php` (méthode GET peut renvoyer 405 — OK si PHP répond)
  - test login :
    `curl -sS -X POST http://127.0.0.1:8088/api/login.php -H 'Content-Type: application/json' -d '{"username":"...","password":"..."}'`
  - URL publique à coller dans MIND Windows
  - comment créer / reset l’admin

### Vérifications

1. `docker compose up -d` OK
2. Login curl local OK (token renvoyé)
3. `personal_tasks.php` avec Bearer token OK (liste vide ou tasks)
4. Tunnel public OK : même login via `https://gestion.…/api/login.php`
5. Donne-moi clairement la ligne à coller dans MIND :
   Paramètres → Gestion → API = `https://gestion.…/api`

## Contraintes

- Ne casse rien d’autre sur le serveur
- Ne mélange pas avec mind-api (stack / DB / réseau séparés)
- Pas besoin de réécrire Gestion en Node/React
- Pas de messagerie/mails obligatoires pour V1 serveur
- Code / config clairs, minimal, prêts pour MIND desktop

Commence par inspecter l’organisation Docker + cloudflared existante ici, puis déploie gestion-v2 en cohérence.
```

---

## Après déploiement (sur le PC Windows MIND)

1. Arrêter MIND → Mettre à jour MIND → Ouvrir MIND  
2. Panneau → ⚙ → **Gestion** → coller `https://gestion.TONDOMAINE/api`  
3. **Se connecter** (user admin créé sur le serveur)  
4. Optionnel : **Importer tâches locales**
