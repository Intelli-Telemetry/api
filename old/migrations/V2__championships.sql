CREATE TYPE championship_category AS ENUM ('F1', 'F2');

CREATE TABLE
    championship (
        id SERIAL PRIMARY KEY,
        port INTEGER NOT NULL CHECK (
            port >= 0
            AND port <= 65535
        ),
        name VARCHAR(50) NOT NULL,
        category championship_category NOT NULL,
        season SMALLINT NOT NULL,
        driver_count SMALLINT DEFAULT 0 CHECK (
            driver_count >= 0
            AND driver_count <= 255
        ),
        owner_id INTEGER NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
    );