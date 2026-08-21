"""
backend/tests/conftest.py — Pytest test harness configuration for Finance Tracker.

Provides:
  - Event loop & in-memory/isolated SQLite database connection for full state isolation.
  - FastAPI app dependency overrides for get_db.
  - httpx.AsyncClient fixture bound to app via ASGITransport.
  - Static IV AES-GCM crypto helpers matching backend/app/crypto_utils.py.
  - Per-test automatic cleanup to ensure zero state pollution between test runs.
"""

import base64
import pytest
import pytest_asyncio
import aiosqlite
from pathlib import Path
from typing import AsyncGenerator
from httpx import AsyncClient, ASGITransport

from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.ciphers.aead import AESGCM

from app.main import app
from app.database import get_db, init_db

STATIC_IV = b"jizifin-cryp"
SALT = b"jizifin-salt-pbkdf2"

# ---------------------------------------------------------------------------
# Crypto test helpers
# ---------------------------------------------------------------------------

def derive_key(password: str = "test-master-passphrase") -> bytes:
    kdf = PBKDF2HMAC(
        algorithm=hashes.SHA256(),
        length=32,
        salt=SALT,
        iterations=100000,
    )
    return kdf.derive(password.encode("utf-8"))

def encrypt_text(plaintext: str, key: bytes) -> str:
    if not plaintext:
        return plaintext
    aesgcm = AESGCM(key)
    ciphertext = aesgcm.encrypt(STATIC_IV, str(plaintext).encode("utf-8"), None)
    return base64.urlsafe_b64encode(ciphertext).decode("ascii").rstrip("=")

def decrypt_text(ciphertext: str, key: bytes) -> str:
    if not ciphertext:
        return ciphertext
    try:
        aesgcm = AESGCM(key)
        padded = ciphertext + "=" * ((4 - len(ciphertext) % 4) % 4)
        raw = base64.urlsafe_b64decode(padded)
        return aesgcm.decrypt(STATIC_IV, raw, None).decode("utf-8")
    except Exception:
        return ciphertext

# ---------------------------------------------------------------------------
# Isolated Database & Async Client Fixtures
# ---------------------------------------------------------------------------

import uuid

@pytest_asyncio.fixture
async def test_db(tmp_path: Path) -> AsyncGenerator[aiosqlite.Connection, None]:
    """Provision a fresh, isolated SQLite database file per test."""
    db_file = tmp_path / f"test_finance_{uuid.uuid4().hex}.db"
    async with aiosqlite.connect(db_file) as conn:
        await init_db(conn=conn)
        yield conn

@pytest_asyncio.fixture
async def client(test_db: aiosqlite.Connection) -> AsyncGenerator[AsyncClient, None]:
    """FastAPI AsyncClient dependency-overridden with test_db."""
    async def override_get_db():
        yield test_db

    app.dependency_overrides[get_db] = override_get_db
    app.state.testing = True
    transport = ASGITransport(app=app)
    async with AsyncClient(transport=transport, base_url="http://test") as ac:
        yield ac
    app.dependency_overrides.clear()
    app.state.testing = False
