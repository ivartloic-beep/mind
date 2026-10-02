/**
 * Zone déposer → workspace Gestion (bureau ou projet).
 */

import { useCallback, useEffect, useState } from "react";
import { listProjects } from "../../services/api";
import {
  gestionGetConfig,
  gestionTasksBackendActive,
  gestionUploadFile,
  isGestionLoggedIn,
} from "../../services/gestion";
import type { Project } from "../../types/models";
import { ProjectSelect } from "../projects/ProjectSelect";

type PendingFile = {
  name: string;
  mime: string;
  base64: string;
  size: number;
};

type Props = {
  /** Indication UI — la session est toujours re-vérifiée au dépôt. */
  loggedIn: boolean;
  onNeedLogin: () => void;
  onSessionResolved?: (loggedIn: boolean) => void;
};

async function fileToPending(file: File): Promise<PendingFile> {
  const buf = await file.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return {
    name: file.name,
    mime: file.type || "application/octet-stream",
    base64: btoa(binary),
    size: file.size,
  };
}

async function sessionIsActive(): Promise<boolean> {
  try {
    if (await gestionTasksBackendActive()) return true;
  } catch {
    /* fallback config */
  }
  try {
    return isGestionLoggedIn(await gestionGetConfig());
  } catch {
    return false;
  }
}

export function DropZone({ loggedIn, onNeedLogin, onSessionResolved }: Props) {
  const [dragging, setDragging] = useState(false);
  const [pending, setPending] = useState<PendingFile | null>(null);
  const [dest, setDest] = useState<"bureau" | "project">("bureau");
  const [projectId, setProjectId] = useState<string | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [needsLogin, setNeedsLogin] = useState(false);

  useEffect(() => {
    void listProjects()
      .then(setProjects)
      .catch(() => setProjects([]));
  }, [pending]);

  const takeFiles = useCallback(
    async (files: FileList | File[]) => {
      const list = Array.from(files);
      if (!list.length) return;
      const file = list[0];
      if (file.size > 25 * 1024 * 1024) {
        setError("Fichier trop volumineux (max 25 Mo).");
        return;
      }
      setError(null);
      setMsg(null);
      setBusy(true);
      try {
        // Ne pas se fier uniquement au state React (session Gestion peut arriver via bridge).
        const active = loggedIn || (await sessionIsActive());
        onSessionResolved?.(active);
        if (!active) {
          setNeedsLogin(true);
          setError("Connecte-toi à Gestion pour déposer un fichier.");
          return;
        }
        setNeedsLogin(false);
        setPending(await fileToPending(file));
        setDest("bureau");
        setProjectId(null);
      } catch {
        setError("Lecture du fichier impossible.");
      } finally {
        setBusy(false);
      }
    },
    [loggedIn, onSessionResolved],
  );

  async function confirmUpload() {
    if (!pending || busy) return;
    if (dest === "project" && !projectId) {
      setError("Choisis un projet.");
      return;
    }
    setBusy(true);
    setError(null);
    setMsg(null);
    try {
      const report = await gestionUploadFile({
        filename: pending.name,
        mime: pending.mime,
        dataBase64: pending.base64,
        visibility: dest === "bureau" ? "personal" : "team",
        projectId: dest === "project" ? projectId : null,
      });
      setMsg(
        dest === "bureau"
          ? `Déposé dans le bureau — ${report.filename}`
          : `Déposé dans le projet — ${report.filename}`,
      );
      setPending(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Envoi impossible");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="drop-zone-block" aria-label="Déposer un fichier">
      {!pending ? (
        <div
          className={`drop-zone ${dragging ? "is-dragging" : ""} ${busy ? "is-busy" : ""}`}
          onDragEnter={(e) => {
            e.preventDefault();
            setDragging(true);
          }}
          onDragOver={(e) => {
            e.preventDefault();
            setDragging(true);
          }}
          onDragLeave={(e) => {
            e.preventDefault();
            setDragging(false);
          }}
          onDrop={(e) => {
            e.preventDefault();
            setDragging(false);
            void takeFiles(e.dataTransfer.files);
          }}
        >
          <p className="drop-zone-title">Déposer un fichier</p>
          <p className="drop-zone-hint">
            Bureau Gestion ou projet — PDF, images, docs…
          </p>
          <label className="drop-zone-browse">
            Choisir
            <input
              type="file"
              hidden
              disabled={busy}
              onChange={(e) => {
                if (e.target.files) void takeFiles(e.target.files);
                e.target.value = "";
              }}
            />
          </label>
        </div>
      ) : (
        <div className="drop-sheet">
          <p className="drop-sheet-name" title={pending.name}>
            {pending.name}
          </p>
          <div className="drop-sheet-dest" role="group" aria-label="Destination">
            <button
              type="button"
              className={`drop-dest-btn ${dest === "bureau" ? "is-active" : ""}`}
              disabled={busy}
              onClick={() => setDest("bureau")}
            >
              Bureau
            </button>
            <button
              type="button"
              className={`drop-dest-btn ${dest === "project" ? "is-active" : ""}`}
              disabled={busy}
              onClick={() => setDest("project")}
            >
              Projet
            </button>
          </div>
          {dest === "project" && (
            <ProjectSelect
              projects={projects}
              value={projectId}
              disabled={busy}
              onChange={setProjectId}
              ariaLabel="Projet destination"
            />
          )}
          <div className="drop-sheet-actions">
            <button
              type="button"
              className="panel-action-btn is-primary"
              disabled={busy || (dest === "project" && !projectId)}
              onClick={() => void confirmUpload()}
            >
              {busy ? "Envoi…" : "Déposer"}
            </button>
            <button
              type="button"
              className="panel-link-btn"
              disabled={busy}
              onClick={() => {
                setPending(null);
                setError(null);
              }}
            >
              Annuler
            </button>
          </div>
        </div>
      )}
      {msg && <p className="panel-muted drop-zone-msg">{msg}</p>}
      {error && <p className="panel-error drop-zone-msg">{error}</p>}
      {needsLogin && (
        <button
          type="button"
          className="panel-link-btn drop-zone-login"
          onClick={onNeedLogin}
        >
          Ouvrir la connexion Gestion
        </button>
      )}
    </section>
  );
}
