import base64
import sqlite3
from pathlib import Path

from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.ciphers.aead import AESGCM

STATIC_IV = b"jizifin-cryp"
SALT = b"jizifin-salt-pbkdf2"

def derive_key(password: str) -> bytes:
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
    # Use urlsafe base64 and strip padding to match frontend
    return base64.urlsafe_b64encode(ciphertext).decode("ascii").rstrip("=")

def decrypt_text(ciphertext: str, key: bytes) -> str:
    if not ciphertext:
        return ciphertext
    try:
        aesgcm = AESGCM(key)
        # Re-add padding required by python's base64 decoder
        padded = ciphertext + "=" * ((4 - len(ciphertext) % 4) % 4)
        raw = base64.urlsafe_b64decode(padded)
        plaintext = aesgcm.decrypt(STATIC_IV, raw, None)
        return plaintext.decode("utf-8")
    except Exception:
        return ciphertext

def process_database(db_path: Path, key: bytes, encrypt: bool) -> None:
    """
    Iterates over all tables and columns that contain sensitive data,
    and encrypts or decrypts them in place using the provided key.
    Temporarily disables foreign keys to allow mass updates.
    """
    # Mapping of tables to their sensitive columns
    target_columns = {
        "users": ["name"],
        "splits": ["category"],
        "income_categories": ["category"],
        "projects": ["name"],
        "expenses": ["name", "who_paid", "category"],
        "expense_overrides": ["user_name"],
        "income": ["name", "who", "category"],
        "recurring_expenses": ["name", "who_paid", "category"],
        "budgets": ["category"],
        "split_allocations": ["category", "user_name"],
        "tags": ["name", "description"]
    }
    
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    
    try:
        # Disable foreign keys during the mass update
        conn.execute("PRAGMA foreign_keys=OFF;")
        
        for table, columns in target_columns.items():
            # Check if table exists
            cur = conn.execute("SELECT name FROM sqlite_master WHERE type='table' AND name=?", (table,))
            if not cur.fetchone():
                continue
            
            # Fetch all rows
            rows = conn.execute(f"SELECT rowid as _rowid, * FROM {table}").fetchall()
            
            for row in rows:
                updates = []
                params = []
                for col in columns:
                    val = row[col]
                    if val is not None:
                        new_val = encrypt_text(val, key) if encrypt else decrypt_text(val, key)
                        updates.append(f"{col} = ?")
                        params.append(new_val)
                
                if updates:
                    params.append(row["_rowid"])
                    query = f"UPDATE {table} SET {', '.join(updates)} WHERE rowid = ?"
                    conn.execute(query, params)
                    
        conn.commit()
    finally:
        conn.execute("PRAGMA foreign_keys=ON;")
        conn.close()

def encrypt_database(db_path: Path, key: bytes) -> None:
    process_database(db_path, key, encrypt=True)

def decrypt_database(db_path: Path, key: bytes) -> None:
    process_database(db_path, key, encrypt=False)
