/**
 * Minuteur panneau — 25/5 par défaut, notif + indication visuelle à zéro.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import "./timer.css";

type Mode = "work" | "break";
type Status = "idle" | "running" | "paused" | "finished";

const STORAGE_KEY = "ma-tete.timer.prefs";

type Prefs = {
  workMinutes: number;
  breakMinutes: number;
  soundEnabled: boolean;
};

function loadPrefs(): Prefs {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return { workMinutes: 25, breakMinutes: 5, soundEnabled: true };
    }
    const parsed = JSON.parse(raw) as Partial<Prefs>;
    return {
      workMinutes: clampMinutes(parsed.workMinutes ?? 25),
      breakMinutes: clampMinutes(parsed.breakMinutes ?? 5),
      soundEnabled: parsed.soundEnabled ?? true,
    };
  } catch {
    return { workMinutes: 25, breakMinutes: 5, soundEnabled: true };
  }
}

function savePrefs(prefs: Prefs) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
}

function clampMinutes(value: number): number {
  if (!Number.isFinite(value)) return 25;
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
    /* son optionnel — skip silencieux */
  }
}

async function notifyDone(mode: Mode) {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === "granted";
    }
    if (!granted) return;
    await sendNotification({
      title: "Ma Tête — Minuteur",
      body:
        mode === "work"
          ? "Session de travail terminée. Pause ?"
          : "Pause terminée. Au travail ?",
    });
  } catch {
    /* notif indisponible hors Tauri / permissions */
  }
}

export function TimerPanel() {
  const initial = useMemo(() => loadPrefs(), []);
  const [workMinutes, setWorkMinutes] = useState(initial.workMinutes);
  const [breakMinutes, setBreakMinutes] = useState(initial.breakMinutes);
  const [soundEnabled, setSoundEnabled] = useState(initial.soundEnabled);
  const [mode, setMode] = useState<Mode>("work");
  const [status, setStatus] = useState<Status>("idle");
  const [remaining, setRemaining] = useState(initial.workMinutes * 60);
  const modeRef = useRef(mode);
  const soundRef = useRef(soundEnabled);
  modeRef.current = mode;
  soundRef.current = soundEnabled;

  useEffect(() => {
    savePrefs({ workMinutes, breakMinutes, soundEnabled });
  }, [workMinutes, breakMinutes, soundEnabled]);

  useEffect(() => {
    if (status !== "running") return;
    const id = window.setInterval(() => {
      setRemaining((prev) => {
        if (prev <= 1) {
          window.clearInterval(id);
          const finishedMode = modeRef.current;
          setStatus("finished");
          void notifyDone(finishedMode);
          if (soundRef.current) playBeep();
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => window.clearInterval(id);
  }, [status]);

  function durationFor(next: Mode): number {
    return (next === "work" ? workMinutes : breakMinutes) * 60;
  }

  function start() {
    if (status === "finished" || status === "idle") {
      // Après fin : enchaîne sur l'autre mode.
      const next =
        status === "finished" ? (mode === "work" ? "break" : "work") : mode;
      setMode(next);
      setRemaining(durationFor(next));
    } else if (remaining <= 0) {
      setRemaining(durationFor(mode));
    }
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
    setRemaining(durationFor(mode));
  }

  function reset() {
    const next =
      status === "finished" ? (mode === "work" ? "break" : "work") : mode;
    setMode(next);
    setStatus("idle");
    setRemaining(durationFor(next));
  }

  function applyWorkMinutes(value: number) {
    const mins = clampMinutes(value);
    setWorkMinutes(mins);
    if (status === "idle" && mode === "work") {
      setRemaining(mins * 60);
    }
  }

  function applyBreakMinutes(value: number) {
    const mins = clampMinutes(value);
    setBreakMinutes(mins);
    if (status === "idle" && mode === "break") {
      setRemaining(mins * 60);
    }
  }

  function switchMode(next: Mode) {
    if (status === "running" || status === "paused") return;
    setMode(next);
    setStatus("idle");
    setRemaining(durationFor(next));
  }

  const label = mode === "work" ? "Travail" : "Pause";

  return (
    <div className={`timer-panel ${status === "finished" ? "is-finished" : ""}`}>
      <div className="timer-display" aria-live="polite">
        <span className="timer-clock">⏱ {formatMmSs(remaining)}</span>
        <span className="timer-mode">
          {label}
          {status === "finished" ? " — terminé" : ""}
        </span>
      </div>

      <div className="timer-controls">
        {status === "running" ? (
          <button type="button" className="timer-btn" onClick={pause}>
            Pause
          </button>
        ) : status === "paused" ? (
          <button type="button" className="timer-btn is-primary" onClick={resume}>
            Reprendre
          </button>
        ) : (
          <button type="button" className="timer-btn is-primary" onClick={start}>
            Démarrer
          </button>
        )}
        <button
          type="button"
          className="timer-btn"
          onClick={stop}
          disabled={status === "idle"}
        >
          Arrêter
        </button>
        <button type="button" className="timer-btn" onClick={reset}>
          Réinit.
        </button>
      </div>

      <div className="timer-modes">
        <button
          type="button"
          className={`timer-chip ${mode === "work" ? "is-active" : ""}`}
          disabled={status === "running" || status === "paused"}
          onClick={() => switchMode("work")}
        >
          Travail
        </button>
        <button
          type="button"
          className={`timer-chip ${mode === "break" ? "is-active" : ""}`}
          disabled={status === "running" || status === "paused"}
          onClick={() => switchMode("break")}
        >
          Pause
        </button>
      </div>

      <div className="timer-durations">
        <label>
          Travail
          <input
            type="number"
            min={1}
            max={180}
            value={workMinutes}
            disabled={status === "running" || status === "paused"}
            onChange={(e) => applyWorkMinutes(Number(e.target.value))}
          />
          <span>min</span>
        </label>
        <label>
          Pause
          <input
            type="number"
            min={1}
            max={180}
            value={breakMinutes}
            disabled={status === "running" || status === "paused"}
            onChange={(e) => applyBreakMinutes(Number(e.target.value))}
          />
          <span>min</span>
        </label>
      </div>

      <label className="timer-sound">
        <input
          type="checkbox"
          checked={soundEnabled}
          onChange={(e) => setSoundEnabled(e.target.checked)}
        />
        <span>Son à la fin (simple)</span>
      </label>
    </div>
  );
}
