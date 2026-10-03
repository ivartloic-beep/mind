# Prompt à coller dans Cursor sur le PC serveur

Copier-coller le bloc ci-dessous **tel quel**.

Les fichiers PWA sont dans le dépôt desktop GitHub  
`ivartloic-beep/mind` → branche `cursor/ma-tete-iphone-pwa-6e1b` (ou `main` si déjà mergée)  
dossier `public/mind/`.

---

```
Déploie la PWA mobile MIND derrière https://mind.louetline.fr/app/ sur CE serveur.

## Contexte

- J’ai déjà **mind-api** exposé via Cloudflare Tunnel sur `https://mind.louetline.fr`
  (API : `/health`, `/tasks`, `/notes`, `/reminders`, … — Bearer token).
- L’app Windows sync déjà vers cette API.
- Dans le repo GitHub `ivartloic-beep/mind`, branche `cursor/ma-tete-iphone-pwa-6e1b`
  (fallback `main` si la PWA y est), il y a un dossier statique `public/mind/` :
  `index.html`, `app.css`, `app.js`, `manifest.json`, `sw.js`, `icons/`.
- Je veux pouvoir ouvrir sur iPhone :
  `https://mind.louetline.fr/app/`
  puis Safari → Partager → Sur l’écran d’accueil.

## Objectif

Servir ces fichiers en **HTTPS** sous le chemin `/app/` (ou `/app/index.html`)
**sans casser** l’API existante sur le même hostname.

## Ce que tu dois faire

1. **Inspecte** comment mind-api + Cloudflare Tunnel sont organisés ici
   (docker-compose, reverse-proxy, cloudflared, chemins). Ne suppose rien.

2. **Récupère** les fichiers PWA :
   - clone/pull du repo `https://github.com/ivartloic-beep/mind`
     branche `cursor/ma-tete-iphone-pwa-6e1b` (sinon `main`),
   - ou copie depuis un chemin local si le repo est déjà là,
   - source : dossier `public/mind/` → cible serveur dédiée, ex. `~/apps/mind-api/public/app/`
     (adapte au layout réel).

3. **Sers** le contenu en statique sur `https://mind.louetline.fr/app/` :
   - Choisis le diff **minimal** selon l’infra :
     A) static file serving dans l’API Node (Hono/Fastify/Express) monté sur `/app`, ou
     B) nginx/Caddy devant qui route `/app/*` → fichiers et le reste → API, ou
     C) petit conteneur `nginx:alpine` sur un port local + règle Tunnel/path.
   - Les assets doivent résoudre en relatif (`./app.js`, `./manifest.json`, icons).
   - `GET /app` → **301/302 vers `/app/`** (critique : sans slash, `./app.css` devient `/app.css` = 401 API).
   - `GET /app/` et `GET /app/index.html` → 200 HTML PWA.
   - `GET /app/manifest.json`, `/app/sw.js`, `/app/icons/icon-180.png` → 200.
   - L’API continue : `GET /health` → `{"ok":true,...}`, `GET /tasks` avec Bearer inchangé.
   - CORS API déjà ouvert : ne le casse pas.
   - Service worker : scope `/app/` (déjà prévu dans la PWA).

4. **Cache / headers** (simple) :
   - HTML : `Cache-Control: no-cache` (ou max-age court)
   - `sw.js` : no-cache
   - icons / css / js versionnés par déploiement : cache OK (max-age raisonnable)

5. **Vérifications** (documente les commandes et résultats) :
   - `curl -sS https://mind.louetline.fr/health`
   - `curl -sSI https://mind.louetline.fr/app/` → 200, content-type HTML
   - `curl -sS https://mind.louetline.fr/app/ | head` → titre/title MIND, pas une page d’erreur API
   - `curl -sSI https://mind.louetline.fr/app/manifest.json` → 200 JSON
   - `curl -sSI https://mind.louetline.fr/app/sw.js` → 200
   - `curl -sSI https://mind.louetline.fr/app/icons/icon-180.png` → 200 image
   - Confirme qu’un `GET /tasks` sans token renvoie toujours 401 (API vivante)

6. **README court** à côté du déploiement : comment mettre à jour la PWA
   (`git pull` + recopier `public/mind/` + restart si besoin).

## Contraintes

- Ne casse **rien** d’autre (autres tunnels, Gestion, backups…).
- Ne change pas le schéma DB ni les routes API sauf si indispensable pour servir `/app`.
- Pas de login web supplémentaire : la PWA gère le Bearer en localStorage.
- Pas d’App Store, pas de compte Apple.
- Diff minimal, conventions du serveur respectées.
- Si le repo GitHub n’est pas accessible depuis ce PC, dis-le clairement et
  indique exactement quels fichiers je dois coller (liste + chemins cibles).

Commence par inspecter l’existant mind-api / cloudflared, puis déploie `/app`.
```
