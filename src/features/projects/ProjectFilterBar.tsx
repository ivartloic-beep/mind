import type { Project } from "../../types/models";
import type { ProjectFilterValue } from "./filter";

type Props = {
  projects: Project[];
  value: ProjectFilterValue;
  onChange: (value: ProjectFilterValue) => void;
};

export function ProjectFilterBar({ projects, value, onChange }: Props) {
  const specific =
    value !== "all" && value !== "no-project" ? value : "";

  return (
    <div className="project-filter" role="group" aria-label="Filtrer par projet">
      <button
        type="button"
        className={`project-chip ${value === "all" ? "is-active" : ""}`}
        onClick={() => onChange("all")}
      >
        Tout
      </button>
      <button
        type="button"
        className={`project-chip ${value === "no-project" ? "is-active" : ""}`}
        onClick={() => onChange("no-project")}
      >
        Sans projet
      </button>
      <label className="project-filter-select-wrap">
        <span className="sr-only">Projet</span>
        <select
          className={`project-filter-select ${specific ? "is-active" : ""}`}
          value={specific}
          onChange={(e) => {
            const next = e.target.value;
            onChange(next ? next : "all");
          }}
        >
          <option value="">Projet…</option>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
