# Proposition — « Analyser l’écran » (Capture)

**Verdict : faisable sous Windows + Tauri.** Pas d’API Gmail. Screenshot + extraction, puis champs éditables. Cloud **pas obligatoire**.

Aujourd’hui : Capture (CTRL+ALT+N) = Tâche / Note / Idée (fenêtre centrale 480×540). Le CRM express (nom, organisme, tél, email) est dans le **panneau droit**, mode Capturer. Le bouton **« Analyser l’écran »** s’y branche.

---

## 1. Faisabilité Windows / Tauri

| Brique | Verdict | Comment |
|--------|---------|---------|
| Screenshot | Oui | Commande Rust : DXGI / `BitBlt`, ou crate type `xcap`. Pas besoin d’un plugin Tauri dédié. |
| Exclure le panneau MIND | Oui | On connaît le rectangle du panneau (`~268 px` bord droit) et de la fenêtre Capture. Soit **masquer** panneau + capture 1 frame puis shot, soit **recadrer / noircir** ces HWND sur le bitmap. Recadrage = moins de flash. |
| OCR local | Oui | **Windows.Media.Ocr** (Win10+, déjà sur la machine). Suffisant pour emails, tél., URLs, beaucoup de noms visibles. |
| Vision LLM | Oui, optionnel | Meilleur sur « qui est le contact vs le CC / le logo ». Coût + envoi du mail. |

**Reco technique V1 :** screenshot du moniteur de la fenêtre au premier plan → masque des fenêtres MIND → OCR Windows → heuristiques (regex email/tél, lignes « De : », signature) → JSON champs CRM. Pas de LLM requis pour un premier slice utile (Gmail web classique).

---

## 2. Flux UX (V1 livrée)

Objectif : Gmail reste visible à gauche ; correction à **droite** dans le panneau (pas dans la Capture centrale, qui cacherait le mail).

1. Raccourci Capture (existant) **ou** panneau → mode **Capturer**.
2. Choisir le type **CRM** (Capture centrale ou bouton CRM du panneau).
3. Bouton **« Analyser l’écran »** (au-dessus de Nom / Organisme / Tél / Email).
4. MIND range la Capture centrale si elle est ouverte ; le panneau **reste visible**. Les HWND MIND (panneau + capture) sont **masqués en blanc sur le bitmap**, pas de flash 200 ms.
5. Capture du moniteur du panneau (Gmail à gauche sur le même écran).
6. OCR local + heuristiques → préremplit **Nom**, **Organisme**, **Tél**, **Email** (champs séparés).
7. État **« Relis avant de créer »** : champs éditables, Gmail toujours à gauche, panneau à droite.
8. **Créer** → prospect Gestion. Rien n’est poussé sans validation.

Pas d’écriture automatique dans Gestion. Si l’OCR est vide / douteux : message d’erreur + champs inchangés.

---

## 3. Locale vs API

| | Locale (reco V1) | API vision (plus tard) |
|--|------------------|------------------------|
| Moteur | Windows OCR + règles | GPT / Claude vision (image JPEG) |
| Confidentialité | Le mail **ne quitte pas** le PC | Capture d’écran = contenu du mail chez un tiers |
| Coût | 0 | Quelques centimes / analyse ; clé API à coller |
| Qualité | Bonne sur texte net ; faible si layout chargé / image-only | Meilleure (expéditeur vs destinataire, organisme) |
| Dépendance | Win10+ | Réseau + secret |

**Pas de cloud obligatoire.** Un interrupteur Paramètres « Analyse cloud (optionnel) » peut venir en V2 si la V1 locale rate trop Gmail.

Ne pas envoyer le screenshot vers `mind.louetline.fr` ni Gestion : hors sujet et trop sensible.

---

## 4. Limites

- **Écran verrouillé / session inactive** : rien d’utile à capturer.
- **Multi-écran** : V1 capture le moniteur du panneau (Gmail à gauche sur le même écran). Clique d’abord dans Gmail si tu es en dual-screen.
- **Gmail web** : OK si le fil est à l’écran (texte DOM). Barre latérale, pubs, « À : toi » mélangent les emails. La signature est plus fiable que le header CC.
- **Gmail app / autre client** : même principe screenshot ; pas d’accès API.
- **Fenêtre trop petite / scroll** : on ne voit que le viewport.
- **HDR / scaling DPI** : à tester ; coordonnées panneau en DIP vs pixels physiques.
- **Erreurs** : OCR vide, mauvais nom (logo, « Equipe support »), email de Loïc au lieu du prospect. D’où **toujours éditer**.
- **Capture centrale ouverte** : si on analyse sans la fermer, elle recouvre Gmail — d’où l’affichage résultat **dans le panneau**, Capture fermée ou hors écran.

---

## 5. Décisions VALIDÉES (Loïc)

- **Cible V1 = CRM seulement** (Contact express). Pas Tâche / Note.
- **Résultat dans le panneau droit** (Gmail reste à gauche).
- **Moteur V1 = OCR Windows local uniquement** (`Windows.Media.Ocr` + heuristiques). Pas de cloud.
- **Shot = recadrer / masquer les HWND MIND** sur le bitmap. Pas de flash 200 ms (on ne cache pas tout MIND).
- **Champs séparés dès la V1** : Nom, Organisme, **Tél**, **Email**.

Hors V1 (volontaire) : iPhone, analyse Tâche depuis un PDF, plugin Gmail, envoi cloud par défaut.
