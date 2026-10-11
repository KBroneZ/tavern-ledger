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
- **Own rating (class found, path not mapped).** `NetCacheBaconRatingInfo` has two instance ints, `<Rating>k__BackingField` (offset 16) and `<DuosRating>k__BackingField` (offset 20). The game's `NetCache` keeps such objects in its `m_netCache` map, but `NetCache` has no static instance of its own (it is reached through a service locator in `Blizzard.T5.ServiceLocator`), and no class has a static field of type `NetCacheBaconRatingInfo`. `BaconLobbyDataModel` (the Battlegrounds screen's data model) also has an `m_Rating` int. **The fixed path to either is not mapped yet**; the reader returns "unsupported game version" for the rating until it is.

Race ids are the game's `TAG_RACE` numbers; the names are those of the `hearthstone` package (already pinned, `requirements.txt`), the same names the log writes as `CARDRACE`.

## Not done

1. Map the path to the own rating (through the service locator to `NetCache`, or to the Battlegrounds screen's data model) and compare it with the rating the screen shows. `ServiceManager` (in `Blizzard.T5.ServiceLocator`) holds the services in its statics `s_runtimeServices` (offset 0), `s_dynamicServices` (8) and `s_serviceObjects` (16, a `List`). The explorer's `rating` mode walks from there; it has not been run with the game open yet.
2. Record the result here with the build.

## Per build

| Build | Mono layout | Tribes path | Rating path | Checked on |
|---|---|---|---|---|
| 253216 | `LAYOUT_BDWGC_X64` (above) | `GameState.s_instance` → `m_availableRacesInBattlegroundsExcludingAmalgam`: matches the tavern on 2 games | not mapped | 2026-10-11, Duos, 2 games |
