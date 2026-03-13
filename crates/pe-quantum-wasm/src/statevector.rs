use num_complex::Complex64;

use crate::error::QuantumError;
use crate::types::MAX_QUBITS;

/// A 2^n complex amplitude vector representing an n-qubit quantum state.
#[derive(Debug, Clone)]
pub struct StateVector {
    num_qubits: usize,
    amplitudes: Vec<Complex64>,
}

impl StateVector {
    /// Create the |00...0⟩ state for `n` qubits.
    pub fn new(num_qubits: usize) -> Result<Self, QuantumError> {
        if num_qubits == 0 {
            return Err(QuantumError::EmptyCircuit);
        }
        if num_qubits > MAX_QUBITS {
            return Err(QuantumError::TooManyQubits {
                required: num_qubits,
                max: MAX_QUBITS,
            });
        }

        let size = 1 << num_qubits;
        let mut amplitudes = vec![Complex64::new(0.0, 0.0); size];
        amplitudes[0] = Complex64::new(1.0, 0.0);
        Ok(Self {
            num_qubits,
            amplitudes,
        })
    }

    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    pub fn amplitudes(&self) -> &[Complex64] {
        &self.amplitudes
    }

    /// Probability of measuring the given basis state.
    pub fn probability(&self, state_index: usize) -> f64 {
        if state_index >= self.amplitudes.len() {
            return 0.0;
        }
        self.amplitudes[state_index].norm_sqr()
    }

    /// All probabilities as a vector.
    pub fn probabilities(&self) -> Vec<f64> {
        self.amplitudes.iter().map(|a| a.norm_sqr()).collect()
    }

    /// Measure all qubits, collapsing the state. Returns the measured bitstring
    /// as a usize and the probability of that outcome.
    pub fn measure<R: rand::Rng>(&mut self, rng: &mut R) -> (usize, f64) {
        let probs = self.probabilities();
        let r: f64 = rng.gen();
        let mut cumulative = 0.0;
        let mut outcome = probs.len() - 1;

        for (i, &p) in probs.iter().enumerate() {
            cumulative += p;
            if r < cumulative {
                outcome = i;
                break;
            }
        }

        let prob = probs[outcome];

        // Collapse
        for (i, amp) in self.amplitudes.iter_mut().enumerate() {
            if i == outcome {
                *amp = Complex64::new(1.0, 0.0);
            } else {
                *amp = Complex64::new(0.0, 0.0);
            }
        }

        (outcome, prob)
    }

    /// Apply a single-qubit gate represented as a 2×2 matrix.
    fn apply_single(&mut self, qubit: usize, gate: [[Complex64; 2]; 2]) -> Result<(), QuantumError> {
        if qubit >= self.num_qubits {
            return Err(QuantumError::QubitOutOfRange(qubit, self.num_qubits));
        }

        let size = self.amplitudes.len();
        let bit = 1 << qubit;

        for i in 0..size {
            if i & bit != 0 {
                continue;
            }
            let j = i | bit;
            let a = self.amplitudes[i];
            let b = self.amplitudes[j];
            self.amplitudes[i] = gate[0][0] * a + gate[0][1] * b;
            self.amplitudes[j] = gate[1][0] * a + gate[1][1] * b;
        }
        Ok(())
    }

    /// Apply a controlled-unitary gate (2-qubit).
    fn apply_controlled(
        &mut self,
        control: usize,
        target: usize,
        gate: [[Complex64; 2]; 2],
    ) -> Result<(), QuantumError> {
        if control >= self.num_qubits {
            return Err(QuantumError::QubitOutOfRange(control, self.num_qubits));
        }
        if target >= self.num_qubits {
            return Err(QuantumError::QubitOutOfRange(target, self.num_qubits));
        }
        if control == target {
            return Err(QuantumError::SameControlTarget(control));
        }

        let size = self.amplitudes.len();
        let ctrl_bit = 1 << control;
        let tgt_bit = 1 << target;

        for i in 0..size {
            // Only apply when control qubit is |1⟩ and target is |0⟩ position
            if (i & ctrl_bit) == 0 || (i & tgt_bit) != 0 {
                continue;
            }
            // i has control=1, target=0
            let idx0 = i & !tgt_bit; // control=1, target=0
            let idx1 = i | tgt_bit;  // control=1, target=1

            let a = self.amplitudes[idx0];
            let b = self.amplitudes[idx1];
            self.amplitudes[idx0] = gate[0][0] * a + gate[0][1] * b;
            self.amplitudes[idx1] = gate[1][0] * a + gate[1][1] * b;
        }
        Ok(())
    }

