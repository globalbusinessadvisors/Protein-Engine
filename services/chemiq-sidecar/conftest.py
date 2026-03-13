"""Pytest configuration for chemiq-sidecar tests."""

import pytest
from fastapi.testclient import TestClient

from main import app


@pytest.fixture
def client():
    """Create a FastAPI test client."""
    return TestClient(app)
