# Memory reader: how the fields were found (T-327, D-063)

Session 055, 2026-10-11. Game build 253216, `mono-2.0-bdwgc.dll` from the game's `MonoBleedingEdge\EmbedRuntime` folder (64-bit). Design and decision: [lobby-tribes-exact.md](lobby-tribes-exact.md) sections 4 and 6.

Clean room: no code, offsets or structure layouts from HearthMirror, HDT, HSTracker, Firestone/MindVision, BobsBuddy or unity-spy were read or used; Nomi's Kitchen was not looked at. No decompiler, disassembler or game file was used. Everything below comes from Mono's public structures and from names Mono keeps at run time in the user's own client.

## How the walk was found

A scratch explorer (kept outside the repo) used the crate's own read-only process access (`PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ`). It printed only offsets, assembly, class and field names, type codes, counts and small integers; never a string from a game object, never a player name or BattleTag.

1. **Root domain.** Mono exports `mono_get_root_domain`, which only returns a global. Its first bytes in the game are `48 8b 05 <disp32> c3` (`mov rax, [rip + disp32]; ret`); the global is at `function + 7 + disp32`. The reader checks those bytes every time; other bytes mean an unknown runtime.
2. **Assemblies.** At each pointer-sized offset of the domain, the explorer tried "list node → assembly → name pointer" until a name read back as an assembly name: the list of loaded assemblies (`domain_assemblies`) is at `domain + 0xa0`, a singly linked list (`data`, `next`); each assembly's name is at `assembly + 0x10` (119 assemblies loaded, the first `mscorlib`).
3. **Image.** The pointer in the assembly whose target holds the same name at a small offset: `assembly + 0x60` → image, whose `assembly_name` is at `image + 0x30`.
4. **Classes.** A hash table in the image shaped like Mono's internal hash table (three function pointers, a size, an entry count, a table pointer) at `image + 0x4d0`: size at `+0x18`, table at `+0x20` (`Assembly-CSharp`: 6 247 buckets, 14 095 classes). In each class: image back pointer at `+0x40`, name at `+0x48`, namespace at `+0x50`; the chain to the next class in the bucket at `+0x108` (the offset most often pointing at another class of the same image: 200 of 200 sampled).
5. **Fields.** Checked on `mscorlib`'s `System.String`, whose fields are public knowledge: the field array is at `class + 0x98`, entries of `0x20` bytes (type, name, parent, offset), `_stringLength` at offset 16 and `_firstChar` at 20 as expected; the field count at `class + 0x100` (7, matching the 7 entries). Each type keeps its attributes in the low 16 bits at `type + 8` (static `0x10`, literal `0x40`); parent class pointer at `class + 0x30` (`Object` for the game's classes).
6. **Statics.** The runtime info at `class + 0xd0` holds the root domain's vtable at `+8`; the vtable points back to its class at `+0`. The static data pointer sits right after the method slots: `vtable + 0x48 + 8 × vtable_size`, `vtable_size` at `class + 0x5c`. Checked on `String.Empty`, which points at a string object whose vtable is `String`'s.
7. **Lists.** `List<T>`: items array at `+0x10`, count at `+0x18`; the array's length at `+0x18`, elements from `+0x20`.

The reader (`crates/game-memory/src/mono.rs`, `LAYOUT_BDWGC_X64`) uses these offsets, and checks a back pointer or a name at each step, so a different layout fails instead of reading the wrong thing.

## What the game keeps

