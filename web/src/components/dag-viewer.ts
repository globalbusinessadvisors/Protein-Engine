/**
 * DAG viewer component — visualizes variant lineage as a directed acyclic graph.
 *
 * Placeholder: full implementation will render parent→child relationships
 * between protein variants across generations using SVG or Canvas.
 */

import type { ProteinVariant } from "../types";

export interface DagNode {
  id: string;
  label: string;
  parentId: string | null;
  generation: number;
}

export function buildDag(variants: ProteinVariant[]): DagNode[] {
  return variants.map((v) => ({
    id: v.id,
    label: `${v.name} (gen ${v.generation})`,
    parentId: v.parent_id,
    generation: v.generation,
  }));
}

export function renderDag(container: HTMLElement, variants: ProteinVariant[]): void {
  const nodes = buildDag(variants);
  const pre = document.createElement("pre");
  pre.textContent = nodes
    .map((n) => `${n.parentId ?? "root"} → ${n.id}  ${n.label}`)
    .join("\n");
  container.replaceChildren(pre);
}
