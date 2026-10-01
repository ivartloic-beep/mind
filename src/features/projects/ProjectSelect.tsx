import type { Project } from "../../types/models";

type Props = {
  projects: Project[];
  value: string | null;
  disabled?: boolean;
  onChange: (projectId: string | null) => void;
  ariaLabel?: string;
};

/** Sélecteur projet — défaut Aucun, jamais forcé. */
export function ProjectSelect({
  projects,
  value,
  disabled,
  onChange,
  ariaLabel = "Projet",
}: Props) {
  return (
    <select
      className="project-assign-select"
      aria-label={ariaLabel}
      disabled={disabled}
      value={value ?? ""}
      onChange={(e) => onChange(e.target.value ? e.target.value : null)}
      onClick={(e) => e.stopPropagation()}
      onMouseDown={(e) => e.stopPropagation()}
    >
      <option value="">Aucun</option>
      {projects.map((p) => (
        <option key={p.id} value={p.id}>
          {p.name}
        </option>
      ))}
    </select>
  );
}
