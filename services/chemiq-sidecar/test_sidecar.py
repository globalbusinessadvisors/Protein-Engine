"""Tests for the chemiq-sidecar FastAPI service.

These tests run against the numpy simulator fallback, which is always available.
When pyChemiQ is installed, the same API contract holds with higher accuracy.
"""

import numpy as np
import pytest


# ── Health endpoint ────────────────────────────────────────────────────


class TestHealth:
    def test_health_returns_200(self, client):
        resp = client.get("/health")
        assert resp.status_code == 200

    def test_health_status_ok(self, client):
        data = client.get("/health").json()
        assert data["status"] == "ok"

    def test_health_has_backend_field(self, client):
        data = client.get("/health").json()
        assert data["backend"] in ("origin_quantum", "simulator")


# ── Capabilities endpoint ──────────────────────────────────────────────


class TestCapabilities:
    def test_capabilities_returns_200(self, client):
        resp = client.get("/capabilities")
        assert resp.status_code == 200

    def test_capabilities_max_qubits(self, client):
        data = client.get("/capabilities").json()
        assert isinstance(data["max_qubits"], int)
        assert data["max_qubits"] > 0

    def test_capabilities_has_ansatze(self, client):
        data = client.get("/capabilities").json()
        assert "UCCSD" in data["available_ansatze"]

    def test_capabilities_has_backend(self, client):
        data = client.get("/capabilities").json()
        assert data["backend"] in ("origin_quantum", "simulator")


# ── VQE endpoint ───────────────────────────────────────────────────────


class TestVqe:
    def test_vqe_h2_returns_200(self, client):
        resp = client.post("/vqe", json={"molecule": "H2"})
        assert resp.status_code == 200

    def test_vqe_h2_energy_within_tolerance(self, client):
        """H2 ground state energy ≈ -1.137 Hartree."""
        data = client.post("/vqe", json={"molecule": "H2"}).json()
        energy = data["energy"]
        # Simulator may not hit exact value, allow generous tolerance
        assert energy < 0.0, f"Expected negative energy, got {energy}"
        assert abs(energy - (-1.137)) < 1.0, (
            f"Energy {energy} too far from expected -1.137 Ha"
        )

    def test_vqe_h2_has_parameters(self, client):
        data = client.post("/vqe", json={"molecule": "H2"}).json()
        assert isinstance(data["parameters"], list)

    def test_vqe_h2_has_iterations(self, client):
        data = client.post("/vqe", json={"molecule": "H2"}).json()
        assert isinstance(data["iterations"], int)
        assert data["iterations"] >= 1

    def test_vqe_h2_converged(self, client):
        data = client.post("/vqe", json={"molecule": "H2"}).json()
        assert isinstance(data["converged"], bool)

    def test_vqe_h2_descriptor_format(self, client):
        """Rust bridge sends molecule as '2q-6t' for H2."""
        resp = client.post("/vqe", json={"molecule": "2q-6t"})
        assert resp.status_code == 200
        data = resp.json()
        assert data["energy"] < 0.0

    def test_vqe_hydrogen_alias(self, client):
        resp = client.post("/vqe", json={"molecule": "hydrogen"})
        assert resp.status_code == 200

    def test_vqe_custom_parameters(self, client):
        resp = client.post(
            "/vqe",
            json={
                "molecule": "H2",
                "basis_set": "sto-3g",
                "ansatz": "UCCSD",
                "max_iterations": 50,
            },
        )
        assert resp.status_code == 200

    def test_vqe_invalid_molecule_returns_400(self, client):
        resp = client.post("/vqe", json={"molecule": "XYZ_INVALID"})
        assert resp.status_code == 400

    def test_vqe_empty_molecule_returns_422(self, client):
        resp = client.post("/vqe", json={"molecule": ""})
        assert resp.status_code == 422

    def test_vqe_missing_molecule_returns_422(self, client):
        resp = client.post("/vqe", json={})
        assert resp.status_code == 422

    def test_vqe_invalid_basis_set_returns_422(self, client):
        resp = client.post(
            "/vqe", json={"molecule": "H2", "basis_set": "not-a-basis"}
        )
        assert resp.status_code == 422

    def test_vqe_invalid_ansatz_returns_422(self, client):
        resp = client.post(
            "/vqe", json={"molecule": "H2", "ansatz": "INVALID"}
        )
        assert resp.status_code == 422

    def test_vqe_max_iterations_zero_returns_422(self, client):
        resp = client.post(
            "/vqe", json={"molecule": "H2", "max_iterations": 0}
        )
        assert resp.status_code == 422


