/**
 * Panneau compact — actions + 5 dernières tâches / notes + minuteur.
 */

import { useCallback, useEffect, useState, useTransition } from "react";
import {
  autostartIsEnabled,
  autostartSetEnabled,
  clearTaskReminder,
  deleteNote,
  deleteTask,
  dismissReminder,
  listNotes,
  listProjects,
  listTasks,
  openTaskFromReminder,
  setTaskDone,
  setTaskReminder,
  snoozeReminder,
  type SnoozeKind,
} from "../../services/api";
import { captureShow } from "../../services/capture";
import {
  listenDataChanged,
  listenPanelFocusSettings,
  listenPanelStateChanged,
  listenReminderDue,
  listenReminderOpenTask,
  type ReminderDuePayload,
} from "../../services/events";
import {
  gestionGetConfig,
  gestionLogin,
  gestionLogout,
  gestionMigrateLocalTasks,
  gestionSetConfig,
  gestionShow,
  isGestionLoggedIn,
} from "../../services/gestion";
import { libraryShow } from "../../services/library";
import {
  panelGetState,
  panelSetAlwaysOnTop,
  panelSetOpen,
} from "../../services/panel";
import { createScratchPostit } from "../../services/postit";
import {
  listShortcuts,
  type ShortcutInfo,
} from "../../services/shortcuts";
import {
  syncGetConfig,
  syncNow,
  syncSetConfig,
  syncTest,
  type SyncConfigView,
} from "../../services/sync";
import {
  setPanelAlwaysOnTop as storeSetAot,
  setPanelOpen as storeSetOpen,
} from "../../stores/ui-store";
import type { Note, Task } from "../../types/models";
import { isTaskOpen } from "../../types/models";
import { ReminderDueBanner } from "../reminders/ReminderDueBanner";
import { TimerPanel } from "../timer/TimerPanel";
import "./panel.css";

type LoadState = "loading" | "ready" | "error";

function sortRecentTasks(rows: Task[]): Task[] {
  return [...rows]
    .filter((t) => isTaskOpen(t))
    .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
    .slice(0, 5);
}

function sortRecentNotes(rows: Note[]): Note[] {
  return [...rows]
    .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
    .slice(0, 5);
}

function preview(text: string): string {
  const one = text.replace(/\s+/g, " ").trim();
  return one.length > 48 ? `${one.slice(0, 48)}…` : one;
}

