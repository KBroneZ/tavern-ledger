-- T-534 (D-103, amends D-082's minimum): every public player with at least
-- one finished game in the mode and period is on the boards.
--
-- The site has few players yet, and with 20 games per board (10 with the
-- hero) every board was empty. The minimum stays one value in one place, so
-- it can be raised again by replacing these two functions; the rebuild, the
-- reads and every other D-082 rule (public profiles with an allowed display
-- name only, the impossible-value exclusions, the one profile switch of D-089)
-- are unchanged. The website marks rows that rest on fewer than 20 games (10
-- with the hero) as "few games" instead of hiding them.
--
-- `create or replace` keeps each function's owner and privileges: no client
-- role can call them (revoked in 20261011150000_leaderboards.sql).

create or replace function private.leaderboard_min_games()
returns integer language sql immutable set search_path = '' as $$ select 1 $$;
create or replace function private.leaderboard_min_hero_games()
returns integer language sql immutable set search_path = '' as $$ select 1 $$;

comment on function public.leaderboard(text, text) is
  'Public leaderboards (T-506, T-534): every board of one mode (solo, duos) and period (7d, 30d, all), public profiles with an allowed display name and at least one finished game in it. Self-reported data.';
