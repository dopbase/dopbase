-- A protected environment, such as production, is reached only through a
-- grant on that environment or a project_manager grant. Project-level and
-- all-projects grants below project_manager stop at it.
ALTER TABLE environments ADD COLUMN protected INTEGER NOT NULL DEFAULT 0
  CHECK (protected IN (0, 1));
