CREATE TABLE saved_sessions(
    id INTEGER NOT NULL PRIMARY KEY,
    events BYTEA NOT NULL,
    session_data BYTEA NOT NULL,
    participants BYTEA NOT NULL,
    session_history BYTEA NOT NULL,
    final_classification BYTEA NOT NULL,
    championship_id SERIAl NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
)