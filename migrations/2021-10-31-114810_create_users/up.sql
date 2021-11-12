-- Storing user information
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    usr TEXT NOT NULL,
    pwd TEXT NOT NULL, -- Will be hashed
    lckdwn timestamp with time zone DEFAULT (now() at time zone 'utc') NOT NULL, --//TODO If this is a date in the future, this user should get 429 until then. 
    crt timestamp with time zone DEFAULT (now() at time zone 'utc') NOT NULL,
    last_accessed timestamp with time zone DEFAULT (now() at time zone 'utc') NOT NULL
);

-- Stores requests to/from this database. Used for rate limiting, and logging.
CREATE TABLE reqs (
    id SERIAL PRIMARY KEY,
    usr_id SERIAL NOT NULL,
    crt timestamp with time zone DEFAULT (now() at time zone 'utc') NOT NULL,
    word TEXT NOT NULL,
    lang TEXT NOT NULL,
    speed REAL NOT NULL,
    fmt TEXT NOT NULL,
    CONSTRAINT fk_users FOREIGN KEY(usr_id) REFERENCES users(id)
);

-- A smart-cache, useful for caching the most popular x requests so we don't have to regenerate them.
CREATE TABLE cache (
    id SERIAL PRIMARY KEY,
    crt timestamp with time zone DEFAULT (now() at time zone 'utc') NOT NULL,
    nme TEXT NOT NULL,
    word TEXT NOT NULL,
    lang TEXT NOT NULL
)