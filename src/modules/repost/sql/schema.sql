-- lookup table for the ignored links on the room
CREATE TABLE IF NOT EXISTS ignored_links (
    `link` TEXT NOT NULL,
    `room` TEXT NOT NULL,
    UNIQUE(link, room)
);

-- full table of the links per room
CREATE TABLE IF NOT EXISTS links (
    `link` TEXT NOT NULL,
    `time` BLOB NOT NULL,
    `nick` TEXT NOT NULL,
    `room` TEXT NOT NULL,
    `posts` INTEGER,
    `ignored` BOOLEAN,
    UNIQUE(link, room)
)
