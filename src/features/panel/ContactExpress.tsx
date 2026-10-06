/**
 * Feuille courte — crée un prospect CRM Gestion (nom, organisme, tél, email).
 * « Analyser l’écran » : OCR local Windows, champs éditables, rien sans Créer.
 */

import { useState } from "react";
import { analyzeScreenCrm } from "../../services/capture";
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
  const [phone, setPhone] = useState("");
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hint, setHint] = useState<string | null>(null);
  const [createdId, setCreatedId] = useState<string | null>(null);

  async function submit() {
    const trimmed = name.trim();
    if (!trimmed || busy || analyzing) return;
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
        phone: phone.trim() || null,
        email: email.trim() || null,
      });
      setCreatedId(report.prospectId);
    } catch (err) {
      setError(formatInvokeError(err, "Création impossible"));
    } finally {
      setBusy(false);
    }
  }

  async function analyzeScreen() {
    if (busy || analyzing) return;
    setAnalyzing(true);
    setError(null);
    setHint(null);
    try {
      const result = await analyzeScreenCrm();
      if (result.name.trim()) setName(result.name.trim());
      if (result.organisme.trim()) setOrganisme(result.organisme.trim());
      if (result.phone.trim()) setPhone(result.phone.trim());
      if (result.email.trim()) setEmail(result.email.trim());
      const filled = Boolean(
        result.name.trim() ||
          result.organisme.trim() ||
          result.phone.trim() ||
          result.email.trim(),
      );
      if (!filled) {
        setError("Rien d’exploitable. Clique dans Gmail à gauche, puis réessaie.");
      } else {
        setHint("Relis avant de créer — Gmail reste visible à gauche.");
      }
    } catch (err) {
      setError(formatInvokeError(err, "Analyse impossible"));
    } finally {
      setAnalyzing(false);
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
      <button
        type="button"
        className="panel-action-btn contact-analyze-btn"
        disabled={busy || analyzing}
        onClick={() => void analyzeScreen()}
      >
        {analyzing ? "Analyse…" : "Analyser l’écran"}
      </button>
      {analyzing && (
        <p className="panel-muted">Lecture de l’écran (hors panneau MIND)…</p>
      )}
      {hint && !analyzing && <p className="panel-muted">{hint}</p>}
      <label className="panel-field">
        <span>Nom</span>
        <input
          className="panel-input"
          type="text"
          value={name}
          disabled={busy || analyzing}
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
          disabled={busy || analyzing}
          placeholder="Optionnel"
          autoComplete="off"
          onChange={(e) => setOrganisme(e.target.value)}
        />
      </label>
      <label className="panel-field">
        <span>Tél</span>
        <input
          className="panel-input"
          type="tel"
          value={phone}
          disabled={busy || analyzing}
          placeholder="Optionnel"
          autoComplete="off"
          onChange={(e) => setPhone(e.target.value)}
        />
      </label>
      <label className="panel-field">
        <span>Email</span>
        <input
          className="panel-input"
          type="email"
          value={email}
          disabled={busy || analyzing}
          placeholder="Optionnel"
          autoComplete="off"
          onChange={(e) => setEmail(e.target.value)}
        />
      </label>
      <div className="contact-express-actions">
        <button
          type="button"
          className="panel-action-btn is-primary"
          disabled={busy || analyzing || !name.trim()}
          onClick={() => void submit()}
        >
          {busy ? "Création…" : "Créer"}
        </button>
        <button
          type="button"
          className="panel-link-btn"
          disabled={busy || analyzing}
          onClick={onClose}
        >
          Annuler
        </button>
      </div>
      {error && <p className="panel-error">{error}</p>}
    </div>
  );
}
