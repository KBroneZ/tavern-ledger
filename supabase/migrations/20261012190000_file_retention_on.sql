-- Turn on the 28-day rolling retention of game files (T-544, D-109, D-113).
-- The user's OK on 2026-10-12, after a report-only run (mode "report", nothing due)
-- and the owner's exemption in private.unlimited_history.
update private.file_retention set live = true, updated_at = now();
