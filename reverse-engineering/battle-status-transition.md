# Status transition preparation and storage helpers

## Mapping and evidence

`src/battle_status_transition.rs` translates InitStatusVars
C2/450D..4584, the post-handler store C2/4585..4597, and ResetStatusMod
C2/44FF..450C (file offsets 02450D..024597 and 0244FF..02450C).
The actual target bytes match clean ROM. Pinned battle--status_effect.asm
supplies labels and structure. Confidence is high for these helpers, not
UpdateStatus's full dispatcher or individual status handlers.

## Preparation

Pending set words are filtered through allowed-status masks. Pending clear
words are copied unchanged at this phase; the dispatcher filters them later.
Existing Petrify is added to the set scratch word regardless of immunity.
Near Fatal is requested at HP <= floor(max HP / 8), including exact equality;
if currently Near Fatal and HP is above that threshold, removal is requested.
Existing pending words are not overwritten by this preparation.

Overcast converts a pending Dead request into Zombie and requests Condemned
removal. It does not trigger solely from current Dead. The source's 16-bit
LDA at direct-page FB spans current-status-high and pending-set-low bytes;
its sign test therefore reads the pending Dead bit. This resolves the source
comment's uncertainty and is covered by actual instruction comparisons.

If the source retaliation marker word is negative, the helper stores a
resolved pre-handler status snapshot for retaliation. It preserves scratch
set words after taking the snapshot, and does not replace current status.

## Post-handler storage and pending reset

The native store_after_handlers API requires already processed/resolved
status words. It applies the source Zombie normalization mask 4DFA, clearing
Blind, Poison, Near Fatal, Sleep, Confuse and Berserk when Zombie is set,
then stores both current status words. Reset clears all four pending words.
These helpers must not be used to skip unimplemented per-status effects.

## Validation and integration

`python tools/verify_battle_status_transition.py` compares 16,384 cases per
helper, executing 921,271 actual target instructions without hooks. It
checks prepared scratch words, unchanged pending/current state, retaliation
snapshots, controlled post-handler storage and pending reset. Captures and
selected traces remain local under captures/native-battle-status-transition.
Post-handler inputs in this comparison are controlled resolved values; the
individual handlers are intentionally not executed or claimed as validated.

Connected Rust tests send an immune Poison request through preparation and
complete its no-change path (no status handlers are selected). Another feeds
native lethal HP damage's Dead request into Overcast preparation, checking
Zombie/Near Fatal requests and Condemned removal while current status remains
unchanged. Neither establishes a complete death transition or enemy defeat.

## Remaining scope

Complete UpdateStatus and per-status handlers now exist in [battle-status-update.md](battle-status-update.md). Downstream consumers of status flags, full attack/action execution, rendering, audio, victory/rewards and field return remain unfinished. The story still pauses at the first battle. Original PPU/runtime comparison remains unavailable.
