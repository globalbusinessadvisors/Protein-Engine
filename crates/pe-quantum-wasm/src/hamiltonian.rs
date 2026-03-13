use num_complex::Complex64;
use serde::{Deserialize, Serialize};

/// A Pauli operator acting on specific qubits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PauliOp {
    I,
    X,
    Y,
    Z,
}

/// A single term in a Hamiltonian: coefficient × (tensor product of Paulis).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauliTerm {
    pub coefficient: f64,
    pub operators: Vec<(usize, PauliOp)>,
}

/// A molecular Hamiltonian expressed as a sum of Pauli terms.
///
/// H = Σ_i c_i × P_i where each P_i is a tensor product of Pauli matrices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MolecularHamiltonian {
    pub num_qubits: usize,
    pub terms: Vec<PauliTerm>,
}

impl MolecularHamiltonian {
    /// Build the H2 molecule Hamiltonian at equilibrium bond length.
    ///
    /// This is the standard 2-qubit Jordan-Wigner encoded H2 Hamiltonian
    /// with known ground state energy ≈ −1.137 Hartree.
    pub fn h2_molecule() -> Self {
        Self {
            num_qubits: 2,
            terms: vec![
                PauliTerm {
                    coefficient: -0.4804,
                    operators: vec![],
                },
                PauliTerm {
                    coefficient: 0.3435,
                    operators: vec![(0, PauliOp::Z)],
                },
                PauliTerm {
                    coefficient: -0.4347,
                    operators: vec![(1, PauliOp::Z)],
                },
                PauliTerm {
                    coefficient: 0.5716,
                    operators: vec![(0, PauliOp::Z), (1, PauliOp::Z)],
                },
                PauliTerm {
                    coefficient: 0.0910,
                    operators: vec![(0, PauliOp::X), (1, PauliOp::X)],
                },
                PauliTerm {
                    coefficient: 0.0910,
                    operators: vec![(0, PauliOp::Y), (1, PauliOp::Y)],
                },
            ],
        }
    }

    /// Convert to a dense 2^n × 2^n complex matrix.
    pub fn to_matrix(&self) -> Vec<Vec<Complex64>> {
        let dim = 1 << self.num_qubits;
        let mut matrix = vec![vec![Complex64::new(0.0, 0.0); dim]; dim];

        for term in &self.terms {
            let term_matrix = self.pauli_term_matrix(term);
            for i in 0..dim {
                for j in 0..dim {
                    matrix[i][j] += term_matrix[i][j];
                }
            }
        }
        matrix
    }

    fn pauli_term_matrix(&self, term: &PauliTerm) -> Vec<Vec<Complex64>> {
        let dim = 1 << self.num_qubits;
        let coeff = Complex64::new(term.coefficient, 0.0);

        if term.operators.is_empty() {
            // Identity term
            let mut m = vec![vec![Complex64::new(0.0, 0.0); dim]; dim];
            for i in 0..dim {
                m[i][i] = coeff;
            }
            return m;
        }

        // Build full tensor product matrix
        let mut full = vec![vec![Complex64::new(1.0, 0.0)]]; // 1×1 identity start

        for qubit in 0..self.num_qubits {
            let op = term
                .operators
                .iter()
                .find(|(q, _)| *q == qubit)
                .map(|(_, p)| *p)
                .unwrap_or(PauliOp::I);
            let pauli = pauli_matrix(op);
            full = tensor_product(&full, &pauli);
        }

        // Multiply by coefficient
        for row in &mut full {
            for val in row.iter_mut() {
                *val *= coeff;
            }
        }
        full
    }
}

fn pauli_matrix(op: PauliOp) -> Vec<Vec<Complex64>> {
    let zero = Complex64::new(0.0, 0.0);
    let one = Complex64::new(1.0, 0.0);
    let neg = Complex64::new(-1.0, 0.0);
    let pi = Complex64::new(0.0, 1.0);
    let ni = Complex64::new(0.0, -1.0);

    match op {
        PauliOp::I => vec![vec![one, zero], vec![zero, one]],
        PauliOp::X => vec![vec![zero, one], vec![one, zero]],
        PauliOp::Y => vec![vec![zero, ni], vec![pi, zero]],
        PauliOp::Z => vec![vec![one, zero], vec![zero, neg]],
    }
}

fn tensor_product(a: &[Vec<Complex64>], b: &[Vec<Complex64>]) -> Vec<Vec<Complex64>> {
    let ra = a.len();
    let ca = if ra > 0 { a[0].len() } else { 0 };
    let rb = b.len();
    let cb = if rb > 0 { b[0].len() } else { 0 };

    let rows = ra * rb;
    let cols = ca * cb;
    let mut result = vec![vec![Complex64::new(0.0, 0.0); cols]; rows];

    for i in 0..ra {
        for j in 0..ca {
            for k in 0..rb {
                for l in 0..cb {
                    result[i * rb + k][j * cb + l] = a[i][j] * b[k][l];
                }
            }
        }
    }
    result
}
