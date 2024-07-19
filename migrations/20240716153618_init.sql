CREATE TABLE reminders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message TEXT NOT NULL DEFAULT "something",
    timestamp INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    message_id INTEGER NOT NULL,
    active BOOLEAN NOT NULL DEFAULT 1
);
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    discord_id INTEGER NOT NULL UNIQUE,
    utc_offset INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX user_index ON users (discord_id);
CREATE TABLE channels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    discord_id INTEGER NOT NULL UNIQUE
);
CREATE INDEX channel_index ON channels (discord_id);
CREATE TABLE reminder_user (
    reminder_id INTEGER,
    user_id INTEGER,
    PRIMARY KEY (reminder_id, user_id),
    FOREIGN KEY (reminder_id) REFERENCES reminders(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE TABLE reminder_channel (
    reminder_id INTEGER PRIMARY KEY,
    channel_id INTEGER,
    FOREIGN KEY (reminder_id) REFERENCES reminders(id),
    FOREIGN KEY (channel_id) REFERENCES channels(id)
);
