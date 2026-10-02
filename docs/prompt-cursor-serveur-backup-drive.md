---
cursor:
  subagentId: "bc-9065af54-d128-54bd-bcd2-361cf0996e1b"
---

# Prompt à coller dans Cursor sur le PC serveur

Objectif : **sauvegarde automatique** de Gestion (MySQL + fichiers uploads) vers **Google Drive**, au cas où le PC serveur lâche.

Ce n’est **pas** du sync live Drive pour l’app — uniquement une **copie de secours** hors machine.

Copier-coller le bloc ci-dessous tel quel dans un agent Cursor **sur le PC serveur**.

---

```
Mets en place une sauvegarde automatique de gestion-v2 vers Google Drive (rclone).

## Contexte

- Stack gestion déjà déployé en Docker sur cette machine (front + API sur https://gestion.louetline.fr, MySQL en container).
- Données critiques :
  1. base MySQL (projets, tâches, CRM, users…)
  2. fichiers uploadés (photos, docs…) — typiquement volume / dossier `api/uploads`
- Objectif : copie de secours hors PC serveur, pas un sync Drive dans l’app Windows.
- Ne casse rien d’autre (mind-api, tunnels Cloudflare, etc.).

## Objectif

1. Installer / réutiliser **rclone** configuré avec un remote Google Drive (ex. `gdrive:`)
2. Script quotidien `backup-gestion.sh` qui :
   - fait un `mysqldump` (ou `docker compose exec` mysqldump) compressé
   - archive le dossier `uploads` (tar.gz ou rclone sync)
   - envoie vers Drive dans un dossier dédié, ex. `gdrive:Backups/gestion-louetline/`
   - conserve une rétention (ex. 14 jours locaux + 30 jours Drive, ou prune des vieux dumps)
3. Cron / systemd timer tous les jours (ex. 03:30)
4. README : comment restaurer MySQL + uploads depuis un backup
5. Log simple + alerte basique si échec (fichier log ; optionnel : echo mail / webhook si déjà dispo)

## Détails techniques

- Inspecte d’abord où est le stack gestion (`docker compose`, noms des services `db` / `web`, volumes).
- Dump MySQL :
  - user/db depuis le `.env` du stack (ne pas hardcoder les secrets dans le script versionné)
  - fichier nommé `gestion-mysql-YYYYMMDD-HHMM.sql.gz`
- Uploads :
  - soit `tar czf gestion-uploads-YYYYMMDD.tar.gz` du volume monté
  - soit `rclone sync` direct du dossier uploads vers Drive (plus simple pour gros volumes)
- Drive :
  - sous-dossiers `mysql/` et `uploads/` (ou un dossier daté par run)
  - si rclone n’est pas encore autorisé sur ce compte Google : guide pas à pas `rclone config` (OAuth) et pause pour que je finisse l’auth dans le navigateur
- Ne committe jamais de tokens rclone / `.env` dans git

## Livrables

- `scripts/backup-gestion.sh` (idempotent, set -euo pipefail)
- entrée cron ou unit systemd `gestion-backup.timer`
- `.env.example` pour chemins / remote rclone / rétention (sans secrets)
- `README-BACKUP.md` : install, test manuel, restore

## Vérifications

1. Lancer le script une fois à la main → fichiers visibles sur Drive
2. Vérifier qu’un vieux dump serait bien purgé selon la rétention (dry-run OK)
3. Documenter la commande de restore MySQL + recopie uploads

## Contraintes

- Backup = secours, pas de modification du code métier gestion-v2
- Minimal, fiable, logs clairs
- Si rclone / Drive est déjà configuré pour autre chose, réutilise le remote existant

Commence par localiser le compose gestion + les volumes, puis propose le plan court et implémente.
```

---

## Après mise en place

1. Vérifie sur Drive le dossier `Backups/gestion-louetline/` (ou équivalent)
2. Une fois par mois : test restore sur une machine jetable / DB temporaire
3. MIND Windows n’a rien à configurer pour ça — c’est 100 % serveur
