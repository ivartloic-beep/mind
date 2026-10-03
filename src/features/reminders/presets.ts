/** Presets fireAt ISO pour rappels tâche. */

export function fireAtInMinutes(minutes: number): string {
  return new Date(Date.now() + minutes * 60_000).toISOString();
}

export function fireAtTomorrowMorning(): string {
  const d = new Date();
  d.setDate(d.getDate() + 1);
  d.setHours(9, 0, 0, 0);
  return d.toISOString();
}

/** Heure locale `HH:MM` → ISO. Si l’heure est déjà passée (ou < 30 s), demain. */
export function fireAtAtLocalTime(hhmm: string): string {
  const m = /^(\d{1,2}):(\d{2})$/.exec(hhmm.trim());
  if (!m) throw new Error("Heure invalide (HH:MM)");
  const h = Number(m[1]);
  const min = Number(m[2]);
  if (h > 23 || min > 59) throw new Error("Heure invalide (HH:MM)");
  const d = new Date();
  d.setSeconds(0, 0);
  d.setMilliseconds(0);
  d.setHours(h, min, 0, 0);
  if (d.getTime() <= Date.now() + 30_000) {
    d.setDate(d.getDate() + 1);
  }
  return d.toISOString();
}

export function localTimeFromIso(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "09:00";
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  return `${hh}:${mm}`;
}

export function formatReminderLabel(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const now = new Date();
  const sameDay =
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate();
  const time = d.toLocaleTimeString("fr-FR", {
    hour: "2-digit",
    minute: "2-digit",
  });
  if (sameDay) return `auj. ${time}`;
  return d.toLocaleString("fr-FR", {
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}
