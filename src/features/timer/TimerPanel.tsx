/**
 * Minuteur durée libre — démarrer / pause / +5 min à la fin.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import "./timer.css";

type Status = "idle" | "running" | "paused" | "finished";

const STORAGE_KEY = "ma-tete.timer.prefs";

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

export function TimerPanel() {
  const initial = useMemo(() => loadPrefs(), []);
  const [minutes, setMinutes] = useState(initial.minutes);
  const [soundEnabled, setSoundEnabled] = useState(initial.soundEnabled);
  const [status, setStatus] = useState<Status>("idle");
  const [remaining, setRemaining] = useState(initial.minutes * 60);
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
          void notifyDone();
          if (soundRef.current) playBeep();
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => window.clearInterval(id);
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
    setRemaining(minutes * 60);
  }

  function addFiveMinutes() {
    setRemaining((prev) => prev + 5 * 60);
    setStatus("running");
  }

  return (
    <div className={`timer-panel ${status === "finished" ? "is-finished" : ""}`}>
      <div className="timer-display" aria-live="polite">
        <span className="timer-clock">⏱ {formatMmSs(remaining)}</span>
        <span className="timer-mode">
          {status === "finished"
            ? "Terminé"
            : status === "running"
              ? "En cours"
              : status === "paused"
                ? "En pause"
                : "Prêt"}
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
        {status === "finished" && (
          <button
            type="button"
            className="timer-btn is-primary"
            onClick={addFiveMinutes}
          >
            +5 min
          </button>
        )}
      </div>

      <div className="timer-durations">
        <label>
          Durée
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
      </div>

      <label className="timer-sound">
        <input
          type="checkbox"
          checked={soundEnabled}
          onChange={(e) => setSoundEnabled(e.target.checked)}
        />
        <span>Son à la fin</span>
      </label>
    </div>
  );
}
