CREATE TYPE user_provider AS ENUM ('Local', 'Google');

CREATE TYPE user_role AS ENUM ('Free', 'Premium', 'Business', 'Admin');

CREATE TABLE
    users (
        id INTEGER NOT NULL PRIMARY KEY,
        email VARCHAR(100) NOT NULL UNIQUE,
        username VARCHAR(50) NOT NULL,
        password VARCHAR(255),
        avatar VARCHAR(100) NOT NULL,
        provider user_provider NOT NULL DEFAULT 'Local',
        role user_role NOT NULL DEFAULT 'Free',
        active BOOLEAN NOT NULL DEFAULT FALSE,
        created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
    );