# Native complete CheckHit component

## Source and addresses

`src/battle_hit.rs` joins gates, ordinary accuracy and miss processing using
pinned `battle--check_hit.asm` labels and the actual patched ROM.
CheckHit spans C2/220D..23EC (file 02220D..0223EC). It calls bit helpers
C2/51F0 and C2/522A, status helper C2/5864, and battle RNG. The target Evade
fix remains authoritative. See battle-hit-gates.md and battle-accuracy.md
for stage-specific evidence. Confidence is high within this routine's scope.

## Effects and control flow

The native API mutates an explicit effects state. Reflect sets the target
mask without marking a normal miss. Ordinary misses can choose a block
animation only for character targets without Vanish, Zombie or Magitek.
The eligible physical or magical BlockGfx byte is ORed with forced Golem or
Interceptor bits, then a set bit is selected using source table RNG.
Its one-based index is the block graphic ID. Magical block target markers
use the odd actor offset, physical markers the even offset.

IDs six and above clear all four attack status payload bytes and return a
hit, preserving weapon spellcast and normal miss flags. Other misses set
B2 bit 02, clear weapon spellcast and conditionally add the target to the
miss-display mask. Image and failed level-divisibility checks go directly
to miss processing and bypass block selection. Queued Vanish/Image removals
remain separate from current status. Existing masks and pending bits survive.

## Validation

`python tools/verify_battle_hit.py` executes 32,768 complete target CheckHit
calls, 2,126,844 instructions, returning through the actual stack epilogue.
Cases cover 8- and 16-bit accumulator entry with 8-bit index registers, the
routine's required index width. It checks preserved A/X/Y and processor
mode bits, final RNG, outcome, pending removals, reflected/miss masks,
weapon spellcast, B2 flags, block graphic, Golem/dog target markers, and all
four attack status payload bytes. Hardware multiplication/division helpers
are byte-checked and modeled; other helpers execute actual ROM instructions.
Outcomes: 9,255 hits, 20,161 misses, 3,352 reflections. Generated captures
remain local under captures/native-battle-hit.

A connected Rust test resolves a controlled automatic hit, consumes the next
RNG byte for modifiers, and runs native base damage, elements, accumulation
and HP application. The 40-HP target reaches zero with Dead queued. This
assumes caller-prepared target/attack working values and is not a complete
Magitek command or live battle.

## Remaining scope

This component returns reflection selection but does not choose reflected
targets or execute their actions. It records animation IDs rather than
rendering them. Battle initialization must supply working evasion/block
values, target masks, statuses and attack properties. Hit resolution alone
does not commit status, execute special effects, charge MP, award rewards,
or return to the field. The live story still pauses at the first battle.
Original emulator/runtime comparison remains unavailable.
