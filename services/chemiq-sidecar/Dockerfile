FROM python:3.11-slim

WORKDIR /app

# Origin Quantum deps may need system libraries for compilation
RUN apt-get update && \
    apt-get install -y --no-install-recommends gcc g++ curl && \
    rm -rf /var/lib/apt/lists/*

COPY requirements.txt .

# Install core deps first (always succeed), then optional quantum deps
RUN pip install --no-cache-dir \
        fastapi uvicorn[standard] pydantic numpy && \
    pip install --no-cache-dir pychemiq pyqpanda3 pyqpanda_alg || \
        echo "WARNING: pyChemiQ/pyqpanda not installed — running in simulator mode"

COPY main.py .
COPY conftest.py test_sidecar.py ./

EXPOSE 8100

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -f http://localhost:8100/health || exit 1

CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8100"]
