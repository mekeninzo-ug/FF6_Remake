# AI script initialization and decoded boundaries

## Mapping and extraction

Native `src/battle_ai.rs` now implements InitAI C2/2D71..2D98
(file 022D71..022D98) and FindAIScriptEnd C2/1A43..1A5E. Command lengths
are extracted from C2/1DAF..1DBE, file 021DAF..021DBE. Those code and table
regions match clean ROM. Primary structure and labels come from pinned
`battle--init_2.asm` and `battle--ai_cmd.asm`. No source ROM is modified.

The repeatable battle extractor now includes ai_command_sizes.bin. It
verifies both input ROM identities and compares all sixteen bytes against
the pinned source table before writing local assets. The running native
component uses extracted pointers, scripts and lengths, not the ROM.

## Initialization semantics

FindAIScriptEnd advances by complete command lengths. FE ends a conditional
block and FF ends a script; these values inside operands are skipped.
InitAI starts at the monster pointer, scans through FE-delimited blocks
until FF, then probes the next byte. If that byte is FF, the existing
retaliation pointer is preserved. Otherwise the following byte begins the
retaliation script. Normal and retaliation pointers are relative to CF/8700.
Malformed or unterminated data beyond the extracted region returns an
explicit native error instead of reading unrelated ROM banks.

The first Guard integration test now initializes both actual Guard script
records: normal pointer 0000, end-next pointer 0005, no retaliation script
(existing FFFF retained). This runs alongside the derived trait, hit and
damage checks; it does not yet execute an enemy turn.

## Validation

`python tools/verify_battle_ai_init.py` executes actual target InitAI in 768
cases (all 384 records, two existing retaliation values), plus 734 normal
block scans. It compares pointers, end bytes, next offsets and preserved
registers across 215,589 instructions, with no hooks. The target has 175
monster entries without retaliation. Captures and traces remain local under
captures/native-battle-ai-init. Confidence is high for initialization and
byte-stream boundaries, not full AI behavior.

## Clean versus target script content

Decoded complete normal and retaliation streams are compared independently
of their offsets. Normal pointers move for 382 entries, while normal stream
bytes differ only for decimal monster IDs 34, 36, 37 (source labels PTERODON,
WHITE_DRGN, DOOM_DRGN). Retaliation bytes differ for IDs 127 and 243 (source
labels SRBEHEMOTH_UNDEAD and MAG_ROADER_4). This is a byte comparison, not a
claim of semantic equivalence for unchanged streams or runtime interpretation
of changed ones. Full per-record CPU/file locations and hashes are in the
local version-script-comparison.json capture; changed IDs are also retained
in battle-ai-init-validation.json.

## Remaining scope

The interpreter still only selects the limited attack forms documented in
battle-rng.md. Conditional evaluation, target selection, action loops,
retaliation scheduling and the remaining AI commands are not implemented.
Full monster loading, downstream status-flag processing, command execution and live battle remain
unfinished. Original runtime comparison remains unavailable.

Normal attack-block execution now exists in [battle-ai-turn.md](battle-ai-turn.md); conditions, effects and retaliation execution remain incomplete.

Complete status update is now available in [battle-status-update.md](battle-status-update.md); downstream flag consumers and full battle execution remain unfinished.
