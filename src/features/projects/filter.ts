import type { NoteFilter, TaskFilter } from "../../types/models";

/** Filtre vue : mêmes données, projections différentes. */
export type ProjectFilterValue = "all" | "no-project" | string;

export function toTaskFilter(filter: ProjectFilterValue): TaskFilter | undefined {
  if (filter === "all") return undefined;
  if (filter === "no-project") return { noProject: true };
  return { projectId: filter };
}

export function toNoteFilter(filter: ProjectFilterValue): NoteFilter | undefined {
  if (filter === "all") return undefined;
  if (filter === "no-project") return { noProject: true };
  return { projectId: filter };
}
