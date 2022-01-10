-- Storing user information
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    usr TEXT NOT NULL,
    pwd TEXT NOT NULL, -- Will be hashed
    lckdwn timestamp with time zone DEFAULT (now() at time zone 'utc') NOT NULL,
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

-- Create relevant indexes
CREATE INDEX user_id_index ON users (id);
CREATE INDEX user_usr_index ON users (usr);
CREATE INDEX req_crt_index ON reqs (crt);
CREATE INDEX req_id_index ON reqs (id);