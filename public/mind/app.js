/**
 * MIND mobile — panneau + capture via mind-api.
 * Auth : Bearer stocké localement (même token que le sync desktop).
 */

const STORAGE_KEY = "mind.mobile.prefs";
const DEFAULT_BASE = "https://mind.louetline.fr";

/** @typedef {{ baseUrl: string, token: string }} Prefs */
/** @typedef {{ id: string, title: string, status?: string, priority?: number, dueAt?: string|null, notes?: string, completedAt?: string|null, updatedAt?: string, deletedAt?: string|null }} CloudTask */
/** @typedef {{ id: string, title: string, remindAt: string, taskId?: string|null, body?: string, done?: boolean, deletedAt?: string|null }} CloudReminder */

const state = {
  /** @type {Prefs} */
  prefs: loadPrefs(),
  mode: "day",
  /** @type {CloudTask[]} */
  tasks: [],
  /** @type {CloudReminder[]} */
  reminders: [],
  captureKind: "task",
  busy: false,
  message: "",
  error: "",
  editReminderId: null,
};

function loadPrefs() {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { baseUrl: DEFAULT_BASE, token: "" };
    const parsed = JSON.parse(raw);
    return {
      baseUrl: String(parsed.baseUrl || DEFAULT_BASE).replace(/\/$/, ""),
      token: String(parsed.token || ""),
    };
  } catch {
    return { baseUrl: DEFAULT_BASE, token: "" };
  }
}

function savePrefs() {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state.prefs));
}

function $(sel) {
  const el = document.querySelector(sel);
  if (!el) throw new Error(`missing ${sel}`);
  return el;
}

function uid() {
  if (crypto.randomUUID) return crypto.randomUUID().replace(/-/g, "").slice(0, 32);
  return `m${Date.now().toString(36)}${Math.random().toString(36).slice(2, 10)}`;
}

function nowIso() {
  return new Date().toISOString();
}

function isOpenTask(t) {
  if (t.deletedAt) return false;
  if (t.completedAt) return false;
  const s = (t.status || "todo").toLowerCase();
  return s !== "done" && s !== "completed" && s !== "cancelled" && s !== "canceled";
}

function scoreDayTask(task, now = new Date()) {
  let score = 0;
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const tomorrow = new Date(today);
  tomorrow.setDate(tomorrow.getDate() + 1);
  if (task.dueAt) {
    const due = new Date(task.dueAt.length === 10 ? `${task.dueAt}T12:00:00` : task.dueAt);
    if (!Number.isNaN(due.getTime())) {
      const dueDay = new Date(due.getFullYear(), due.getMonth(), due.getDate());
      if (dueDay < today) {
        const daysLate = Math.floor((today - dueDay) / 86_400_000);
        score += 1000 + Math.min(daysLate, 14) * 20;
      } else if (+dueDay === +today) score += 800;
      else if (+dueDay === +tomorrow) score += 600;
      else {
        const daysAhead = Math.floor((dueDay - today) / 86_400_000);
        score += Math.max(0, 200 - daysAhead * 15);
      }
    }
  }
  const p = Number(task.priority ?? 1);
  if (p >= 2) score += 300;
  else if (p === 1) score += 100;
  else score += 20;
  if ((task.status || "").toLowerCase() === "in_progress") score += 150;
  return score;
}

function pickDayQueue(tasks, limit = 6) {
  return [...tasks]
    .filter(isOpenTask)
    .sort((a, b) => scoreDayTask(b) - scoreDayTask(a) || String(b.updatedAt).localeCompare(String(a.updatedAt)))
    .slice(0, limit);
}

function formatDue(iso) {
  if (!iso) return "";
  const d = new Date(iso.length === 10 ? `${iso}T12:00:00` : iso);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleDateString("fr-FR", { day: "numeric", month: "short" });
}

