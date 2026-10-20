# Battle command and spell eligibility refresh

## Evidence and mapping

The native `src/battle_menu_state.rs` translates UpdateCmdList C2/527D..532B
(file offsets 0x2527D..0x2532B), UpdateEnabledMagic C2/5763..57A9 and
CheckMagicEnabled C2/57AA..57C1 (offsets 0x25763..0x257C1).
Labels and structure come from the pinned disassembly's battle--party.asm.
These code regions are byte-identical between clean and target ROMs.
Confidence is high for the bounded routines and supplied record layout.

## Behavior and layout

Four command records contain ID, flags and cursor targeting bytes. Refresh
sets the character's changed bit. It rotates the existing flags right and
inserts the disabled result in bit 7. Imp checks command properties;
Silence blocks Magic, Lore and X-Magic. Bushido and Runic check weapon
capabilities. Morph needs a counter above 15. Offering changes Fight/Capture
cursor targeting to 0x4E. Invalid/unused IDs with bit 7 set remain disabled.
The native interface rejects IDs 32..127 because their properties are not
part of the extracted command table.

The spell list contains 79 four-byte records: esper, 54 spells, 24 lores.
Refresh uses record ID, flags and MP cost; it preserves the other bytes.
The cost threshold is MP+1 capped at 255, so cost 255 is disabled even with
higher MP. Used espers are disabled. Imp permits only menu ID 35. Silence
is checked by command refresh, not spell refresh. Monster slots leave the
spell records and threshold untouched.

Command record pointers are at C2/544A. Relevant RAM includes changed mask
7E/3A58, status 7E/3EE4..3EE5, hand flags 7E/3BA4..3BA5,
relic flags 7E/3C58 and Morph counter 7E/1CF6 under DBR 7E.
Spell pointers are at 7E/302C, MP at 7E/3C08, used esper mask at
7E/3F2E and computed threshold at 7E/3A4C (actor arrays use slot*2).

## Validation and remaining integration

`python tools/verify_battle_menu_state.py` compares 8,192 command cases and
2,048 spell cases against actual target 65816 instructions, without hooks.
It executes 2,524,097 instructions and compares all record bytes and output
masks/thresholds. Fixtures include all character slots, monster early exits,
status/weapon/relic combinations and MP boundaries. A fixture bank-address
error was corrected before the final passing comparison. See
[battle-menu-state-validation.json](battle-menu-state-validation.json).
Two focused Rust tests cover MP/Imp and Offering/weapon gates.

These routines do not construct lists, render battle menus or execute
commands. They are not yet connected to the full AfterAction2 dispatcher
or a live battle. The playable opening still pauses at its battle request.
