# Shared monster and Rage trait loader

## Mapping and evidence

`src/battle_rage.rs` translates LoadRageProp at C2/2DC1..2E39
(file 022DC1..022E39). The actual target code bytes match clean ROM.
Pinned `battle--init_2.asm` supplies labels and structure; actual target
monster property records supply content. Confidence is high within this
complete helper, not the whole Rage command or monster loader.

## State transformations

The helper replaces special-attack properties and pending status 1/2 from
the selected monster record. It merges weak elements, status immunity,
absorbed/null elements, equipment status 2/3, and the True Knight/Cover relic
bit with existing actor values. Element halves and other high-byte values
survive byte-sized merges. Status 4 immunity is preserved. Metamorph info
and monster flags are replaced together by the property word at offset 17.

Flying at bit 0 of innate status 3/4 is transferred into pending Float at
bit 15. The source then masks pending statuses with 84FE, excluding the
character-only statuses. Innate status 4 bit 0 adds Cover to relic effect 4.
These operations act on pending requests and traits; current status commit
is separate. This distinction matters when Rage modifies an existing actor.

## Validation and integration

`python tools/verify_battle_rage.py` compares all 384 target monster records
with 16 controlled existing actor states each: 6,144 cases and 258,048 actual
target instructions. No helper hooks are used. Every projected field is
compared, including preserved high bytes and overwritten pending words.
Selected traces and native CSV are local under captures/native-battle-rage;
the durable report is battle-rage-validation.json.

The first-Guard integration test now uses this helper rather than manually
picking immunity and resistance fields from property bytes. It checks the
Guard's special property 20 hex, Poison weakness 08, zero innate pending
statuses, and metamorph/monster flag word 1000 hex. Its derived traits feed
immunity normalization and elemental resolution in the connected controlled
Fire Beam test. This is component integration under controlled inputs, not
a complete battle or an original runtime comparison.

## Remaining scope

Rage command selection, learned Rage availability and battle command behavior
remain unfinished. Full monster loading also needs AI/retaliation scripts,
name aliases, drops/steals, HP restoration, Doom Gaze persistence, special
animation, first-strike handling, randomized Strength and downstream status-flag processing.
Rendering and audio remain incomplete. The live story still pauses at the
first battle; no victory/rewards are simulated.

Complete status update is now available in [battle-status-update.md](battle-status-update.md); downstream flag consumers and full battle execution remain unfinished.
