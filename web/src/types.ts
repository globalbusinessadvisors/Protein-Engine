/** TypeScript interfaces matching pe-core domain types. */

export interface FitnessScore {
  reprogramming_efficiency: number;
  expression_stability: number;
  structural_plausibility: number;
  safety_score: number;
  composite: number;
}

export interface ProteinVariant {
  id: string;
  name: string;
  sequence: string;
  target_factor: string;
  generation: number;
  parent_id: string | null;
}

export interface CycleResult {
  generation: number;
  variants_created: number;
  variants_scored: number;
  promoted: Array<{
    name: string;
    sequence: string;
    composite: number;
  }>;
}

export interface VqeResult {
  ground_state_energy: number;
  optimal_parameters: number[];
  converged: boolean;
  iterations: number;
}
