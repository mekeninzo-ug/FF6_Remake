# Post-action readiness and ATB/menu gates

## Mapping and evidence

`src/battle_readiness.rs` translates pending-action reconciliation
C2/08C6..091E, CheckPlayerAction C2/091F..0976, DisableATB C2/0977..0985,
and CancelAction C2/0986..09B3. It includes C2/11EF menu eligibility and the
source queue/flag helpers. These readiness/cancel code regions match clean
ROM. Pinned battle--action_1.asm, battle--action_2.asm and battle--atb.asm
provide labels and structure. Confidence is high within these components,
not the full AfterAction2 sequence or actual battle menu.

## Pending commands

Reconciliation sets blocking bits before checking Quick ownership. Dead,
Petrify and Stop leave both action blocks. Control, Sleep, Frozen and Seize
can leave the execution block while retaining pending work. Once permitted,
the pending bit is consumed and an existing command goes to either the
normal action queue or advance-wait queue according to its prepared flag.
Advance enqueue clears that flag exactly as the source helper does. This
routine does not itself execute or delete the command.

## Player/cancel/disable paths

Player checks exclude monsters, Umaro, valid character AI and colosseum
control. They apply Quick, targetability, Seize/Charm and status gates before
requesting a menu. The menu helper adds its own update/B1 restrictions and
encodes the extra-status bit into the graphics argument. Graphics effects
are returned as typed requests, not rendered here.

Cancellation uses source low-byte target masks, Hide and menu-ready flags
to decide whether to enqueue advance-wait work and which flags to clear.
Characters request menu closure. DisableATB first clears the source flag
bits, sets only the gauge high byte to 01, preserves the low byte, then runs
cancellation. These operations are distinct from normal-action cleanup.

## Validation and integration

`python tools/verify_battle_readiness.py` compares 16,384 cases against
341,256 actual target instructions. It executes readiness, status checks,
queue mutation and menu gating, intercepting only ExecBtlGfx. Results compare
actor flags, gauge, both complete queue buffers/cursors and menu arguments.
The cases include 4,409 close-menu requests and seven open-menu requests.
Captures and selected traces remain local under captures/native-battle-readiness.

A connected Rust test keeps a queued Guard command blocked by Frozen, runs
full native status removal, and reconciles it into the action queue without
losing the command. Focused tests cover Quick ownership, encoded player-menu
requests and hidden-actor ATB disable. These are controlled component paths,
not a live command menu or original runtime battle.

## Remaining scope

The full AfterAction2 update-flag dispatcher, magic/command menu refresh,
Morph/Reraise/Condemned consumers, complete attack/action execution,
retaliation/victory, rendering/audio and field return remain unfinished.
The live story still pauses at the first battle. This module's requests do
not establish that a battle menu has been presented or an attack executed.
