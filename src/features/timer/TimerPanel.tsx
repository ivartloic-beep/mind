/**
 * Minuteur compact — durée/son/boutons sur une ligne ;
 * fin : panneau ouvert + écran grisé OK / Reporter.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { panelSetOpen } from "../../services/panel";
import "./timer.css";

type Status = "idle" | "running" | "paused" | "finished";

const STORAGE_KEY = "ma-tete.timer.prefs";

const SNOOZE_PRESETS = [
  { label: "5 min", minutes: 5 },
  { label: "15 min", minutes: 15 },
  { label: "30 min", minutes: 30 },
  { label: "1 h", minutes: 60 },
] as const;

type Prefs = {
  minutes: number;
  soundEnabled: boolean;
};

function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return { minutes: 10, soundEnabled: true };
    }
    const parsed = JSON.parse(raw) as Partial<Prefs> & {
      workMinutes?: number;
    };
    const minutes = clampMinutes(parsed.minutes ?? parsed.workMinutes ?? 10);
    return {
      minutes,
      soundEnabled: parsed.soundEnabled ?? true,
    };
  } catch {
    return { minutes: 10, soundEnabled: true };
  }
}

function savePrefs(prefs: Prefs) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
}

function clampMinutes(value: number): number {
  if (!Number.isFinite(value)) return 10;
  return Math.min(180, Math.max(1, Math.round(value)));
}

function formatMmSs(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${String(m).padStart(2, "0")}:${String(r).padStart(2, "0")}`;
}

function playBeep() {
  try {
    const Ctx =
      window.AudioContext ||
      (window as unknown as { webkitAudioContext: typeof AudioContext })
        .webkitAudioContext;
    if (!Ctx) return;
    const ctx = new Ctx();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = "sine";
    osc.frequency.value = 880;
    gain.gain.value = 0.04;
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.start();
    gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.35);
    osc.stop(ctx.currentTime + 0.4);
    window.setTimeout(() => void ctx.close(), 500);
  } catch {
    /* son optionnel */
  }
}

async function notifyDone() {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === "granted";
    }
    if (!granted) return;
    await sendNotification({
      title: "Ma Tête — Minuteur",
      body: "Temps écoulé.",
    });
  } catch {
    /* hors Tauri */
  }
}

async function revealPanel() {
  try {
    await panelSetOpen(true);
  } catch {
    /* hors Tauri */
  }
}

