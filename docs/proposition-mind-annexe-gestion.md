---
cursor:
  subagentId: "bc-9065af54-d128-54bd-bcd2-361cf0996e1b"
---

# Proposition — MIND comme annexe de Gestion

Panneau cible : **400 × 720**, **sans scroll**, aéré (pas une grille d’icônes).  
Principe : MIND **capture et oriente** ; Gestion **détaille et archive**.

---

## 1. Vision

MIND n’est pas un second Gestion. C’est une **télécommande permanente** collée au bord de l’écran :

1. **Capturer** vite (tâche, note, idée, fichier, contact).
2. **Voir l’essentiel** du jour (3–4 items max, avec attributs).
3. **Ouvrir juste** la bonne fiche Gestion (docs, CRM, projet…).
4. **Rester sync** dans les deux sens, sans que Loïc y pense.

Si une action demande plus de 2 champs ou une liste longue → **feuille éphémère** (capture / dépôt) ou **fenêtre Gestion**, jamais le panneau principal.

---

## 2. Budget d’écran (contrainte dure)

Hauteur utile ≈ 720 − poignée − marges ≈ **650–680 px**.  
Sans scroll, le panneau ne peut porter que **4 zones** :

| Zone | Rôle | Hauteur indicative |
|------|------|--------------------|
| A. Identité + statut Gestion | MIND + « connecté » | ~70 px |
| B. Actions (1 rangée) | 4 raccourcis max | ~56 px |
| C. File du jour | 3–4 lignes riches | ~280 px |
| D. Déposer / Minuteur | 1 bloc contextuel | ~160 px |

**À retirer du panneau principal** (déjà trop chargé aujourd’hui) :

- liste Notes + Tâches empilées (5+5) ;
- gros bloc Paramètres (déjà une vue séparée — garder) ;
- minuteur **permanent** (le basculer en mode, voir §4).

Bibliothèque et fiche riche restent des fenêtres dédiées.

---

## 3. Architecture proposée : 3 modes, 1 panneau

Pas d’onglets denses. Un **mode** à la fois, bascule en 1 clic (segment 3 positions en zone B) :

### Mode **Jour** (défaut)

- 3–4 **tâches ouvertes** les plus urgentes (échéance / priorité / statut).
- Clic titre → fiche Gestion (déjà en place).
- Pied : **zone Déposer** (drag-and-drop) toujours visible.

### Mode **Capturer**

Remplace la file par un mini-choix :

- Tâche · Note · Idée · Contact · Fichier  

Puis ouvre la **fenêtre Capture** déjà existante (ou une variante contact/fichier).  
Le panneau lui-même ne devient pas un formulaire.

### Mode **Minuteur**

Uniquement le timer + 1 ligne « tâche liée » optionnelle.  
Rien d’autre — respire.

Paramètres restent derrière ⚙ (vue plein panneau, scroll OK **uniquement** là).

---

## 4. Ce qu’on peut ajouter (sans fouillis)

### A. Déjà utile / à durcir (socle annexe)

| Capacité | Sens sync | Où |
|----------|-----------|-----|
| Tâches personal_tasks | ↔ | Capture + file Jour + fiche Gestion |
| Notes / idées bureau ou projet | ↔ | Capture |
| Projets (Production + espaces) | ↔ | Select capture / biblio |
| Session Gestion | — | Paramètres |
| Sync auto (focus, fermeture Gestion, poll) | ↔ | Invisible |

### B. Prochaines briques « raccourci Gestion » (recommandées)

#### 1) Zone **Déposer** (priorité haute)

- Zone unique en bas du mode Jour : « Déposer un fichier ».
- Au drop → **feuille courte** (pas le panneau) :
  - Destination : Bureau · Projet · Fiche tâche · Contact CRM (si contexte).
  - Titre optionnel.
- Envoi via API workspace / pièce jointe tâche.
- **Jamais** de navigateur de dossiers dans le panneau.

#### 2) **Contact CRM express**

- Depuis Capturer → « Contact ».
- Feuille : Nom, Organisme, Tél/mail (3 champs).
- Crée le prospect Gestion ; bouton « Ouvrir fiche » après création.
- Pas de pipeline / deals dans le panneau.

#### 3) **Raccourcis « Ouvrir dans Gestion »** (4 max, rangée B)

Exemple figé (pas une app drawer) :

1. Capturer  
2. Bureau  
3. CRM  
4. Projets  

Chaque bouton = `gestion_show` + deep-link JS (`openMyBureau`, `openCrmPage`, liste projets).  
Zéro sous-menu.

#### 4) **File du jour intelligente** (remplace « Récent »)

