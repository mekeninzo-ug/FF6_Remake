# Queued-action initialization and Imp's Call

Evidence: verified clean/target ROMs, supplied Bug Fixes Used Compendium and
FF6 disassembly commit `813013276c952fdd27edcf7b8b86f17291542cbf`,
`battle/action_1.asm`, `data/battle_cmd_prop.asm` and `battle/bit_math.asm`.
Native implementation: `src/battle_action_init.rs`.

## Address map

| Region | CPU | File offset | Finding |
|---|---|---|---|
| InitPlayerAction | C2/0276–C2/02DB | 020276–0202DB | 102 bytes, four changed bytes |
| Patched call site | C2/02A7 | 0202A7 | `20 E1 67 EA`: JSR helper then NOP |
| Imp helper | C2/67E1–C2/67F1 | 0267E1–0267F1 | 17 injected bytes; clean region is FF padding |
| BattleCmdProp | CF/FE00 | 0FFE00 | 32 two-byte command records, unchanged |
| RandBit | C2/522A | 02522A | Source table/index RNG chooses a remaining target bit |

The clean disassembly labels the routine but does not include the target helper.
The extractor checks all 32 source command flag records and retains target
properties. Byte 0 contains Gogo/Mimic/Imp/unknown flags; byte 1 contains target
flags. This initializer reads only the Imp-allowed bit (04) of byte 0.

| Source state | Role |
|---|---|
| 7E/3420 + 2*list index | Queued command/attack |
| 7E/3520 + 2*list index | Queued targets |
| 7E/3620 + 2*list index | Cached MP cost |
| 7E/3A7C and DP B5 | Working command/attack words |
| DP B8 | Working targets |
| 7E/3A4C | Working cached MP cost |
| 7E/3A4A | Changed-status target mask, low byte used here |
| 7E/3F2E | Characters whose summon is already used/pending |
| 7E/3A40–3A41 | Raw allegiance filter bytes |

CPU code is 65816 with M/X width and native-stack behavior. The parallel
queued arrays remain data; no flat-file 6502 decoding is used.

## Native behavior

The initializer copies a queued payload to working state. It does not unlink
the command, charge MP, execute an attack or apply damage. The queued bytes
remain unchanged, and cached MP cost is copied rather than recomputed.

When the caller supplies the retarget carry flag, commands below 1D hex clear
the actor's bit from the changed-status mask. If that bit was set, clear the
working targets. Immediate callers can omit this retarget step. Magitek (1D)
and later commands skip this mask operation. Monster target masks have a zero
low byte, so this particular mask test does not clear monster targets.

For Imp status (status 1 bit 20):

1. The target helper first checks Summon (19 hex) and clears that character's
   bit from 3F2E. It then restores the original command and compares against 1E.
2. Commands 1E and above are unchanged. Lower commands with the Imp-allowed
   property flag are also unchanged.
3. A disallowed command becomes working Fight (command and attack both zero),
   with working MP cost zero. The raw source allegiance byte filters only the
   character or monster half of the target word, according to actor side.
4. RandBit selects from the remaining targets. A zero mask consumes no RNG;
   a nonzero mask consumes a scaled table draw, including a one-bit mask.

The helper's summon-bit change directly matches the supplied **Imp's Call**
note: an interrupted summon should remain usable after Imp status is removed.
This confirms that bounded state correction, not every aspect of Imp, summon
availability or related fixes. Magic/X-Magic being allowed at this stage does
not bypass later spell-specific Imp restrictions in attack execution.

The function returns the original queued command in A, even when the working
command became Fight. `InitializedAction` keeps `returned_command` separate
from working `command`/`attack`; collapsing them would change later branches
in ExecAction. The raw source allegiance word is supplied by the caller; the
initializer does not invent a legal-target/ownership model.

## Integration and validation

```powershell
python tools/extract_native_battle.py
.\tools\cargo-local.ps1 test --offline
.\tools\cargo-local.ps1 build --release --offline
python tools/verify_action_init.py
```

The native integration test now follows a Magitek input through shared command
storage, advance preparation, wait timing, normal-action enqueue and working
payload initialization. Its queued node remains allocated and its cached cost
and attack ID survive. This is controlled component integration, not a live
battle or proof of the full opening initializer's inputs.

The verifier compares 8192 cases with actual target initializer, patched helper,
RandA and RandBit instructions (209,373 instructions). It varies commands,
actors, node indices, caller carry, Imp status, target masks, allegiance masks
and RNG indices. It checks original/working commands, attacks, targets, costs,
changed/summon masks, RNG consumption and unchanged queued payload bytes.
Four retained traces link CPU and file offsets, opcodes, registers, stack and
width state. No game-subsystem callback is substituted inside this routine;
only the verified hardware multiply helper is hooked by the bounded checker.

Confidence is high for this stage. Full action dispatch/execution, targeting,
spell restrictions, MP subtraction, hit/HP/status application, battle visuals,
sound, victory/rewards and event continuation remain incomplete. Live story
still pauses at the first battle request. The local save format does not yet
serialize an executing battle, since that battle is not connected to play.
