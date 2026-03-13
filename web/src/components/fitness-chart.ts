/**
 * Fitness chart component — visualizes fitness scores over generations.
 *
 * Placeholder: full implementation will render a line/radar chart
 * of fitness dimensions using Canvas or a lightweight charting library.
 */

import type { FitnessScore } from "../types";

export function renderFitnessChart(
  container: HTMLElement,
  scores: FitnessScore[],
): void {
  const pre = document.createElement("pre");
  pre.textContent = scores
    .map(
      (s, i) =>
        `[${i}] composite=${s.composite.toFixed(3)}  ` +
        `reprog=${s.reprogramming_efficiency.toFixed(3)}  ` +
        `stab=${s.expression_stability.toFixed(3)}  ` +
        `struct=${s.structural_plausibility.toFixed(3)}  ` +
        `safety=${s.safety_score.toFixed(3)}`,
    )
    .join("\n");
  container.replaceChildren(pre);
}