Au plus **4 lignes**, score simple :

1. Tâches en retard  
2. Échéance aujourd’hui / demain  
3. Priorité haute  
4. Dernière capturée  

Afficher : titre + chips statut / priorité / date (déjà amorcé).  
Notes : **pas** dans cette file — elles restent en Capture + Bibliothèque / Bureau Gestion.

#### 5) **Inbox légère** (option)

Badge « 2 à classer » si fichiers / captures sans projet.  
Clic → feuille de classement, pas une liste infinie dans le panneau.

### C. Plus tard (utile mais pas dans le panneau)

| Idée | Pourquoi plus tard / ailleurs |
|------|-------------------------------|
| Créer un deal CRM | Trop de champs → Gestion |
| Compta / NDF / validation | Rare, lourd → Gestion |
| Messagerie / mails | Hors annexe productivité |
| Édition riche note / mindmap | Capture + Gestion |
| Multi-assignés, sous-tâches | Fiche Gestion |
| Catalogue / invitations | Hors scope panneau |

---

## 5. Contrat de synchronisation (annexe réelle)

Pour que ça tienne comme une annexe, pas un miroir bancal :

1. **Source de vérité Gestion** dès qu’une session est active (pull = prune local manquant — déjà pour notes/tâches).
2. **Push immédiat** à chaque création / édition / suppression MIND.
3. **Pull** : fermeture Gestion, focus panneau, poll ~20–30 s, bouton Sync.
4. **Deep-links stables** : tâche, note bureau, note projet, contact, dépôt fichier.
5. **Hors-ligne** : écriture locale + pastille « cache » ; flush au retour réseau (sans ressusciter les suppressions Gestion).
6. **Pas de double modèle** : pas de « tâches MIND » vs « tâches Gestion » visibles pour Loïc.

---

## 6. Maquette mentale du mode Jour

```
┌─────────────────────────────┐
│ MIND              ⚙         │
│ Gestion · Loïc              │
├─────────────────────────────┤
│ [Capturer] [Bureau] [CRM] [Projets] │
├─────────────────────────────┤
│ Aujourd’hui                 │
│ ○ Relancer devis            │
│   En cours · Haute · 3 oct  │
│ ○ Préparer tournée          │
│   À faire · Moyenne · 4 oct │
│ ○ Appeler Martin            │
│   À faire · Haute           │
├─────────────────────────────┤
│ ┌─────────────────────────┐ │
│ │  Déposer un fichier     │ │
│ │  → Bureau / Projet / …  │ │
│ └─────────────────────────┘ │
│ Jour │ Capturer │ Minuteur  │
└─────────────────────────────┘
```

Aéré : beaucoup d’air, **3 tâches max visibles**, pas de notes ici, pas de timer permanent.

---

## 7. Phasage proposé

### Phase A — Panneau « annexe » (layout)

- Repasser le panneau en **3 modes** sans scroll.
- File du jour (≤4 tâches) avec chips.
- Rangée 4 deep-links Gestion.
- Minuteur isolé dans son mode.

### Phase B — Déposer

- Drop zone + feuille destination (Bureau / Projet / Tâche).
- Upload workspace ou PJ tâche.
- Sync visible (toast discret « Déposé dans … »).

### Phase C — CRM express

- Capture contact (3 champs) → API CRM.
- Deep-link ouverture fiche.

### Phase D — Polish sync

- Indicateur sync (ok / cache / erreur) d’une ligne.
- Deep-links note projet + bureau depuis la bibliothèque.
- Tests manuels checklist aller-retour.

---

## 8. Critères de réussite

- Le panneau **ne scroll jamais** en mode Jour / Capturer / Minuteur.
- En &lt; 5 s : créer une tâche ou une note qui apparaît dans Gestion.
- En &lt; 5 s : déposer un PDF au bon endroit Gestion.
- Clic tâche → fiche Gestion complète (docs inclus) — déjà amorcé.
- Zéro sensation de « deuxième logiciel » : mêmes projets, mêmes tâches, mêmes notes.

---

## 9. Recommandation immédiate

Enchaîner **Phase A puis B** : c’est ce qui transforme MIND en vraie annexe sans alourdir.  
CRM express (C) ensuite — fort gain, mais secondaire au flux fichiers + tâches déjà amorcé.

À valider avec toi avant code :

1. OK pour **retirer les notes** de la file panneau (gardées en Capture / Bibliothèque / Bureau) ?  
2. OK pour **3 modes** (Jour / Capturer / Minuteur) plutôt que tout empiler ?  
3. Destinations drop V1 : **Bureau + Projet** seulement, ou aussi **PJ sur une tâche** ?
