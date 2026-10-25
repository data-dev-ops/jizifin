-- Jizifin schema v1 (DuckDB 2.0).
-- Columns suffixed _ct hold deterministic AES-GCM ciphertext (Base64URL, no padding):
-- equality, IN and GROUP BY only; never ranges, LIKE or ORDER BY.
-- Referential integrity is enforced by the repository UnitOfWork until the spike
-- confirms DuckDB FK behaviour on updates of referenced rows.

-- Holds 'auth_proof_sha256': SHA-256 of the client-derived login proof (never the proof itself).
CREATE TABLE app_config (
    key   VARCHAR PRIMARY KEY,
    value VARCHAR NOT NULL
);

CREATE TABLE sessions (
    token_sha256 VARCHAR PRIMARY KEY,
    last_active  TIMESTAMP NOT NULL
);

CREATE SEQUENCE seq_members START 1;
CREATE TABLE members (
    id        BIGINT PRIMARY KEY DEFAULT nextval('seq_members'),
    name_ct   VARCHAR NOT NULL UNIQUE,
    color     VARCHAR NOT NULL DEFAULT '#64748b',
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE SEQUENCE seq_categories START 1;
CREATE TABLE categories (
    id       BIGINT PRIMARY KEY DEFAULT nextval('seq_categories'),
    kind     VARCHAR NOT NULL CHECK (kind IN ('expense', 'income')),
    name_ct  VARCHAR NOT NULL,
    pay_mode VARCHAR NOT NULL DEFAULT 'shared' CHECK (pay_mode IN ('shared', 'personal', 'income_ratio')),
    UNIQUE (kind, name_ct)
);

CREATE SEQUENCE seq_agreements START 1;
CREATE TABLE agreements (
    id          BIGINT PRIMARY KEY DEFAULT nextval('seq_agreements'),
    category_id BIGINT NOT NULL,
    valid_from  DATE NOT NULL,
    valid_to    DATE,
    note_ct     VARCHAR,
    CHECK (valid_to IS NULL OR valid_to >= valid_from)
);

CREATE TABLE agreement_shares (
    agreement_id BIGINT NOT NULL,
    member_id    BIGINT NOT NULL,
    pct          DOUBLE NOT NULL CHECK (pct >= 0),
    PRIMARY KEY (agreement_id, member_id)
);

CREATE SEQUENCE seq_accounts START 1;
CREATE TABLE accounts (
    id                BIGINT PRIMARY KEY DEFAULT nextval('seq_accounts'),
    name_ct           VARCHAR NOT NULL UNIQUE,
    safety_margin_pct DOUBLE NOT NULL DEFAULT 0,
    deposit_mode      VARCHAR NOT NULL DEFAULT 'equal' CHECK (deposit_mode IN ('equal', 'salary_ratio', 'fixed'))
);

CREATE TABLE account_members (
    account_id          BIGINT NOT NULL,
    member_id           BIGINT NOT NULL,
    fixed_deposit_cents BIGINT,
    due_day             INTEGER NOT NULL DEFAULT 1 CHECK (due_day BETWEEN 1 AND 31),
    PRIMARY KEY (account_id, member_id)
);

CREATE TABLE account_categories (
    account_id     BIGINT NOT NULL,
    category_id    BIGINT NOT NULL,
    expected_cents BIGINT NOT NULL DEFAULT 0 CHECK (expected_cents >= 0),
    PRIMARY KEY (account_id, category_id)
);

CREATE SEQUENCE seq_projects START 1;
CREATE TABLE projects (
    id           BIGINT PRIMARY KEY DEFAULT nextval('seq_projects'),
    name_ct      VARCHAR NOT NULL,
    target_cents BIGINT NOT NULL DEFAULT 0,
    color        VARCHAR NOT NULL DEFAULT '#64748b'
);

CREATE TABLE project_members (
    project_id BIGINT NOT NULL,
    member_id  BIGINT NOT NULL,
    PRIMARY KEY (project_id, member_id)
);

CREATE SEQUENCE seq_tags START 1;
CREATE TABLE tags (
    id             BIGINT PRIMARY KEY DEFAULT nextval('seq_tags'),
    name_ct        VARCHAR NOT NULL,
    description_ct VARCHAR,
    valid_from     DATE,
    valid_to       DATE,
    CHECK (valid_to IS NULL OR valid_from IS NULL OR valid_to >= valid_from)
);

CREATE SEQUENCE seq_recurring START 1;
CREATE TABLE recurring_templates (
    id           BIGINT PRIMARY KEY DEFAULT nextval('seq_recurring'),
    kind         VARCHAR NOT NULL CHECK (kind IN ('expense', 'income')),
    amount_cents BIGINT NOT NULL,
    member_id    BIGINT NOT NULL,
    category_id  BIGINT NOT NULL,
    account_id   BIGINT,
    day_of_month INTEGER NOT NULL CHECK (day_of_month BETWEEN 1 AND 31),
    frequency    VARCHAR NOT NULL CHECK (frequency IN ('monthly', 'weekly', 'biweekly', 'four_weekly', 'quarterly', 'annual')),
    valid_from   DATE NOT NULL,
    valid_to     DATE,
    name_ct      VARCHAR NOT NULL,
    is_active    BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE SEQUENCE seq_entries START 1;
CREATE TABLE entries (
    id           BIGINT PRIMARY KEY DEFAULT nextval('seq_entries'),
    kind         VARCHAR NOT NULL CHECK (kind IN ('expense', 'income', 'deposit', 'correction')),
    entry_date   DATE NOT NULL,
    amount_cents BIGINT NOT NULL,
    member_id    BIGINT,
    category_id  BIGINT,
    account_id   BIGINT,
    project_id   BIGINT,
    tag_id       BIGINT,
    recurring_id BIGINT,
    name_ct      VARCHAR,
    note_ct      VARCHAR,
    created_at   TIMESTAMP NOT NULL DEFAULT current_timestamp,
    CHECK (kind NOT IN ('expense', 'income') OR (member_id IS NOT NULL AND category_id IS NOT NULL)),
    CHECK (kind NOT IN ('deposit', 'correction') OR account_id IS NOT NULL)
);
CREATE INDEX idx_entries_date ON entries (entry_date, id);
CREATE INDEX idx_entries_member_date ON entries (member_id, entry_date);
CREATE INDEX idx_entries_category_date ON entries (category_id, entry_date);
CREATE INDEX idx_entries_account_date ON entries (account_id, entry_date);
CREATE INDEX idx_entries_project ON entries (project_id);
-- One generated entry per template occurrence: makes the recurring catch-up idempotent.
CREATE UNIQUE INDEX uq_entries_recurring_date ON entries (recurring_id, entry_date);

CREATE TABLE entry_share_overrides (
    entry_id  BIGINT NOT NULL,
    member_id BIGINT NOT NULL,
    pct       DOUBLE NOT NULL CHECK (pct >= 0),
    PRIMARY KEY (entry_id, member_id)
);

CREATE SEQUENCE seq_jobs START 1;
CREATE TABLE jobs (
    id           BIGINT PRIMARY KEY DEFAULT nextval('seq_jobs'),
    member_id    BIGINT NOT NULL,
    name_ct      VARCHAR NOT NULL,
    amount_cents BIGINT NOT NULL CHECK (amount_cents >= 0),
    frequency    VARCHAR NOT NULL CHECK (frequency IN ('monthly', 'weekly', 'biweekly', 'annual')),
    valid_from   DATE NOT NULL,
    valid_to     DATE,
    notes_ct     VARCHAR,
    CHECK (valid_to IS NULL OR valid_to >= valid_from)
);

CREATE TABLE income_overrides (
    member_id    BIGINT NOT NULL,
    month        DATE NOT NULL CHECK (day(month) = 1),
    amount_cents BIGINT NOT NULL,
    note_ct      VARCHAR,
    PRIMARY KEY (member_id, month)
);

CREATE TABLE budgets (
    category_id BIGINT NOT NULL,
    month       DATE NOT NULL CHECK (day(month) = 1),
    limit_cents BIGINT NOT NULL CHECK (limit_cents >= 0),
    source      VARCHAR NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'accepted_proposal')),
    PRIMARY KEY (category_id, month)
);

CREATE SEQUENCE seq_offset_rules START 1;
CREATE TABLE offset_rules (
    id                 BIGINT PRIMARY KEY DEFAULT nextval('seq_offset_rules'),
    credit_member_id   BIGINT NOT NULL,
    credit_category_id BIGINT NOT NULL,
    debit_member_id    BIGINT NOT NULL,
    debit_category_id  BIGINT NOT NULL,
    is_active          BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE month_seals (
    month       DATE PRIMARY KEY CHECK (day(month) = 1),
    result_json JSON NOT NULL,
    inputs_hash VARCHAR NOT NULL,
    sealed_at   TIMESTAMP NOT NULL DEFAULT current_timestamp
);
