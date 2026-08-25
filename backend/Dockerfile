# Multi-stage build for FastAPI backend with Python 3.14
FROM python:3.14-slim AS builder

WORKDIR /app

# Copy uv binary directly from official image (instant, no apt-get/curl needed)
COPY --from=ghcr.io/astral-sh/uv:latest /uv /bin/uv

# Copy lock file and pyproject for layer caching
COPY uv.lock pyproject.toml ./

# Sync dependencies with BuildKit cache mount
RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --frozen --no-dev --no-editable

# ─────────────────────────────────────────────────────────────────────────────

FROM python:3.14-slim

WORKDIR /app

# Copy virtual environment from builder
COPY --from=builder /app/.venv .venv

# Copy application code
COPY app/ app/
COPY pyproject.toml ./

ENV PATH="/app/.venv/bin:$PATH" \
    PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8000/')" || exit 1

EXPOSE 8000

CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000"]
