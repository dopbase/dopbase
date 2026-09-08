CREATE TABLE environment_id_reservations (
  id TEXT PRIMARY KEY
);

CREATE TABLE environment_id_sequence (
  id INTEGER PRIMARY KEY CHECK(id = 1),
  next_number INTEGER NOT NULL CHECK(next_number BETWEEN 1000 AND 1000000)
);

INSERT INTO environment_id_sequence(id, next_number) VALUES(1, 1000);

INSERT OR IGNORE INTO environment_id_reservations(id)
SELECT id FROM environments;

INSERT OR IGNORE INTO environment_id_reservations(id)
SELECT environment_id
FROM audit_events
WHERE environment_id IS NOT NULL AND environment_id <> '';

UPDATE environment_id_sequence
SET next_number = MIN(
  1000000,
  MAX(
    1000,
    COALESCE((
      SELECT MAX(CAST(substr(id, 5) AS INTEGER)) + 1
      FROM environment_id_reservations
      WHERE id GLOB 'env_[0-9]*'
        AND length(id) BETWEEN 8 AND 10
        AND substr(id, 5) NOT GLOB '*[^0-9]*'
    ), 1000)
  )
)
WHERE id = 1;
