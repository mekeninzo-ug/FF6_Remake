# Patch notes versus ROM evidence

## Confirmed in the ROM

The patched fingerprint matches the supplied TWUE v3.07 + Bug Fixes image, and all expanded bytes in its IPS records match that image. The decoded asset diff independently shows changes in dialogue, item/monster/action text, title and character graphics, battle/map graphics, and gameplay data. The TWUE item-description pointer table and its 256 decoded strings are present in the added F1 bank. The `Hard cider` dialogue edits appear in the patched English story text.

The field event-script ROM slice contains 249 changed bytes in 23 runs. These bytes confirm event-script changes at the binary level; the upstream event disassembly is incomplete, so I do not attribute each run to a named feature.

Music, SFX, and sample data extracted from the two ROMs are byte-identical after normalizing the ROM-specific BRR directory names in MML output. The optional Music Player patch is not evidenced by these audio assets.

## Documentation claims not independently attributed to a code feature

The readme and bug-fix compendium describe the base TWUE text/graphic changes and many gameplay fixes. The asset and event diffs confirm ROM edits in those areas, but they do not by themselves prove the behavior of each named fix (for example, Sketch, Vanish/Runic, Rippler, or the various battle fixes). Those names remain documentation claims unless a dedicated code-level decoder or runtime comparison maps them to the changed instructions/data.

## Optional patches mentioned in the documents

The supplied readme/compatibility notes describe these as add-ons or separate options, so this report does not count them as installed solely because they are documented:

- Rewards Display
- Blitz Screen
- Y Equip Relics
- Mini-Maps Upgrade
- Rage Checklist
- Rage and Dance description expansions
- Bushido speed-up
- Updated Opera
- Music Player

The item and monster list documents are included as naming references; they are not ROM evidence by themselves. See `reference/patch_notes/` for all five original text files.
