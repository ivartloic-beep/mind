/**
 * Feuille courte — crée un prospect CRM Gestion en 3 champs.
 */

import { useState } from "react";
import {
  formatInvokeError,
  gestionCreateProspect,
  gestionEnsureSession,
  gestionShowProspect,
  isGestionLoggedIn,
} from "../../services/gestion";

type Props = {
  loggedIn: boolean;
  onNeedLogin: () => void;
  onClose: () => void;
};

export function ContactExpress({ loggedIn, onNeedLogin, onClose }: Props) {
  const [name, setName] = useState("");
  const [organisme, setOrganisme] = useState("");
  const [contact, setContact] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [createdId, setCreatedId] = useState<string | null>(null);

  async function submit() {
    const trimmed = name.trim();
    if (!trimmed || busy) return;
    setBusy(true);
    setError(null);
    try {
      let ok = loggedIn;
      if (!ok) {
        const cfg = await gestionEnsureSession();
        ok = isGestionLoggedIn(cfg);
      }
      if (!ok) {
        onNeedLogin();
        setError("Connecte-toi à Gestion pour créer un contact.");
        return;
      }
      const report = await gestionCreateProspect({
        name: trimmed,
        organisme: organisme.trim() || null,
        contact: contact.trim() || null,
      });
      setCreatedId(report.prospectId);
    } catch (err) {
      setError(formatInvokeError(err, "Création impossible"));
    } finally {
      setBusy(false);
    }
  }

  if (createdId) {
    return (
      <div className="contact-express">
        <p className="panel-muted">Contact créé dans Gestion.</p>
        <div className="contact-express-actions">
          <button
            type="button"
            className="panel-action-btn is-primary"
            onClick={() => void gestionShowProspect(createdId)}
          >
            Ouvrir la fiche
          </button>
          <button type="button" className="panel-link-btn" onClick={onClose}>
            Fermer
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="contact-express">
      <h3 className="contact-express-title">Contact express</h3>
      <label className="panel-field">
        <span>Nom</span>
        <input
          className="panel-input"
          type="text"
          value={name}
          disabled={busy}
          placeholder="Prénom Nom"
          autoComplete="off"
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              void submit();
            }
          }}
        />
      </label>
      <label className="panel-field">
        <span>Organisme</span>
        <input
          className="panel-input"
          type="text"
          value={organisme}
          disabled={busy}
          placeholder="Optionnel"
          autoComplete="off"
          onChange={(e) => setOrganisme(e.target.value)}
        />
      </label>
      <label className="panel-field">
        <span>Tél ou e-mail</span>
        <input
          className="panel-input"
          type="text"
          value={contact}
          disabled={busy}
          placeholder="Optionnel"
          autoComplete="off"
          onChange={(e) => setContact(e.target.value)}
        />
      </label>
      <div className="contact-express-actions">
        <button
          type="button"
          className="panel-action-btn is-primary"
          disabled={busy || !name.trim()}
          onClick={() => void submit()}
        >
          {busy ? "Création…" : "Créer"}
        </button>
        <button
          type="button"
          className="panel-link-btn"
          disabled={busy}
          onClick={onClose}
        >
          Annuler
        </button>
      </div>
      {error && <p className="panel-error">{error}</p>}
    </div>
  );
}
