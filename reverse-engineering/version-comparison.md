# Clean versus patched ROM

## Target variant

The target fingerprint matches the single supplied file named
`Final Fantasy VI (TWUE 3.07) + Bug Fixes.ips`. The readme lists separate
optional Updated Opera, Add-Ons and Music Player patches. They are not silently
included in the target: these optional features are absent unless independently
confirmed by the supplied ROM bytes.

## Direct binary and asset evidence

- The supplied IPS has 2,174 records. Expanding records in memory wrote
  1,805,587 byte positions; all 1,805,587 equal the patched ROM at those
  offsets. No patch application was performed.
- The mapped output comparison reports 239 changed files: 27 text, 205
  graphics and 7 data files. Source: `extracted/comparison/asset_diff.csv`.
- Changed text includes story and battle dialogue, item/monster/action names,
  and descriptions. The target includes the item-description pointer table
  and decoded description strings in the added bank.
- Changed graphics include the title/opening artwork, restored character and
  monster graphics, battle backgrounds and map sprites. The World of Ruin
  world tilemap differs substantially.
- Changed data includes monster properties, event battle groups, treasure
  properties, a Phantom Train sub-tilemap, battle animation frames and the
  World of Ruin world tilemap.
- The preserved field event slice has 23 changed runs totalling 249 bytes.
  The upstream event disassembly is partial, so these runs are not attributed
  to individual scripts.
- Music sequences, BRR samples and extracted SFX data compare byte-identical.
- The complete short/long entrance pointer-plus-record ranges also compare
  byte-identical: 1,129 point and 152 line records in each image. The independent
  audit checks all 512 logical map ranges in each table against ROM bytes,
  normalized export pointers and source-map addresses. This does not prove
  that patched code or event gating around entrances is unchanged.

## Patch notes reconciled with ROM observations

The supplied notes describe the uncensored title/art, restored names, expanded
item descriptions, script revisions and many gameplay bug fixes. The direct
asset comparison supports edits in graphics, text and gameplay data, and the
event slice proves binary edits there. This does not independently confirm the
runtime effect of every named code fix. Code-level maps or controlled gameplay
comparisons are still needed for each behavior.

The patch notes also describe optional features (Updated Opera, Add-Ons, Music
Player, and other third-party compatibility patches). They must not be treated
as part of the selected target without byte-level confirmation. Optional Music
Player content is not evidenced by the identical music/SFX extracts.

## Exceptions

The upstream battle-background table has 75 entries: 48 decode as valid LZSS,
six point to non-LZSS spans, and 21 resolve to invalid/zero-filled spans in
both ROMs. Uncertain content is preserved as unclassified data rather than
replaced with clean-ROM exports. The patched expansion dump preserves all
bytes in `$F0–$FF`; only validated structures are decoded.

## Mapped Cancel-dash and Sliding Dash changes

The target replaces vanilla inline player speed selection at `$C0:4E21` with
calls to new code at `$C0:D674` (file `0x00D674`). The clean bytes at the new
routine are `$FF`. The patched code computes speed index 2 plus normalized
Cancel and Sprint Shoes flags, independently adding each bonus. On-foot gait
shifts at `$C0:5885/$5892` call a new direction-adjusted helper at `$C0:D68D`.
A bounded A8/Y16 65816 instruction evaluator checks both raw-ROM helpers against
the Rust port. These byte findings match the supplied compendium's Cancel
Button Dash and Sliding Dash descriptions. Original event/config compatibility
and full gameplay effects remain unvalidated. See [player-movement.md](player-movement.md)
and `player-movement-validation.json` for scope, inputs and traces.

## NPC table differences

Trigger table bytes are unchanged. The NPC table has six changed bytes across
four records: map 200 NPCs 13/14 change sprite/layer-priority fields; map 317
NPCs 12/25 change palette 5 to 3. These are direct byte/format findings, not
original-runtime validation or attribution to a named patch fix. See
[npc-property-differences.json](npc-property-differences.json) and [field-events.md](field-events.md).

These raw-table checks extend the earlier canonical asset diff; generated
native bundles are outside that report's file counts.

See [battle-ai-init.md](battle-ai-init.md) for native AI initialization, decoded script boundaries and content changes separated from relocation.

See [battle-status-update.md](battle-status-update.md) for complete native UpdateStatus, all patched status handlers, full target instruction comparisons and clean baseline differences.
