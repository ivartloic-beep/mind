# Prompt — correctif PWA MIND (chemins /app)

Copier-coller tel quel sur le PC serveur.

---

```
Correctif urgent pour https://mind.louetline.fr/app/

## Problème

Sans slash final, `https://mind.louetline.fr/app` charge le HTML mais les
chemins relatifs `./app.css` / `./app.js` deviennent
`https://mind.louetline.fr/app.css` et `…/app.js` → l’API répond 401.
Résultat iPhone : page blanche/gris, tous les blocs empilés, sans style.

## À faire

1. Pull `main` du repo `https://github.com/ivartloic-beep/mind`
   (commit avec fix `<base>` + redirect `/app` → `/app/` dans `public/mind/`).
2. Recopie `public/mind/` vers le dossier servi sous `/app/`.
3. Ajoute une redirection serveur : `GET /app` → `301/302 Location: /app/`.
4. Redémarre le service si besoin.
5. Vérifie :
   - `curl -sSI https://mind.louetline.fr/app` → redirect vers `/app/`
   - `curl -sSI https://mind.louetline.fr/app/app.css` → 200 text/css
   - `curl -sS https://mind.louetline.fr/app/ | head` → contient `<base href="/app/">` ou redirect JS
   - `curl -sS https://mind.louetline.fr/health` toujours OK

Ne casse pas l’API.
```
