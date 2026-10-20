# Casting gate and MP payment

## Mapping and evidence

`src/battle_resources.rs` translates the casting gate at C2/3201..3224
(file 023201..023224) and cached MP payment at C2/32B1..32EB. Both code
regions match clean ROM. Pinned battle--attack.asm supplies labels and
structure; actual target instructions govern the native behavior. Confidence
is high for these bounded phases, not the full ExecAttack routine.

## Two distinct costs

Working attack property 11A5 controls whether Silence and Imp checks apply.
If zero, both status checks are bypassed. Otherwise Silence rejects casting;
Imp rejects it unless last spell is Imp (attack 35 decimal). Silence still
rejects that spell. The rejection queues a ResetAction graphics request for
the actor. This phase does not consume MP or clear the queued cost.

Cached queued cost 3A4C is a separate word used later during attack-effect
calculation. Zero returns without changing resources or update flags.
Sufficient MP is subtracted, cached cost cleared, and spell/esper update flag
0080 set. Exact payment can leave zero MP. Insufficient MP preserves current
MP and clears cached cost. It changes monster working command to Mimic (18)
without executing that command, clears graphics target parameters A2/A4,
disables the attack-message flag, and requests the source attack-message
setup, Not Enough MP message, and graphics-copy sequence. Native effects are
recorded explicitly; rendering them is still a caller responsibility.

## Validation and integration

`python tools/verify_battle_resources.py` compares 810 casting combinations
and 16,384 payment cases against 207,035 actual target instructions. Casting
covers all ten actor offsets, zero/nonzero working cost, Silence, Imp and its
spell exception. Payment includes full-word arithmetic, zero cost, exact MP,
insufficiency, character/monster command differences and existing flags.
Graphics setup/queue/copy interfaces are intercepted; their internal buffer
side effects are outside this comparison. The checker compares the exposed
resource state, branch result and effect requests. Captures and selected
traces remain local under captures/native-battle-resources.

A connected Rust test obtains the target Fire spell's base cost, stores a
monster Magic command in the native pool, initializes its working action and
target properties, checks Silence rejection, then checks non-Silenced
payment: 15 MP becomes 11 for cached cost 4, cached cost becomes zero, update
flag 0080 is set. This is controlled component wiring, not a complete spell
or original runtime battle. No RNG is consumed by these phases.

## Remaining scope

These phases do not validate all command prerequisites, execute the spell,
consume inventory, run Runic/Air Anchor or reflection, commit status, render
messages, award rewards or return to the field. The full attack/action loop,
audio and battle presentation remain unfinished. The live story still
pauses at the first battle; no original runtime equivalence is claimed.
