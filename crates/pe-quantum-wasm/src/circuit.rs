use crate::error::QuantumError;
use crate::statevector::StateVector;
use crate::types::{GateType, MAX_QUBITS};

/// A single gate application in a quantum circuit.
#[derive(Debug, Clone)]
pub struct GateApplication {
    pub gate: GateType,
    pub target: usize,
    pub control: Option<usize>,
    pub parameter: Option<f64>,
}

/// An ordered sequence of gate applications on a fixed-size qubit register.
#[derive(Debug, Clone)]
pub struct QuantumCircuit {
    num_qubits: usize,
    gates: Vec<GateApplication>,
}

impl QuantumCircuit {
    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    pub fn gates(&self) -> &[GateApplication] {
        &self.gates
    }

    /// Execute this circuit, producing the final statevector.
    pub fn execute(&self) -> Result<StateVector, QuantumError> {
        let mut sv = StateVector::new(self.num_qubits)?;
        self.apply_to(&mut sv)?;
        Ok(sv)
    }

    /// Apply this circuit to an existing statevector.
    pub fn apply_to(&self, sv: &mut StateVector) -> Result<(), QuantumError> {
        for g in &self.gates {
            match g.gate {
                GateType::H => sv.h(g.target)?,
                GateType::X => sv.x(g.target)?,
                GateType::Y => sv.y(g.target)?,
                GateType::Z => sv.z(g.target)?,
                GateType::Rx => sv.rx(g.target, g.parameter.unwrap_or(0.0))?,
                GateType::Ry => sv.ry(g.target, g.parameter.unwrap_or(0.0))?,
                GateType::Rz => sv.rz(g.target, g.parameter.unwrap_or(0.0))?,
                GateType::Cnot => {
                    sv.cnot(g.control.unwrap_or(0), g.target)?;
                }
                GateType::Cz => {
                    sv.cz(g.control.unwrap_or(0), g.target)?;
                }
            }
        }
        Ok(())
    }
}

/// Fluent builder for constructing quantum circuits.
#[derive(Debug)]
pub struct CircuitBuilder {
    num_qubits: usize,
    gates: Vec<GateApplication>,
}

impl CircuitBuilder {
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
        Ok(Self {
            num_qubits,
            gates: Vec::new(),
        })
    }

    pub fn h(mut self, qubit: usize) -> Result<Self, QuantumError> {
        self.check_qubit(qubit)?;
        self.gates.push(GateApplication {
            gate: GateType::H,
            target: qubit,
            control: None,
            parameter: None,
        });
        Ok(self)
    }

    pub fn x(mut self, qubit: usize) -> Result<Self, QuantumError> {
        self.check_qubit(qubit)?;
        self.gates.push(GateApplication {
            gate: GateType::X,
            target: qubit,
            control: None,
            parameter: None,
        });
        Ok(self)
    }

    pub fn y(mut self, qubit: usize) -> Result<Self, QuantumError> {
        self.check_qubit(qubit)?;
        self.gates.push(GateApplication {
            gate: GateType::Y,
            target: qubit,
            control: None,
            parameter: None,
        });
        Ok(self)
    }

    pub fn z(mut self, qubit: usize) -> Result<Self, QuantumError> {
        self.check_qubit(qubit)?;
        self.gates.push(GateApplication {
            gate: GateType::Z,
            target: qubit,
            control: None,
            parameter: None,
        });
        Ok(self)
    }

    pub fn rx(mut self, qubit: usize, theta: f64) -> Result<Self, QuantumError> {
        self.check_qubit(qubit)?;
        self.gates.push(GateApplication {
            gate: GateType::Rx,
            target: qubit,
            control: None,
            parameter: Some(theta),
        });
        Ok(self)
    }

    pub fn ry(mut self, qubit: usize, theta: f64) -> Result<Self, QuantumError> {
        self.check_qubit(qubit)?;
        self.gates.push(GateApplication {
            gate: GateType::Ry,
            target: qubit,
            control: None,
            parameter: Some(theta),
        });
        Ok(self)
    }

    pub fn rz(mut self, qubit: usize, theta: f64) -> Result<Self, QuantumError> {
        self.check_qubit(qubit)?;
        self.gates.push(GateApplication {
            gate: GateType::Rz,
            target: qubit,
            control: None,
            parameter: Some(theta),
        });
        Ok(self)
    }

    pub fn cnot(mut self, control: usize, target: usize) -> Result<Self, QuantumError> {
        self.check_qubit(control)?;
        self.check_qubit(target)?;
        if control == target {
            return Err(QuantumError::SameControlTarget(control));
        }
        self.gates.push(GateApplication {
            gate: GateType::Cnot,
            target,
            control: Some(control),
            parameter: None,
        });
        Ok(self)
    }

    pub fn cz(mut self, control: usize, target: usize) -> Result<Self, QuantumError> {
        self.check_qubit(control)?;
        self.check_qubit(target)?;
        if control == target {
            return Err(QuantumError::SameControlTarget(control));
        }
        self.gates.push(GateApplication {
            gate: GateType::Cz,
            target,
            control: Some(control),
            parameter: None,
        });
        Ok(self)
    }

    pub fn build(self) -> QuantumCircuit {
        QuantumCircuit {
            num_qubits: self.num_qubits,
            gates: self.gates,
        }
    }

    fn check_qubit(&self, qubit: usize) -> Result<(), QuantumError> {
        if qubit >= self.num_qubits {
            Err(QuantumError::QubitOutOfRange(qubit, self.num_qubits))
        } else {
            Ok(())
        }
    }
}