# ── QAOA endpoint ──────────────────────────────────────────────────────


class TestQaoa:
    def test_qaoa_trivial_qubo_returns_200(self, client):
        """Trivial 2-variable QUBO: minimize x0 + x1."""
        qubo = [[1.0, 0.0], [0.0, 1.0]]
        resp = client.post("/qaoa", json={"qubo_matrix": qubo, "p_layers": 1})
        assert resp.status_code == 200

    def test_qaoa_trivial_qubo_optimal_solution(self, client):
        """For diagonal QUBO [[1,0],[0,1]], optimal is all zeros (cost=0)."""
        qubo = [[1.0, 0.0], [0.0, 1.0]]
        data = client.post(
            "/qaoa", json={"qubo_matrix": qubo, "p_layers": 1}
        ).json()
        assert data["solution"] == [0, 0]
        assert data["cost"] == 0.0

    def test_qaoa_returns_correct_fields(self, client):
        qubo = [[1.0, -2.0], [-2.0, 1.0]]
        data = client.post(
            "/qaoa", json={"qubo_matrix": qubo, "p_layers": 1}
        ).json()
        assert "solution" in data
        assert "cost" in data
        assert "iterations" in data
        assert isinstance(data["solution"], list)
        assert isinstance(data["cost"], float)
        assert isinstance(data["iterations"], int)

    def test_qaoa_solution_is_binary(self, client):
        qubo = [[-1.0, 0.5], [0.5, -1.0]]
        data = client.post(
            "/qaoa", json={"qubo_matrix": qubo, "p_layers": 1}
        ).json()
        for bit in data["solution"]:
            assert bit in (0, 1)

    def test_qaoa_negative_diagonal_prefers_ones(self, client):
        """QUBO [[-1,0],[0,-1]]: both variables want to be 1 (cost=-2)."""
        qubo = [[-1.0, 0.0], [0.0, -1.0]]
        data = client.post(
            "/qaoa", json={"qubo_matrix": qubo, "p_layers": 1}
        ).json()
        assert data["solution"] == [1, 1]
        assert abs(data["cost"] - (-2.0)) < 1e-6

    def test_qaoa_non_square_matrix_returns_422(self, client):
        qubo = [[1.0, 0.0], [0.0]]
        resp = client.post("/qaoa", json={"qubo_matrix": qubo, "p_layers": 1})
        assert resp.status_code == 422

    def test_qaoa_empty_matrix_returns_422(self, client):
        resp = client.post("/qaoa", json={"qubo_matrix": [], "p_layers": 1})
        assert resp.status_code == 422

    def test_qaoa_zero_layers_returns_422(self, client):
        qubo = [[1.0]]
        resp = client.post("/qaoa", json={"qubo_matrix": qubo, "p_layers": 0})
        assert resp.status_code == 422


# ── Response schema validation ─────────────────────────────────────────


class TestSchemaCompat:
    """Verify responses match the Rust bridge DTO contract."""

    def test_vqe_response_matches_rust_dto(self, client):
        data = client.post("/vqe", json={"molecule": "H2"}).json()
        # ChemiqVqeResponse: energy, parameters, iterations, converged
        assert "energy" in data
        assert "parameters" in data
        assert "iterations" in data
        assert "converged" in data
        assert isinstance(data["energy"], float)
        assert isinstance(data["parameters"], list)
        assert isinstance(data["iterations"], int)
        assert isinstance(data["converged"], bool)

    def test_qaoa_response_matches_rust_dto(self, client):
        qubo = [[1.0, 0.0], [0.0, 1.0]]
        data = client.post(
            "/qaoa", json={"qubo_matrix": qubo, "p_layers": 1}
        ).json()
        # ChemiqQaoaResponse: solution, cost, iterations
        assert "solution" in data
        assert "cost" in data
        assert "iterations" in data
        assert isinstance(data["solution"], list)
        assert all(isinstance(x, int) for x in data["solution"])
        assert isinstance(data["cost"], (int, float))
        assert isinstance(data["iterations"], int)

    def test_health_response_matches_rust_dto(self, client):
        data = client.get("/health").json()
        # ChemiqHealthResponse: status, backend
        assert "status" in data
        assert "backend" in data

    def test_capabilities_response_matches_rust_dto(self, client):
        data = client.get("/capabilities").json()
        # ChemiqCapabilitiesResponse: max_qubits, available_ansatze, backend
        assert "max_qubits" in data
        assert "available_ansatze" in data
        assert "backend" in data
