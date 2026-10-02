/**
 * Post-it autonome sans cadre Windows — drag + croix = suppression.
 */

import { useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getPostit, upsertPostit } from "../../services/api";
import { postitSetAlwaysOnTop, postitUpdateGeometry } from "../../services/postit";
import type { PostIt } from "../../types/models";
import "./postit.css";

function readPostitId(): string | null {
  const params = new URLSearchParams(window.location.search);
  return params.get("postitId");
}

export function PostItApp() {
  const postitId = useMemo(() => readPostitId(), []);
  const [postit, setPostit] = useState<PostIt | null>(null);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const saveTimer = useRef<number | null>(null);
  const geoTimer = useRef<number | null>(null);
  const postitRef = useRef<PostIt | null>(null);
  postitRef.current = postit;

  useEffect(() => {
    if (!postitId) {
      setError("Post-it sans identifiant.");
      return;
    }
    let cancelled = false;

    async function bootstrap() {
      try {
        const p = await getPostit(postitId!);
        if (cancelled) return;
        if (!p) {
          setError("Post-it introuvable.");
          return;
        }
        setPostit(p);
        setTitle(p.title ?? "");
        setBody(p.body ?? "");
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Chargement impossible");
        }
      }
    }

    void bootstrap();
    return () => {
      cancelled = true;
    };
  }, [postitId]);

  useEffect(() => {
    if (!postitId) return;
    const unsubs: Array<() => void> = [];

    async function wireWindow() {
      try {
        const win = getCurrentWindow();
        const persistGeo = () => {
          if (geoTimer.current) window.clearTimeout(geoTimer.current);
          geoTimer.current = window.setTimeout(() => {
            void (async () => {
              try {
                const pos = await win.outerPosition();
                const size = await win.outerSize();
                const scale = await win.scaleFactor();
                await postitUpdateGeometry(
                  postitId!,
                  pos.x / scale,
                  pos.y / scale,
                  size.width / scale,
                  size.height / scale,
                );
              } catch {
                /* ignore */
              }
            })();
          }, 280);
        };

        unsubs.push(await win.onMoved(persistGeo));
        unsubs.push(await win.onResized(persistGeo));
      } catch {
        /* hors Tauri */
      }
    }

    void wireWindow();
    return () => {
      for (const u of unsubs) u();
      if (geoTimer.current) window.clearTimeout(geoTimer.current);
    };
  }, [postitId]);

  function scheduleSave(nextTitle: string, nextBody: string) {
    const current = postitRef.current;
    if (!current) return;
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      void (async () => {
        setSaving(true);
        try {
          const updated = await upsertPostit({
            ...current,
            title: nextTitle.trim() || undefined,
            body: nextBody,
          });
          setPostit(updated);
          setError(null);
        } catch (err) {
          setError(err instanceof Error ? err.message : "Enregistrement impossible");
        } finally {
          setSaving(false);
        }
      })();
    }, 400);
  }

  async function toggleAlwaysOnTop() {
    if (!postit) return;
    try {
      const updated = await postitSetAlwaysOnTop(postit.id, !postit.alwaysOnTop);
      setPostit(updated);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Always-on-top impossible");
    }
  }

  async function closePostit() {
    try {
      await getCurrentWindow().close();
    } catch {
      /* hors Tauri */
    }
  }

  if (error && !postit) {
    return (
      <main className="postit-root">
        <p className="postit-error">{error}</p>
        <button type="button" className="postit-close" onClick={() => void closePostit()}>
          ×
        </button>
      </main>
    );
  }

  return (
    <main className="postit-root">
      <header className="postit-header" data-tauri-drag-region>
        <input
          className="postit-title"
          value={title}
          placeholder="Post-it"
          onChange={(e) => {
            const next = e.target.value;
            setTitle(next);
            scheduleSave(next, body);
          }}
        />
        {saving && <span className="postit-saving">…</span>}
        <button
          type="button"
          className={`postit-pin ${(postit?.alwaysOnTop ?? true) ? "is-on" : ""}`}
          aria-label={
            postit?.alwaysOnTop ?? true
              ? "Désépingler (ne plus garder au-dessus)"
              : "Épingler (garder au-dessus)"
          }
          title={
            postit?.alwaysOnTop ?? true
              ? "Épinglé — toujours au-dessus"
              : "Épingler — toujours au-dessus"
          }
          aria-pressed={postit?.alwaysOnTop ?? true}
          onClick={() => void toggleAlwaysOnTop()}
        >
          <svg viewBox="0 0 24 24" width="15" height="15" aria-hidden="true">
            <path
              fill="currentColor"
              d="M16 3a1 1 0 0 1 1 1v2.2l1.6.9a1 1 0 0 1 .4 1.3l-1.5 2.7.9.9a1 1 0 0 1 0 1.4l-2.1 2.1-1.1-1.1-3.2 5.6a1 1 0 0 1-1.8-.2l-1.5-4.1-1.8 1.8a1 1 0 0 1-1.4 0L4.4 15a1 1 0 0 1 0-1.4l1.8-1.8-4.1-1.5a1 1 0 0 1-.2-1.8l5.6-3.2-1.1-1.1a1 1 0 0 1 1.4 0l.9.9 2.7-1.5a1 1 0 0 1 1.3.4L15 6V4a1 1 0 0 1 1-1Z"
            />
          </svg>
        </button>
        <button
          type="button"
          className="postit-close"
          aria-label="Fermer et supprimer"
          title="Fermer (supprime le post-it)"
          onClick={() => void closePostit()}
        >
          ×
        </button>
      </header>

      <textarea
        className="postit-body postit-body-full"
        value={body}
        placeholder="Note rapide…"
        autoFocus
        onChange={(e) => {
          const next = e.target.value;
          setBody(next);
          scheduleSave(title, next);
        }}
      />
      {error && postit && <p className="postit-error">{error}</p>}
    </main>
  );
}