    // ── Gate operations ──────────────────────────────────────────────

    /// Hadamard gate: |0⟩ → (|0⟩+|1⟩)/√2, |1⟩ → (|0⟩−|1⟩)/√2
    pub fn h(&mut self, qubit: usize) -> Result<(), QuantumError> {
        let s = Complex64::new(std::f64::consts::FRAC_1_SQRT_2, 0.0);
        self.apply_single(qubit, [[s, s], [s, -s]])
    }

    /// Pauli-X (NOT) gate.
    pub fn x(&mut self, qubit: usize) -> Result<(), QuantumError> {
        let zero = Complex64::new(0.0, 0.0);
        let one = Complex64::new(1.0, 0.0);
        self.apply_single(qubit, [[zero, one], [one, zero]])
    }

    /// Pauli-Y gate.
    pub fn y(&mut self, qubit: usize) -> Result<(), QuantumError> {
        let zero = Complex64::new(0.0, 0.0);
        let ni = Complex64::new(0.0, -1.0);
        let pi = Complex64::new(0.0, 1.0);
        self.apply_single(qubit, [[zero, ni], [pi, zero]])
    }

    /// Pauli-Z gate.
    pub fn z(&mut self, qubit: usize) -> Result<(), QuantumError> {
        let zero = Complex64::new(0.0, 0.0);
        let one = Complex64::new(1.0, 0.0);
        let neg = Complex64::new(-1.0, 0.0);
        self.apply_single(qubit, [[one, zero], [zero, neg]])
    }

    /// Rotation around X axis by angle θ.
    pub fn rx(&mut self, qubit: usize, theta: f64) -> Result<(), QuantumError> {
        let c = Complex64::new((theta / 2.0).cos(), 0.0);
        let s = Complex64::new(0.0, -(theta / 2.0).sin());
        self.apply_single(qubit, [[c, s], [s, c]])
    }

    /// Rotation around Y axis by angle θ.
    pub fn ry(&mut self, qubit: usize, theta: f64) -> Result<(), QuantumError> {
        let c = Complex64::new((theta / 2.0).cos(), 0.0);
        let s_pos = Complex64::new((theta / 2.0).sin(), 0.0);
        let s_neg = Complex64::new(-(theta / 2.0).sin(), 0.0);
        self.apply_single(qubit, [[c, s_neg], [s_pos, c]])
    }

    /// Rotation around Z axis by angle θ.
    pub fn rz(&mut self, qubit: usize, theta: f64) -> Result<(), QuantumError> {
        let zero = Complex64::new(0.0, 0.0);
        let e_neg = Complex64::new((theta / 2.0).cos(), -(theta / 2.0).sin());
        let e_pos = Complex64::new((theta / 2.0).cos(), (theta / 2.0).sin());
        self.apply_single(qubit, [[e_neg, zero], [zero, e_pos]])
    }

    /// CNOT (controlled-X) gate.
    pub fn cnot(&mut self, control: usize, target: usize) -> Result<(), QuantumError> {
        let zero = Complex64::new(0.0, 0.0);
        let one = Complex64::new(1.0, 0.0);
        self.apply_controlled(control, target, [[zero, one], [one, zero]])
    }

    /// CZ (controlled-Z) gate.
    pub fn cz(&mut self, control: usize, target: usize) -> Result<(), QuantumError> {
        let zero = Complex64::new(0.0, 0.0);
        let one = Complex64::new(1.0, 0.0);
        let neg = Complex64::new(-1.0, 0.0);
        self.apply_controlled(control, target, [[one, zero], [zero, neg]])
    }

    /// Compute the expectation value ⟨ψ|H|ψ⟩ for a dense Hamiltonian matrix.
    pub fn expectation_value(&self, hamiltonian: &[Vec<Complex64>]) -> f64 {
        let n = self.amplitudes.len();
        let mut result = Complex64::new(0.0, 0.0);

        for i in 0..n {
            let mut h_psi_i = Complex64::new(0.0, 0.0);
            for j in 0..n {
                h_psi_i += hamiltonian[i][j] * self.amplitudes[j];
            }
            result += self.amplitudes[i].conj() * h_psi_i;
        }
        result.re
    }
}