export function PanelApp() {
  const [open, setOpen] = useState(true);
  const [alwaysOnTop, setAlwaysOnTop] = useState(false);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [notes, setNotes] = useState<Note[]>([]);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [dueReminder, setDueReminder] = useState<ReminderDuePayload | null>(
    null,
  );
  const [reminderBusy, setReminderBusy] = useState(false);
  const [autostart, setAutostart] = useState(true);
  const [autostartBusy, setAutostartBusy] = useState(false);
  const [syncCfg, setSyncCfg] = useState<SyncConfigView | null>(null);
  const [syncToken, setSyncToken] = useState("");
  const [syncBusy, setSyncBusy] = useState(false);
  const [syncMsg, setSyncMsg] = useState<string | null>(null);
  const [shortcuts, setShortcuts] = useState<ShortcutInfo[]>([]);
  const [gestionApiUrl, setGestionApiUrl] = useState("");
  const [gestionUser, setGestionUser] = useState<string | null>(null);
  const [gestionLoggedIn, setGestionLoggedIn] = useState(false);
  const [gestionMigratedAt, setGestionMigratedAt] = useState<string | null>(
    null,
  );
  const [gestionLastError, setGestionLastError] = useState<string | null>(null);
  const [gestionUserName, setGestionUserName] = useState("");
  const [gestionPassword, setGestionPassword] = useState("");
  const [gestionBusy, setGestionBusy] = useState(false);
  const [gestionMsg, setGestionMsg] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [cloudEdit, setCloudEdit] = useState(false);
  const [, startTransition] = useTransition();

  const refresh = useCallback(async () => {
    const [taskRows, noteRows] = await Promise.all([
      listTasks(),
      listNotes(),
    ]);
    setTasks(sortRecentTasks(taskRows));
    setNotes(sortRecentNotes(noteRows));
    setLoadState("ready");
    setLoadError(null);
    try {
      const g = await gestionGetConfig();
      setGestionLastError(g.lastError || null);
      setGestionLoggedIn(isGestionLoggedIn(g));
      setGestionUser(g.userName || null);
    } catch {
      /* ignore */
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      try {
        const state = await panelGetState();
        if (cancelled) return;
        setOpen(state.open);
        setAlwaysOnTop(state.alwaysOnTop);
        storeSetOpen(state.open);
        storeSetAot(state.alwaysOnTop);
      } catch {
        /* hors Tauri */
      }

      try {
        const enabled = await autostartIsEnabled();
        if (!cancelled) setAutostart(enabled);
        const sync = await syncGetConfig();
        if (!cancelled && sync) {
          setSyncCfg(sync);
          setSyncToken(sync.token);
        }
        const keys = await listShortcuts();
        if (!cancelled) setShortcuts(keys);
        const g = await gestionGetConfig();
        if (!cancelled) {
          setGestionApiUrl(g.apiUrl || "https://gestion.louetline.fr/api");
          setGestionLoggedIn(isGestionLoggedIn(g));
          setGestionUser(g.userName || null);
          setGestionMigratedAt(g.tasksMigratedAt || null);
          setGestionLastError(g.lastError || null);
        }
      } catch {
        /* ignore */
      }

      try {
        await listProjects();
        await refresh();
      } catch (err) {
        if (cancelled) return;
        setLoadState("error");
        setLoadError(err instanceof Error ? err.message : "Chargement impossible");
      }
    }

    void bootstrap();

    const unlistens: Array<() => void> = [];
    void listenDataChanged((payload) => {
      if (
        payload.entity === "task" ||
        payload.entity === "note" ||
        payload.entity === "reminder" ||
        payload.entity === "project" ||
        payload.entity === "sync"
      ) {
        startTransition(() => {
          void refresh().catch(() => undefined);
        });
      }
    }).then((fn) => unlistens.push(fn));

    void listenReminderDue((payload) => {
      setDueReminder(payload);
      setOpen(true);
      storeSetOpen(true);
      void panelSetOpen(true).catch(() => undefined);
    }).then((fn) => unlistens.push(fn));

    void listenReminderOpenTask((payload) => {
      setOpen(true);
      storeSetOpen(true);
      window.setTimeout(() => {
        document
          .querySelector(`[data-task-id="${payload.taskId}"]`)
          ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      }, 80);
    }).then((fn) => unlistens.push(fn));

    void listenPanelStateChanged((payload) => {
      setOpen(payload.open);
      storeSetOpen(payload.open);
      setAlwaysOnTop(payload.alwaysOnTop);
      storeSetAot(payload.alwaysOnTop);
    }).then((fn) => unlistens.push(fn));

    void listenPanelFocusSettings(() => {
      setOpen(true);
      storeSetOpen(true);
      setSettingsOpen(true);
    }).then((fn) => unlistens.push(fn));

    return () => {
      cancelled = true;
      for (const fn of unlistens) fn();
    };
  }, [refresh]);

  async function toggleOpen() {
    const next = !open;
    try {
      if (open) {
        setOpen(false);
        storeSetOpen(false);
        await new Promise((r) => window.setTimeout(r, 170));
        const state = await panelSetOpen(false);
        setOpen(state.open);
        storeSetOpen(state.open);
      } else {
        const state = await panelSetOpen(true);
        setOpen(state.open);
        storeSetOpen(state.open);
      }
    } catch {
      setOpen(next);
      storeSetOpen(next);
    }
  }

  async function toggleAlwaysOnTop() {
    const next = !alwaysOnTop;
    setAlwaysOnTop(next);
    storeSetAot(next);
    try {
      const state = await panelSetAlwaysOnTop(next);
      setAlwaysOnTop(state.alwaysOnTop);
      storeSetAot(state.alwaysOnTop);
    } catch {
      setAlwaysOnTop(!next);
      storeSetAot(!next);
    }
  }

  async function toggleAutostart() {
    const next = !autostart;
    setAutostartBusy(true);
    setAutostart(next);
    try {
      setAutostart(await autostartSetEnabled(next));
    } catch {
      setAutostart(!next);
    } finally {
      setAutostartBusy(false);
    }
  }

  async function saveSync(partial?: { enabled?: boolean; token?: string }) {
    if (!syncCfg) return;
    setSyncBusy(true);
    setSyncMsg(null);
    try {
      const next = await syncSetConfig({
        enabled: partial?.enabled ?? syncCfg.enabled,
        baseUrl: syncCfg.baseUrl || "https://mind.louetline.fr",
        token: partial?.token ?? syncToken,
      });
      setSyncCfg(next);
      setSyncToken(next.token);
      setSyncMsg("Enregistré");
    } catch (err) {
      setSyncMsg(err instanceof Error ? err.message : "Erreur sync");
    } finally {
      setSyncBusy(false);
    }
  }

  async function handleSyncTest() {
    setSyncBusy(true);
    setSyncMsg(null);
    try {
      await syncSetConfig({
        enabled: syncCfg?.enabled ?? false,
        baseUrl: syncCfg?.baseUrl || "https://mind.louetline.fr",
        token: syncToken,
      });
      const service = await syncTest();
      setSyncMsg(`OK — ${service}`);
    } catch (err) {
      setSyncMsg(err instanceof Error ? err.message : "Test échoué");
    } finally {
      setSyncBusy(false);
    }
  }

  async function handleSyncNow() {
    setSyncBusy(true);
    setSyncMsg(null);
    try {
      await syncSetConfig({
        enabled: true,
        baseUrl: syncCfg?.baseUrl || "https://mind.louetline.fr",
        token: syncToken,
      });
      const report = await syncNow();
      const cfg = await syncGetConfig();
      if (cfg) setSyncCfg(cfg);
      setSyncMsg(`Sync : ${report.pushed} envoyés, ${report.pulled} reçus`);
      await refresh();
    } catch (err) {
      setSyncMsg(err instanceof Error ? err.message : "Sync échouée");
      const cfg = await syncGetConfig();
      if (cfg) setSyncCfg(cfg);
    } finally {
      setSyncBusy(false);
    }
  }

  async function toggleTaskDone(task: Task) {
    setPendingId(task.id);
    try {
      await setTaskDone(task.id, isTaskOpen(task));
      await refresh();
    } finally {
      setPendingId(null);
    }
  }

  async function quickRemind(task: Task) {
    const fireAt = new Date(Date.now() + 10 * 60_000).toISOString();
    setPendingId(task.id);
    try {
      if (task.reminder) await clearTaskReminder(task.id);
      else await setTaskReminder(task.id, fireAt);
      await refresh();
    } finally {
      setPendingId(null);
    }
  }

  async function removeTask(task: Task) {
    const ok = window.confirm(`Supprimer la tâche « ${task.title} » ?`);
    if (!ok) return;
    setPendingId(task.id);
    try {
      await deleteTask(task.id);
      await refresh();
    } finally {
      setPendingId(null);
    }
  }

  async function removeNote(note: Note) {
    const label = note.title?.trim() || preview(note.body);
    const ok = window.confirm(`Supprimer la note « ${label} » ?`);
    if (!ok) return;
    setPendingId(note.id);
    try {
      await deleteNote(note.id);
      await refresh();
    } finally {
      setPendingId(null);
    }
  }

  async function handleOpenDue() {
    if (!dueReminder) return;
    setReminderBusy(true);
    try {
      await openTaskFromReminder(dueReminder.taskId);
      await libraryShow();
    } finally {
      setReminderBusy(false);
    }
  }

  async function handleSnoozeDue(kind: SnoozeKind) {
    if (!dueReminder) return;
    setReminderBusy(true);
    try {
      await snoozeReminder(dueReminder.reminderId, kind);
      setDueReminder(null);
      await refresh();
    } catch {
      /* keep */
    } finally {
      setReminderBusy(false);
    }
  }

  async function handleDismissDue() {
    if (!dueReminder) return;
    setReminderBusy(true);
    try {
      await dismissReminder(dueReminder.reminderId);
      setDueReminder(null);
      await refresh();
    } catch {
      /* keep */
    } finally {
      setReminderBusy(false);
    }
  }

  return (
    <div className="panel-root">
      <button
        type="button"
        className="panel-handle"
        onClick={() => void toggleOpen()}
        aria-label="Fermer le panneau"
        title="Fermer"
      >
        <span className="panel-handle-arrow" aria-hidden="true">
          ›
        </span>
      </button>

      <div className="panel-body">
        {settingsOpen ? (
          <section
            className="panel-settings-view"
            id="panel-settings"
            aria-label="Paramètres"
          >
            <header className="panel-settings-head">
              <button
                type="button"
                className="panel-settings-back"
                onClick={() => {
                  setSettingsOpen(false);
                  setCloudEdit(false);
                  setSyncMsg(null);
                }}
              >
                ← Retour
              </button>
              <h2>Paramètres</h2>
            </header>

            <div className="panel-settings-group">
              <label className="panel-toggle">
                <input
                  type="checkbox"
                  checked={alwaysOnTop}
                  onChange={() => void toggleAlwaysOnTop()}
                />
                <span>Toujours au-dessus</span>
              </label>
              <label className="panel-toggle">
                <input
                  type="checkbox"
                  checked={autostart}
                  disabled={autostartBusy}
                  onChange={() => void toggleAutostart()}
                />
                <span>Démarrer avec Windows</span>
              </label>
            </div>

            <div className="panel-settings-group">
              <div className="panel-settings-group-head">
                <h3>Cloud</h3>
                {syncCfg?.enabled && syncCfg.hasToken && (
                  <span className="panel-settings-badge">actif</span>
                )}
              </div>
              <label className="panel-toggle">
                <input
                  type="checkbox"
                  checked={syncCfg?.enabled ?? false}
                  disabled={syncBusy || !syncCfg || !syncToken.trim()}
                  onChange={() =>
                    void saveSync({ enabled: !(syncCfg?.enabled ?? false) })
                  }
                />
                <span>Synchronisation</span>
              </label>
              <div className="panel-actions">
                <button
                  type="button"
                  className="panel-action-btn is-primary"
                  disabled={syncBusy || !syncToken.trim()}
                  onClick={() => void handleSyncNow()}
                >
                  Synchroniser
                </button>
                <button
                  type="button"
                  className="panel-link-btn"
                  onClick={() => setCloudEdit((v) => !v)}
                >
                  {cloudEdit ? "Masquer" : "Configurer"}
                </button>
              </div>
              {cloudEdit && (
                <div className="panel-settings-cloud">
                  <label className="panel-field">
                    <span>Token API</span>
                    <input
                      type="password"
                      className="panel-input"
                      value={syncToken}
                      disabled={syncBusy}
                      placeholder="Bearer token"
                      autoComplete="off"
                      onChange={(e) => setSyncToken(e.target.value)}
                      onBlur={() => void saveSync()}
                    />
                  </label>
                  <button
                    type="button"
                    className="panel-action-btn"
                    disabled={syncBusy || !syncToken.trim()}
                    onClick={() => void handleSyncTest()}
                  >
                    Tester la connexion
                  </button>
                </div>
              )}
              {syncMsg && <p className="panel-muted">{syncMsg}</p>}
              {syncCfg?.lastSyncAt && !cloudEdit && (
                <p className="panel-muted">
                  Dernière sync :{" "}
                  {new Date(syncCfg.lastSyncAt).toLocaleString()}
                </p>
              )}
              {syncCfg?.lastError && (
                <p className="panel-error">{syncCfg.lastError}</p>
              )}
            </div>

            <div className="panel-settings-group">
              <div className="panel-settings-group-head">
                <h3>Gestion (app complète)</h3>
              </div>
              <p className="panel-muted">
                URL API + session : les tâches MIND utilisent alors personal_tasks.
              </p>
              <label className="panel-field">
                <span>API Gestion</span>
                <input
                  type="url"
                  className="panel-input"
                  value={gestionApiUrl}
                  disabled={gestionBusy}
                  placeholder="https://gestion.louetline.fr/api"
                  autoComplete="off"
                  onChange={(e) => setGestionApiUrl(e.target.value)}
                />
              </label>
              <div className="panel-actions">
                <button
                  type="button"
                  className="panel-action-btn is-primary"
                  disabled={gestionBusy || !gestionApiUrl.trim()}
                  onClick={() => {
                    void (async () => {
                      setGestionBusy(true);
                      setGestionMsg(null);
                      try {
                        const cfg = await gestionSetConfig(gestionApiUrl.trim());
                        setGestionApiUrl(cfg.apiUrl);
                        setGestionLoggedIn(isGestionLoggedIn(cfg));
                        setGestionMsg("API Gestion enregistrée");
                      } catch (err) {
                        setGestionMsg(
                          err instanceof Error
                            ? err.message
                            : "Enregistrement impossible",
                        );
                      } finally {
                        setGestionBusy(false);
                      }
                    })();
                  }}
                >
                  Enregistrer
                </button>
                <button
                  type="button"
                  className="panel-action-btn"
                  disabled={gestionBusy}
                  onClick={() => void gestionShow()}
                >
                  Ouvrir Gestion
                </button>
              </div>
              {gestionLoggedIn ? (
                <>
                  <p className="panel-muted">
                    Connecté{gestionUser ? ` — ${gestionUser}` : ""}. Tâches =
                    personal_tasks.
                  </p>
                  <div className="panel-actions">
                    <button
                      type="button"
                      className="panel-action-btn"
                      disabled={gestionBusy}
                      onClick={() => {
                        void (async () => {
                          setGestionBusy(true);
                          setGestionMsg(null);
                          try {
                            const report = await gestionMigrateLocalTasks();
                            setGestionMigratedAt(report.migratedAt || null);
                            setGestionMsg(
                              `Import : ${report.created} créées, ${report.skipped} déjà présentes` +
                                (report.errors.length
                                  ? ` (${report.errors.length} erreurs)`
                                  : ""),
                            );
                            await refresh();
                          } catch (err) {
                            setGestionMsg(
                              err instanceof Error
                                ? err.message
                                : "Migration impossible",
                            );
                          } finally {
                            setGestionBusy(false);
                          }
                        })();
                      }}
                    >
                      Importer tâches locales
                    </button>
                    <button
                      type="button"
                      className="panel-action-btn"
                      disabled={gestionBusy}
                      onClick={() => {
                        void (async () => {
                          setGestionBusy(true);
                          try {
                            const cfg = await gestionLogout();
                            setGestionLoggedIn(false);
                            setGestionUser(null);
                            setGestionMigratedAt(cfg.tasksMigratedAt || null);
                            setGestionMsg("Session Gestion fermée");
                            await refresh();
                          } catch (err) {
                            setGestionMsg(
                              err instanceof Error
                                ? err.message
                                : "Déconnexion impossible",
                            );
                          } finally {
                            setGestionBusy(false);
                          }
                        })();
                      }}
                    >
                      Déconnexion
                    </button>
                  </div>
                  {gestionMigratedAt && (
                    <p className="panel-muted">
                      Dernier import :{" "}
                      {new Date(gestionMigratedAt).toLocaleString()}
                    </p>
                  )}
                  {gestionLastError && (
                    <p className="panel-error">{gestionLastError}</p>
                  )}
                </>
              ) : (
                <>
                  <label className="panel-field">
                    <span>Identifiant</span>
                    <input
                      type="text"
                      className="panel-input"
                      value={gestionUserName}
                      disabled={gestionBusy}
                      autoComplete="username"
                      onChange={(e) => setGestionUserName(e.target.value)}
                    />
                  </label>
                  <label className="panel-field">
                    <span>Mot de passe</span>
                    <input
                      type="password"
                      className="panel-input"
                      value={gestionPassword}
                      disabled={gestionBusy}
                      autoComplete="current-password"
                      onChange={(e) => setGestionPassword(e.target.value)}
                    />
                  </label>
                  <div className="panel-actions">
                    <button
                      type="button"
                      className="panel-action-btn is-primary"
                      disabled={
                        gestionBusy ||
                        !gestionApiUrl.trim() ||
                        !gestionUserName.trim() ||
                        !gestionPassword
                      }
                      onClick={() => {
                        void (async () => {
                          setGestionBusy(true);
                          setGestionMsg(null);
                          try {
                            if (gestionApiUrl.trim()) {
                              await gestionSetConfig(gestionApiUrl.trim());
                            }
                            const cfg = await gestionLogin(
                              gestionUserName.trim(),
                              gestionPassword,
                            );
                            setGestionApiUrl(cfg.apiUrl);
                            setGestionLoggedIn(isGestionLoggedIn(cfg));
                            setGestionUser(cfg.userName || null);
                            setGestionPassword("");
                            setGestionMsg("Connecté à Gestion");
                            await refresh();
                          } catch (err) {
                            setGestionMsg(
                              err instanceof Error
                                ? err.message
                                : "Connexion impossible",
                            );
                          } finally {
                            setGestionBusy(false);
                          }
                        })();
                      }}
                    >
                      Se connecter
                    </button>
                  </div>
                </>
              )}
              {gestionMsg && <p className="panel-muted">{gestionMsg}</p>}
            </div>

            <div className="panel-settings-group">
              <div className="panel-settings-group-head">
                <h3>Raccourcis clavier</h3>
              </div>
              {shortcuts.length === 0 ? (
                <p className="panel-muted">Aucun raccourci chargé.</p>
              ) : (
                <ul className="panel-shortcuts-list">
                  {shortcuts.map((item) => (
                    <li key={item.id} className="panel-shortcuts-item">
                      <span className="panel-shortcuts-label">{item.label}</span>
                      <kbd className="panel-shortcuts-keys">{item.keys}</kbd>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </section>
        ) : (
          <>
            <header className="panel-header">
              <div className="panel-header-row">
                <h1>MIND</h1>
                <button
                  type="button"
                  className="panel-settings-btn"
                  aria-label="Paramètres"
                  title="Paramètres"
                  onClick={() => setSettingsOpen(true)}
                >
                  ⚙
                </button>
              </div>
              <p>Capturer d&apos;abord, organiser ensuite.</p>
            </header>

            {dueReminder && (
              <ReminderDueBanner
                due={dueReminder}
                busy={reminderBusy}
                onOpen={() => void handleOpenDue()}
                onSnooze={(kind) => void handleSnoozeDue(kind)}
                onDismiss={() => void handleDismissDue()}
              />
            )}

            <section className="panel-section">
              <h2>Actions</h2>
              <div className="panel-actions">
                <button
                  type="button"
                  className="panel-action-btn is-primary"
                  onClick={() => void captureShow()}
                >
                  Capturer
                </button>
                <button
                  type="button"
                  className="panel-action-btn"
                  onClick={() => void createScratchPostit()}
                >
                  Post-it
                </button>
                <button
                  type="button"
                  className="panel-action-btn"
                  onClick={() => void libraryShow()}
                >
                  Bibliothèque
                </button>
                <button
                  type="button"
                  className="panel-action-btn is-primary"
                  onClick={() => void gestionShow()}
                >
                  Gestion
                </button>
              </div>
            </section>

            <section className="panel-section panel-section-grow">
              <div className="panel-section-head">
                <h2>Récent</h2>
                <button
                  type="button"
                  className="panel-link-btn"
                  onClick={() => void libraryShow()}
                >
                  Voir tout
                </button>
              </div>

              {loadState === "loading" && (
                <p className="panel-muted">Chargement…</p>
              )}
              {loadState === "error" && (
                <p className="panel-error">{loadError ?? "Erreur"}</p>
              )}
              {loadState === "ready" && (
                <>
                  <p className="panel-subhead">Tâches</p>
                  {tasks.length === 0 ? (
                    <p className="panel-muted">Aucune tâche active.</p>
                  ) : (
                    <ul className="panel-recent-list">
                      {tasks.map((task) => (
                        <li
                          key={task.id}
                          className="panel-recent-item"
                          data-task-id={task.id}
                        >
                          <button
                            type="button"
                            className="panel-recent-check"
                            disabled={pendingId === task.id}
                            aria-label="Terminer"
                            onClick={() => void toggleTaskDone(task)}
                          >
                            ○
                          </button>
                          <span className="panel-recent-title">{task.title}</span>
                          <button
                            type="button"
                            className={`panel-recent-bell ${task.reminder ? "has-reminder" : ""}`}
                            disabled={pendingId === task.id}
                            title={
                              task.reminder
                                ? "Retirer le rappel"
                                : "Rappel +10 min"
                            }
                            onClick={() => void quickRemind(task)}
                          >
                            🔔
                          </button>
                          <button
                            type="button"
                            className="item-delete-btn"
                            disabled={pendingId === task.id}
                            aria-label="Supprimer la tâche"
                            title="Supprimer"
                            onClick={() => void removeTask(task)}
                          >
                            ×
                          </button>
                        </li>
                      ))}
                    </ul>
                  )}

                  <p className="panel-subhead">Notes</p>
                  {notes.length === 0 ? (
                    <p className="panel-muted">Aucune note.</p>
                  ) : (
                    <ul className="panel-recent-list">
                      {notes.map((note) => (
                        <li key={note.id} className="panel-recent-item">
                          <span className="panel-recent-kind">
                            {note.kind === "idea" ? "idée" : "note"}
                          </span>
                          <span className="panel-recent-title">
                            {note.title?.trim() || preview(note.body)}
                          </span>
                          <button
                            type="button"
                            className="item-delete-btn"
                            disabled={pendingId === note.id}
                            aria-label="Supprimer la note"
                            title="Supprimer"
                            onClick={() => void removeNote(note)}
                          >
                            ×
                          </button>
                        </li>
                      ))}
                    </ul>
                  )}
                </>
              )}
            </section>

            <section className="panel-section">
              <h2>Minuteur</h2>
              <TimerPanel />
            </section>
          </>
        )}
      </div>
    </div>
  );
}
