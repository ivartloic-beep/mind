# MIND sur iPhone (sans App Store)

Panneau mobile + capture rapide via l’API `https://mind.louetline.fr`.  
Fichiers : `public/mind/` (PWA).

## 1. Déployer / ouvrir la PWA

Les fichiers sont dans le dépôt :

```
public/mind/
  index.html
  app.css
  app.js
  manifest.json
  sw.js
  icons/
```

**Option A — héberger en HTTPS** (recommandé pour l’icône d’accueil) :

Copier `public/mind/` vers un chemin HTTPS, par ex. :

- `https://mind.louetline.fr/app/`  
  ou un sous-domaine / dossier de ton tunnel.

**Option B — test local** :

```bash
npx --yes serve public/mind -l 43129 --no-clipboard
# ou via le serveur Vite du repo :
# http://127.0.0.1:1420/mind/index.html
```

Puis ouvrir l’URL sur l’iPhone (même Wi‑Fi / tunnel).

### Installer l’icône

1. Safari → ouvrir l’URL de la PWA  
2. **Partager** → **Sur l’écran d’accueil** → **Ajouter**  
3. Nom : **MIND**

### Première connexion

1. Ouvrir MIND → ⚙  
2. Coller le **même Bearer** que dans MIND PC (Réglages → sync)  
3. Enregistrer  

Modes : **Jour** (file) · **Capturer** · **Rappels** (HH:MM).

Deep links : `?mode=capture` · `?mode=day` · `?mode=timer`.

---

## 2. Raccourci iOS « Capture MIND » (écran verrouillé)

Sans ouvrir l’app : un raccourci POST/PUT directement l’API.

### Créer le raccourci (une fois)

App **Raccourcis** → **+** → nommer **Capture MIND** :

1. **Demander une entrée**  
   - Type : Texte  
   - Invite : `Capture MIND`  
   - (optionnel) Autoriser plusieurs lignes  

2. **Texte** (ou « Générer UUID » si dispo) — pour l’id, tu peux utiliser :

   - Action **URL** n’est pas nécessaire pour l’id  
   - Astuce simple : action **Texte** avec  
     `m` + *Date actuelle* formatée + caractères aléatoires  
   - Ou action **Distribuer le texte** / script : le plus simple est  
     **Obtenir le contenu de l’URL** après avoir construit l’URL avec un id.

   **Méthode fiable :**

   1. Action **Texte** contenant uniquement :  
      `REPLACE_ID`  
   2. **Remplacer du texte** `REPLACE_ID` par un id :  
      utilise **UUID** (si l’action « Générer UUID » existe sur ta version)  
      sinon `m` + horodatage via **Date** → **Format** → `yyyyMMddHHmmss` + 4 chiffres du presse-papiers aléatoire.

3. **Obtenir le contenu de l’URL**

   - **URL** : `https://mind.louetline.fr/tasks/<ID>`  
     (remplace `<ID>` par la variable id générée)  
   - **Méthode** : `PUT`  
   - **En-têtes** :
     - `Authorization` : `Bearer TON_TOKEN`  
     - `Content-Type` : `application/json`  
   - **Corps de la requête** : JSON  

```json
{
  "id": "<ID>",
  "title": "<Texte demandé>",
  "notes": "",
  "status": "todo",
  "priority": 1
}
```

   Branche le titre sur le résultat de « Demander une entrée », et `id` sur la variable id.

4. **Afficher une notification** : `Capturé dans MIND`.

### Variante presse-papiers (encore plus rapide)

Remplace « Demander une entrée » par **Obtenir le presse-papiers**.  
Flux : copier un texte → déclencher le raccourci → tâche créée.

### Brancher le déclencheur

| Déclencheur | Réglage |
|-------------|---------|
| **Bouton Action** (15 Pro+) | Réglages → Bouton Action → Raccourci → *Capture MIND* |
| **Back Tap** | Accessibilité → Toucher → Toucher l’arrière → Raccourci |
| **Lock Screen** | Maintenir l’écran verrouillé → widget **Raccourcis** → *Capture MIND* |
| **Centre de contrôle** | Réglages → Centre de contrôle → contrôle Raccourcis |

iOS peut demander **Face ID** avant l’appel réseau : normal.

---

## 3. Sync avec le PC

Dès que le desktop a la sync cloud activée (même token), les captures iPhone apparaissent après sync (ou pull périodique).

---

## 4. Limites

- Pas de widget Lock Screen *natif* MIND sans compte Apple Developer.  
- Capture 100 % silencieuse sans Face ID : non.  
- Safari et la PWA « Sur l’écran d’accueil » ont des stockages séparés : configure le token **dans** l’icône installée.