export function TimerPanel() {
  const initial = useMemo(() => loadPrefs(), []);
  const [minutes, setMinutes] = useState(initial.minutes);
  const [soundEnabled, setSoundEnabled] = useState(initial.soundEnabled);
  const [status, setStatus] = useState<Status>("idle");
  const [remaining, setRemaining] = useState(initial.minutes * 60);
  const [snoozeOpen, setSnoozeOpen] = useState(false);
  const [customOpen, setCustomOpen] = useState(false);
  const [customMinutes, setCustomMinutes] = useState(10);
  const soundRef = useRef(soundEnabled);
  const statusRef = useRef(status);
  const remainingRef = useRef(remaining);
  soundRef.current = soundEnabled;
  statusRef.current = status;
  remainingRef.current = remaining;

  useEffect(() => {
    savePrefs({ minutes, soundEnabled });
  }, [minutes, soundEnabled]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen<string>("timer-command", (event) => {
      const cmd = event.payload;
      if (cmd === "pause") {
        if (statusRef.current === "running") setStatus("paused");
        return;
      }
      if (cmd !== "start") return;
      const prev = statusRef.current;
      if (prev === "running") return;
      if (prev === "paused") {
        setStatus("running");
        return;
      }
      if (prev === "finished" || remainingRef.current <= 0) {
        setRemaining(minutes * 60);
      }
      setSnoozeOpen(false);
      setCustomOpen(false);
      setStatus("running");
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [minutes]);

  useEffect(() => {
    if (status !== "running") return;
    const id = window.setInterval(() => {
      setRemaining((prev) => {
        if (prev <= 1) {
          window.clearInterval(id);
          setStatus("finished");
          setSnoozeOpen(false);
          setCustomOpen(false);
          void revealPanel();
          void notifyDone();
          if (soundRef.current) playBeep();
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => window.clearInterval(id);
  }, [status]);

  useEffect(() => {
    if (status === "finished") {
      void revealPanel();
    }
  }, [status]);

  function applyMinutes(value: number) {
    const mins = clampMinutes(value);
    setMinutes(mins);
    if (status === "idle") {
      setRemaining(mins * 60);
    }
  }

  function start() {
    if (status === "finished" || remaining <= 0) {
      setRemaining(minutes * 60);
    }
    setSnoozeOpen(false);
    setCustomOpen(false);
    setStatus("running");
  }

  function pause() {
    if (status === "running") setStatus("paused");
  }

  function resume() {
    if (status === "paused") setStatus("running");
  }

  function stop() {
    setStatus("idle");
    setSnoozeOpen(false);
    setCustomOpen(false);
    setRemaining(minutes * 60);
  }

  function acknowledgeFinished() {
    setStatus("idle");
    setSnoozeOpen(false);
    setCustomOpen(false);
    setRemaining(minutes * 60);
  }

  function snoozeMinutes(mins: number) {
    const m = clampMinutes(mins);
    setRemaining(m * 60);
    setSnoozeOpen(false);
    setCustomOpen(false);
    setStatus("running");
  }

  return (
    <>
      <div className="timer-panel">
        <div className="timer-display" aria-live="polite">
          <span className="timer-clock">⏱ {formatMmSs(remaining)}</span>
          <span className="timer-mode">
            {status === "running"
              ? "En cours"
              : status === "paused"
                ? "En pause"
                : status === "finished"
                  ? "Terminé"
                  : "Prêt"}
          </span>
        </div>

        <div className="timer-toolbar">
          {status === "running" ? (
            <button type="button" className="timer-btn" onClick={pause}>
              Pause
            </button>
          ) : status === "paused" ? (
            <button
              type="button"
              className="timer-btn is-primary"
              onClick={resume}
            >
              Reprendre
            </button>
          ) : (
            <button
              type="button"
              className="timer-btn is-primary"
              onClick={start}
              disabled={status === "finished"}
            >
              Démarrer
            </button>
          )}
          <button
            type="button"
            className="timer-btn"
            onClick={stop}
            disabled={status === "idle" || status === "finished"}
          >
            Arrêter
          </button>

          <label className="timer-duration">
            <span>Durée</span>
            <input
              type="number"
              min={1}
              max={180}
              value={minutes}
              disabled={status === "running" || status === "paused"}
              onChange={(e) => applyMinutes(Number(e.target.value))}
            />
            <span>min</span>
          </label>

          <label className="timer-sound" title="Son à la fin">
            <input
              type="checkbox"
              checked={soundEnabled}
              onChange={(e) => setSoundEnabled(e.target.checked)}
            />
            <span>Son</span>
          </label>
        </div>
      </div>

      {status === "finished" && (
        <div className="timer-finish-overlay" role="alertdialog" aria-modal="true">
          <div className="timer-finish-card">
            <p className="timer-finish-title">Fin du minuteur</p>
            {!snoozeOpen ? (
              <div className="timer-finish-actions">
                <button
                  type="button"
                  className="timer-btn is-primary"
                  onClick={acknowledgeFinished}
                >
                  OK
                </button>
                <button
                  type="button"
                  className="timer-btn"
                  onClick={() => {
                    setSnoozeOpen(true);
                    setCustomOpen(false);
                  }}
                >
                  Reporter
                </button>
              </div>
            ) : (
              <div className="timer-snooze">
                <p className="timer-snooze-label">Reporter de</p>
                <div className="timer-snooze-presets">
                  {SNOOZE_PRESETS.map((p) => (
                    <button
                      key={p.minutes}
                      type="button"
                      className="timer-btn"
                      onClick={() => snoozeMinutes(p.minutes)}
                    >
                      {p.label}
                    </button>
                  ))}
                  <button
                    type="button"
                    className={`timer-btn ${customOpen ? "is-primary" : ""}`}
                    onClick={() => setCustomOpen((v) => !v)}
                  >
                    Perso…
                  </button>
                </div>
                {customOpen && (
                  <div className="timer-snooze-custom">
                    <input
                      type="number"
                      min={1}
                      max={180}
                      value={customMinutes}
                      onChange={(e) =>
                        setCustomMinutes(clampMinutes(Number(e.target.value)))
                      }
                    />
                    <span>min</span>
                    <button
                      type="button"
                      className="timer-btn is-primary"
                      onClick={() => snoozeMinutes(customMinutes)}
                    >
                      OK
                    </button>
                  </div>
                )}
                <button
                  type="button"
                  className="timer-link"
                  onClick={() => {
                    setSnoozeOpen(false);
                    setCustomOpen(false);
                  }}
                >
                  Retour
                </button>
              </div>
            )}
          </div>
        </div>
      )}
    </>
  );
}
