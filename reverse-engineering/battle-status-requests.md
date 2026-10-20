# Per-target status requests

## Mapping and evidence

`src/battle_status_requests.rs` translates MagicStatusEffect at
C2/4406..447E, CalcStatus at C2/447F..448F, and EquipStatusMod plus its
set/remove/toggle handlers at C2/4490..44FE (file 024406..0244FE).
SetStatus1 C2/0E32 is also included in the comparison. These target bytes
match clean ROM. Pinned battle--status_effect.asm and battle--calc_dmg.asm
supply labels and structure. Confidence is high within the bounded request
phase, not full status handling or commit.

## Requests versus current state

Set requests contain requested bits absent from current status; remove
requests contain requested bits already present; toggle produces both.
The helper merges these requests into existing pending words, without
changing current status or filtering pending requests by immunity yet.
CalcStatus separately computes (current OR set) AND NOT clear. That helper
is bit resolution, not a replacement for UpdateStatus's individual handlers.

Magic (physical flag clear) can queue Vanish removal when it is currently
set and the payload does not explicitly contain it. The B3 ignore-Vanish
rule can suppress this removal. A source comment says physical attacks,
but the actual branch selects magic. Fire-element attacks similarly queue
Frozen removal unless their payload explicitly includes Frozen. Undead
inversion/status-removal combinations preserve the target's special Dead
request behavior.

When the attack requires a usable status effect, the source tests requested
changes against allowed-status words. If neither status pair has an allowed
change, it sets the target failure mask and adds the target to the miss mask.
Pending words remain queued and current status is unchanged. A preexisting
pending change alone does not satisfy this attack's effect check.

Mode 0C has no StatusModTbl entry and returns an explicit native error.
No target attack-property record selects that mode; the verifier checks all
256 records. Runtime special effects that might create it remain unverified.

## Validation and connected tests

`python tools/verify_battle_status_requests.py` compares 16,384 request cases
and 16,384 CalcStatus resolutions, executing 1,356,848 actual target
instructions without hooks. It compares pending words, delta scratch words,
failure/miss masks, resolved words, preserved X and unchanged current status.
Captures and selected traces remain local under captures/native-battle-status-requests.

One connected Rust test loads target monster 41's Poison-only special
property 42, merges its traits, initializes its attack and prepares the
special. It queues Poison for allowed and immune controlled targets, checks
the failure mask, and confirms zero power contributes no HP damage. A second
test loads actual Fire Beam properties and queues Frozen removal on a
controlled frozen target while leaving current Frozen unchanged until the
separate resolution/commit phase. Neither test establishes complete command
execution or original runtime equivalence.

## Remaining scope

Complete UpdateStatus is now implemented in [battle-status-update.md](battle-status-update.md), including preparation, handlers and commit. Full target special effects, downstream status-flag consumers, retaliation, attack execution, rendering/audio and battle completion remain unfinished. The live story still pauses at the first battle.