- **Lobby tribes (verified on 2 games).** `GameState` (global namespace) has a static `s_instance` and the instance fields `m_availableRacesInBattlegroundsExcludingAmalgam`, `m_missingRacesInBattlegrounds` and `m_inactiveRacesInBattlegrounds`, each a `List` of race ids. The first holds the lobby's five tribes; the second the five left out (together the ten tribes of this season; Naga is in neither); the third was empty every time. Checked against the tavern-confirmed lobby (D-050: single-tribe offers, with the card data) of the same game:
  - Session `Hearthstone_2026_10_11_03_26_41`, game 1 (Duos): read at hero select and again mid-game, both `[126, 15, 24, 18, 17]` (Aberration, Demon, Dragon, Elemental, Mechanical). The tavern confirmed exactly those five.
  - Session `Hearthstone_2026_10_11_01_10_48` (Duos): one mid-game read, `[20, 24, 14, 11, 23]` (Beast, Dragon, Murloc, Undead, Pirate). It was first matched to game 2 by the time of the read and looked like a failure (game 2's tavern confirmed Aberration, Elemental, Mechanical, Pirate, Undead). Game 3 of the same session confirmed exactly the read's five (a 1 in 252 chance for a random lobby), so the read fell in game 3; game 2 and 3 were saved within three minutes of each other, which is how the read was matched to the wrong game.

  The reader still checks every read (five distinct known tribes, none also in the "left out" list) and the tracker drops it if the tavern later contradicts it (`GameTribes::checked_against`), showing "Lobby tribes: unsupported game version" and the inferred tribes instead.
- **Own rating (mapped and read in session 060, T-337).** `NetCacheBaconRatingInfo` has two instance ints, `<Rating>k__BackingField` (offset 16, Solo) and `<DuosRating>k__BackingField` (offset 20, Duos). No class has a static field of that type and `NetCache` has no static instance; the path goes through the game's service locator. Found with the scratch explorer's `rating` mode (a bounded breadth-first walk from `ServiceManager`'s statics that prints only class, namespace and field names, offsets and the two numbers), run by the user on the Battlegrounds lobby screen:
  1. Assembly `Blizzard.T5.ServiceLocator`, class `Blizzard.T5.Services.ServiceManager`, static `s_runtimeServices` (offset 0): an object of class `Blizzard.T5.Services.ServiceLocator`.
  2. `ServiceLocator.m_services` (offset 0x20): a `System.Collections.Generic.Dictionary`2` (`_entries` at 0x18, an array; `_count` at 0x40).
  3. Each entry is 24 bytes (hash code and next index, 4 bytes each, then key and value, 8 bytes each), from the array's offset 0x20; the value is a `ServiceInfo` (global namespace). The one whose `<Service>k__BackingField` (offset 0x20) is an object of class `NetCache` (entry 23 of the list on this client).
  4. `NetCache.m_netCache` (offset 0x10): a `Blizzard.T5.Core.Map`2` (Blizzard's own hash map: `valueSlots` at 0x28, an `Object[]`; `touchedSlots` at 0x38, the slots used so far).
  5. The one value in `valueSlots` of class `NetCacheBaconRatingInfo` (slot 18 on this client).

  The explorer read Solo 6651 and Duos 7116 there; after the user's next game it read Duos 7197, the number the lobby screen showed, and the shipped reader (`game-memory-check`) read the same Solo 6651 and Duos 7197. The user gave only the Duos number on screen.
  6. **Generic classes.** `Dictionary`2` and `Map`2` are generic instances. Such a class keeps no field count of its own (the value at `class + 0x100` is not a count; the first reader check failed safe there with "field count out of range"): at `class + 0xf0`, where a class definition keeps its flags, is its `MonoGenericClass`, whose first pointer is the generic definition, with the same name and the field count (10 for `Dictionary`2`, 14 for `Map`2`). The instance's own field array at `class + 0x98` holds that many fields, each pointing back to the instance; the arrays are `Entry[]` (entries) and `Object[]` (values), all checked on the user's client. The reader (`game::read_own_rating`) follows the same path by name, offsets from Mono's metadata, and checks the class of every object on it: the locator and the cache by class pointer (`ServiceLocator` in `Blizzard.T5.ServiceLocator`, `NetCache` and `NetCacheBaconRatingInfo` in `Assembly-CSharp`), the dictionary, the entries (`ServiceInfo`), the map and its arrays by class name; counts must not be negative or longer than their array, and only one `NetCache` and one rating object may be found. A service not made yet, or a cache with no rating object yet, is "not yet", never a value. The app's own checks are in "What the reader checks" below.
- **The client's build before a game.** The rating is read on the lobby screen, often before the session's first game, so `Power.log` has no build yet. The client writes its version once at start in `Net.log` (when `[Net]` is on in `log.config`), as part of a user agent: `Hearthstone/36.6.253216 (PC;…)`; the app keeps only the third part, the build, and never anything else from that line (`tracker::client_build`). A game's own build line wins; if the two differ the build is unknown, and an unknown build is not read.

Race ids are the game's `TAG_RACE` numbers; the names are those of the `hearthstone` package (already pinned, `requirements.txt`), the same names the log writes as `CARDRACE`.

## What the reader checks (and what it cannot)

- Only on a game build in `CHECKED_BUILDS` (`crates/tracker/src/game_values.rs`); any other build is not read at all and shows "unsupported game version" until the paths are checked on it and the build is added.
- At every step a back pointer or a name: class → image, field → class, vtable → class, the code bytes of `mono_get_root_domain`, the export's ordinal within the function table.
- Types: the object in `s_instance` must be of the `GameState` class just found (same pointer); both lists must be `List`1` objects whose items are an array (a class named `…[]`).
- Values: five distinct known tribes in the lobby, a non-empty list of distinct known tribes left out, none in both.
- Time: two reads in a row (three seconds apart) must agree, and must not be the five of the game before (the game may still hold them as a game starts). The tavern contradicting the tribes (D-050) drops them for the rest of the game.
- Rating: only on the Battlegrounds lobby screen and only when the running client's build (above) is in `CHECKED_BUILDS`; a read every three seconds, at most 20 per visit to the screen; taken when two reads in a row agree; each value between 1 and 30 000 (0 is "none shown"); a failed check drops the rating for that visit. The change since the last game is the new rating minus the one taken before that game, in that game's mode, only when exactly one game finished between the two and the value moved (D-069).
- **Not checked:** the size of each list element (4 bytes, `int` or an `int` enum: seen as such on build 253216, not read from the array's class) and whether `GameState` is replaced between games (one read at hero select after a different lobby was fresh). Both are reasons the build list is strict: a new build is only added after it is checked on real games.

## Not done

1. Check the rating path again on the next game build before adding it to `CHECKED_BUILDS`, as for the tribes.
2. How soon after a game the client updates the rating on the lobby screen was not measured; the app keeps reading for about a minute per visit and takes the newest value two reads agree on.

## Per build

| Build | Mono layout | Tribes path | Rating path | Checked on |
|---|---|---|---|---|
| 253216 | `LAYOUT_BDWGC_X64` (above) | `GameState.s_instance` → `m_availableRacesInBattlegroundsExcludingAmalgam`: matches the tavern on 2 games | `ServiceManager.s_runtimeServices` → `m_services` → `ServiceInfo` of `NetCache` → `m_netCache` → `NetCacheBaconRatingInfo`: the shipped reader (`game-memory-check`) read Solo 6651, Duos 7197 on the lobby screen, Duos equal to the screen (and to the explorer) | tribes 2026-10-11, Duos, 2 games; rating 2026-10-11, lobby screen |