function formatWhen(iso) {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const now = new Date();
  const same =
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate();
  const time = d.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });
  if (same) return `auj. ${time}`;
  return d.toLocaleString("fr-FR", {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function localTimeFromIso(iso) {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "09:00";
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

function fireAtAtLocalTime(hhmm) {
  const m = /^(\d{1,2}):(\d{2})$/.exec(String(hhmm).trim());
  if (!m) throw new Error("Heure invalide");
  const h = Number(m[1]);
  const min = Number(m[2]);
  const d = new Date();
  d.setSeconds(0, 0);
  d.setMilliseconds(0);
  d.setHours(h, min, 0, 0);
  if (d.getTime() <= Date.now() + 30_000) d.setDate(d.getDate() + 1);
  return d.toISOString();
}

function defaultLocalTime() {
  const d = new Date();
  d.setMinutes(0, 0, 0);
  d.setHours(d.getHours() + 1);
  return localTimeFromIso(d.toISOString());
}

async function api(path, options = {}) {
  const base = state.prefs.baseUrl.replace(/\/$/, "") || DEFAULT_BASE;
  const headers = new Headers(options.headers || {});
  if (state.prefs.token) headers.set("Authorization", `Bearer ${state.prefs.token}`);
  if (options.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }
  const res = await fetch(`${base}${path}`, { ...options, headers });
  if (res.status === 401) throw new Error("Token invalide — ouvre ⚙ et colle ton Bearer.");
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`API ${res.status}${text ? ` — ${text.slice(0, 120)}` : ""}`);
  }
  if (res.status === 204) return null;
  const ct = res.headers.get("content-type") || "";
  if (ct.includes("application/json")) return res.json();
  return null;
}

async function refresh() {
  if (!state.prefs.token) {
    state.tasks = [];
    state.reminders = [];
    state.error = "Colle ton token API (⚙) — le même que dans MIND PC.";
    render();
    return;
  }
  state.error = "";
  const [tasksRes, remindersRes] = await Promise.all([
    api("/tasks?limit=1000"),
    api("/reminders?limit=1000"),
  ]);
  state.tasks = Array.isArray(tasksRes?.items) ? tasksRes.items : [];
  state.reminders = Array.isArray(remindersRes?.items) ? remindersRes.items : [];
  render();
}

async function createCapture() {
  const title = /** @type {HTMLInputElement} */ ($("#capture-title")).value.trim();
  const notes = /** @type {HTMLTextAreaElement} */ ($("#capture-notes")).value.trim();
  if (!title) {
    state.error = "Titre requis.";
    render();
    return;
  }
  state.busy = true;
  state.error = "";
  state.message = "";
  render();
  try {
    const id = uid();
    const ts = nowIso();
    if (state.captureKind === "task") {
      await api(`/tasks/${id}`, {
        method: "PUT",
        body: JSON.stringify({
          id,
          title,
          notes,
          status: "todo",
          priority: 1,
          createdAt: ts,
          updatedAt: ts,
        }),
      });
    } else {
      const kind = state.captureKind === "idea" ? "idea" : "note";
      const body = notes
        ? `${notes}\n\n<!--mind-meta:{"kind":"${kind}"}-->`
        : `<!--mind-meta:{"kind":"${kind}"}-->`;
      await api(`/notes/${id}`, {
        method: "PUT",
        body: JSON.stringify({
          id,
          title,
          body,
          pinned: false,
          createdAt: ts,
          updatedAt: ts,
        }),
      });
    }
    /** @type {HTMLInputElement} */ ($("#capture-title")).value = "";
    /** @type {HTMLTextAreaElement} */ ($("#capture-notes")).value = "";
    state.message =
      state.captureKind === "task" ? "Tâche capturée." : state.captureKind === "idea" ? "Idée capturée." : "Note capturée.";
    await refresh();
  } catch (err) {
    state.error = err instanceof Error ? err.message : "Capture impossible";
    render();
  } finally {
    state.busy = false;
    render();
  }
}

async function completeTask(task) {
  state.busy = true;
  state.error = "";
  render();
  try {
    const ts = nowIso();
    await api(`/tasks/${task.id}`, {
      method: "PUT",
      body: JSON.stringify({
        ...task,
        status: "done",
        completedAt: ts,
        updatedAt: ts,
      }),
    });
    await refresh();
  } catch (err) {
    state.error = err instanceof Error ? err.message : "Mise à jour impossible";
  } finally {
    state.busy = false;
    render();
  }
}

async function scheduleReminder() {
  const taskId = /** @type {HTMLSelectElement} */ ($("#reminder-task")).value;
  const time = /** @type {HTMLInputElement} */ ($("#reminder-time")).value;
  if (!taskId || !time) return;
  const task = state.tasks.find((t) => t.id === taskId);
  state.busy = true;
  state.error = "";
  state.message = "";
  render();
  try {
    const remindAt = fireAtAtLocalTime(time);
    const id = uid();
    const ts = nowIso();
    await api(`/reminders/${id}`, {
      method: "PUT",
      body: JSON.stringify({
        id,
        title: task?.title || "Rappel",
        remindAt,
        taskId,
        body: "",
        done: false,
        createdAt: ts,
        updatedAt: ts,
      }),
    });
    state.message = "Rappel planifié.";
    await refresh();
  } catch (err) {
    state.error = err instanceof Error ? err.message : "Planification impossible";
  } finally {
    state.busy = false;
    render();
  }
}

async function updateReminder(reminder, hhmm) {
  state.busy = true;
  state.error = "";
  render();
  try {
    const remindAt = fireAtAtLocalTime(hhmm);
    await api(`/reminders/${reminder.id}`, {
      method: "PUT",
      body: JSON.stringify({
        ...reminder,
        remindAt,
        done: false,
        updatedAt: nowIso(),
      }),
    });
    state.editReminderId = null;
    await refresh();
  } catch (err) {
    state.error = err instanceof Error ? err.message : "Modification impossible";
  } finally {
    state.busy = false;
    render();
  }
}

async function deleteReminder(id) {
  state.busy = true;
  state.error = "";
  render();
  try {
    await api(`/reminders/${id}`, { method: "DELETE" });
    await refresh();
  } catch (err) {
    state.error = err instanceof Error ? err.message : "Suppression impossible";
  } finally {
    state.busy = false;
    render();
  }
}

function isIos() {
  return /iPad|iPhone|iPod/.test(navigator.userAgent) || (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1);
}

function isStandalone() {
  return window.matchMedia("(display-mode: standalone)").matches || /** @type {any} */ (navigator).standalone === true;
}

function render() {
  const configured = Boolean(state.prefs.token);
  $("#status-line").textContent = configured
    ? `Connecté · ${state.prefs.baseUrl.replace(/^https?:\/\//, "")}`
    : "Token manquant";

  document.querySelectorAll(".mode-btn").forEach((btn) => {
    btn.classList.toggle("is-active", btn.getAttribute("data-mode") === state.mode);
  });
  document.querySelectorAll(".panel").forEach((panel) => {
    panel.classList.toggle("is-active", panel.getAttribute("data-panel") === state.mode);
  });

  const hint = $("#install-hint");
  if (isIos() && !isStandalone()) {
    hint.hidden = false;
    hint.textContent =
      "Sur iPhone : Partager → Sur l’écran d’accueil → ajouter MIND. Puis crée le raccourci Capture (voir docs).";
  } else {
    hint.hidden = true;
  }

  const dayList = $("#day-list");
  const queue = pickDayQueue(state.tasks);
  if (!configured) {
    dayList.innerHTML = `<li class="muted">Configure le token pour voir ta file du jour.</li>`;
  } else if (queue.length === 0) {
    dayList.innerHTML = `<li class="muted">Aucune tâche ouverte.</li>`;
  } else {
    dayList.innerHTML = queue
      .map((t) => {
        const due = formatDue(t.dueAt);
        const meta = [due && `échéance ${due}`, t.status && t.status !== "todo" ? t.status : ""]
          .filter(Boolean)
          .join(" · ");
        return `<li class="item" data-id="${t.id}">
          <button type="button" class="check" data-action="done" aria-label="Terminer"></button>
          <div class="item-main">
            <p class="item-title"></p>
            ${meta ? `<p class="item-meta"></p>` : ""}
          </div>
        </li>`;
      })
      .join("");
    [...dayList.children].forEach((li, i) => {
      const t = queue[i];
      li.querySelector(".item-title").textContent = t.title;
      const metaEl = li.querySelector(".item-meta");
      if (metaEl) {
        const due = formatDue(t.dueAt);
        metaEl.textContent = [due && `échéance ${due}`, t.status && t.status !== "todo" ? t.status : ""]
          .filter(Boolean)
          .join(" · ");
      }
      li.querySelector('[data-action="done"]').addEventListener("click", () => void completeTask(t));
    });
  }

  document.querySelectorAll(".kind-btn").forEach((btn) => {
    btn.classList.toggle("is-active", btn.getAttribute("data-kind") === state.captureKind);
  });

  const remList = $("#reminder-list");
  const now = Date.now();
  const openTasks = state.tasks.filter(isOpenTask).sort((a, b) => a.title.localeCompare(b.title, "fr"));
  const upcoming = state.reminders
    .filter((r) => !r.deletedAt && !r.done)
    .filter((r) => {
      const t = new Date(r.remindAt).getTime();
      return Number.isFinite(t) && t >= now - 60_000;
    })
    .sort((a, b) => new Date(a.remindAt) - new Date(b.remindAt));

  if (!configured) {
    remList.innerHTML = `<li class="muted">Token requis.</li>`;
  } else if (upcoming.length === 0) {
    remList.innerHTML = `<li class="muted">Aucun rappel planifié.</li>`;
  } else {
    remList.innerHTML = upcoming
      .map((r) => {
        const task = r.taskId ? state.tasks.find((t) => t.id === r.taskId) : null;
        const editing = state.editReminderId === r.id;
        return `<li class="item" data-rid="${r.id}">
          <div class="item-main">
            <p class="item-meta when"></p>
            <p class="item-title title"></p>
            <div class="row-actions"></div>
          </div>
        </li>`;
      })
      .join("");
    [...remList.children].forEach((li, i) => {
      const r = upcoming[i];
      const task = r.taskId ? state.tasks.find((t) => t.id === r.taskId) : null;
      li.querySelector(".when").textContent = formatWhen(r.remindAt);
      li.querySelector(".title").textContent = task?.title || r.title || "Rappel";
      const actions = li.querySelector(".row-actions");
      if (state.editReminderId === r.id) {
        actions.innerHTML = `
          <input type="time" class="edit-time" value="${localTimeFromIso(r.remindAt)}" />
          <button type="button" class="btn btn-primary" data-a="save">OK</button>
          <button type="button" class="btn" data-a="cancel">Annuler</button>`;
        actions.querySelector('[data-a="save"]').addEventListener("click", () => {
          const v = /** @type {HTMLInputElement} */ (actions.querySelector(".edit-time")).value;
          void updateReminder(r, v);
        });
        actions.querySelector('[data-a="cancel"]').addEventListener("click", () => {
          state.editReminderId = null;
          render();
        });
      } else {
        actions.innerHTML = `
          <button type="button" class="btn" data-a="edit">Modifier</button>
          <button type="button" class="btn" data-a="del">Supprimer</button>`;
        actions.querySelector('[data-a="edit"]').addEventListener("click", () => {
          state.editReminderId = r.id;
          render();
        });
        actions.querySelector('[data-a="del"]').addEventListener("click", () => void deleteReminder(r.id));
      }
    });
  }

  const sel = /** @type {HTMLSelectElement} */ ($("#reminder-task"));
  const prev = sel.value;
  sel.innerHTML = openTasks.length
    ? openTasks.map((t) => `<option value="${t.id}"></option>`).join("")
    : `<option value="">Aucune tâche ouverte</option>`;
  [...sel.options].forEach((opt, i) => {
    if (openTasks[i]) opt.textContent = openTasks[i].title;
  });
  if (prev && openTasks.some((t) => t.id === prev)) sel.value = prev;

  const msg = $("#flash");
  msg.className = state.error ? "error" : state.message ? "ok" : "muted";
  msg.textContent = state.error || state.message || "";

  $("#capture-submit").disabled = state.busy || !configured;
  $("#reminder-submit").disabled = state.busy || !configured || openTasks.length === 0;
  $("#refresh-btn").disabled = state.busy;
}

function bind() {
  document.querySelectorAll(".mode-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      state.mode = btn.getAttribute("data-mode") || "day";
      state.message = "";
      state.error = "";
      render();
    });
  });

  document.querySelectorAll(".kind-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      state.captureKind = btn.getAttribute("data-kind") || "task";
      render();
    });
  });

  $("#settings-toggle").addEventListener("click", () => {
    $("#settings").classList.toggle("is-open");
  });

  $("#settings-save").addEventListener("click", () => {
    state.prefs.baseUrl =
      /** @type {HTMLInputElement} */ ($("#pref-base")).value.trim().replace(/\/$/, "") || DEFAULT_BASE;
    state.prefs.token = /** @type {HTMLInputElement} */ ($("#pref-token")).value.trim();
    savePrefs();
    state.message = "Enregistré.";
    state.error = "";
    void refresh();
  });

  $("#capture-submit").addEventListener("click", () => void createCapture());
  $("#reminder-submit").addEventListener("click", () => void scheduleReminder());
  $("#refresh-btn").addEventListener("click", () => void refresh());

  /** @type {HTMLInputElement} */ ($("#pref-base")).value = state.prefs.baseUrl;
  /** @type {HTMLInputElement} */ ($("#pref-token")).value = state.prefs.token;
  /** @type {HTMLInputElement} */ ($("#reminder-time")).value = defaultLocalTime();

  // Deep link ?mode=capture|day|timer
  const params = new URLSearchParams(location.search);
  const mode = params.get("mode");
  if (mode === "capture" || mode === "day" || mode === "timer") state.mode = mode;
}

async function main() {
  bind();
  render();
  if ("serviceWorker" in navigator) {
    try {
      await navigator.serviceWorker.register("./sw.js");
    } catch {
      /* ignore */
    }
  }
  try {
    await refresh();
  } catch (err) {
    state.error = err instanceof Error ? err.message : "Chargement impossible";
    render();
  }
}

void main();
