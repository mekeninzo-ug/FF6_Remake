# Native ATB coordination and action queues

Primary reference: FF6 disassembly commit
`813013276c952fdd27edcf7b8b86f17291542cbf`, `battle/main.asm`,
`battle/atb.asm`, `battle/action_1.asm` and `battle/action_2.asm`, checked
against both verified ROMs. All mapped scheduling regions below are unchanged
in the target. Native implementation: `src/battle_scheduler.rs`.

## Address map

| Region | CPU | File offset | Meaning |
|---|---|---|---|
| Main-loop selector | C2/0033–C2/00C4 | 020033–0200C4 | Immediate, retaliation, advance, normal work |
| UpdateBattleTime | C2/111B–C2/1210 | 02111B–021210 | Input handoff, frame gate, actor loop and ready handling |
| Advance wait increment | C2/1193 | 021193 | Half-speed wait counter and action enqueue |
| Gauge increment | C2/11BB | 0211BB | Overflow, ready flags, menu/automatic-action handoff |
| ClearFlag1 | C2/0A43 | 020A43 | AND actor flag byte 3AA1 |
| Enqueue advance | C2/4E66 | 024E66 | Store actor offset and clear 3AA1 bit 0 |
| Enqueue action | C2/4E77 | 024E77 | Append to normal queue |
| Enqueue retaliation | C2/4E84 | 024E84 | Append to counterattack queue |
| CalcCmdDelay | C2/03E4–C2/041F | 0203E4–02041F | Delay arithmetic followed by targeting work |
| CmdDelayTbl | C2/067B | 02067B | 32 one-byte command delays, unchanged |

| WRAM | Contents |
|---|---|
| 7E/3A64–3A65 | Advance queue head/tail |
| 7E/3A66–3A67 | Action queue head/tail |
| 7E/3A68–3A69 | Retaliation queue head/tail |
| 7E/3720 / 3820 / 3920 | Three 256-byte queue arrays |
| 7E/340A | Immediate command-list pointer; FF means absent |
| 7E/3407 / 3406 | Active retaliation / normal actor offsets; negative means absent |
| 7E/3218 + actor offset | 16-bit ATB gauge |
| 7E/322C + actor offset | Advance duration |
| 7E/3AB4 + actor offset | Advance counter |
| 7E/3AC8 + actor offset | Gauge increment |
| 7E/3AA0–3AA1 + actor offset | Scheduling/status flags |
| 7E/33F8 | Zinger attacker, excluded from gauge updates |

Actor offsets here are 0,2,...,18 decimal. They are not field party slots.
The queues retain raw actor-offset bytes; immediate work retains its distinct
command-list pointer. This port is native scheduling logic, not a CPU emulator.

## Queue behavior

`Ring` uses the original byte pointers. Enqueue writes at tail, then wraps
tail with eight-bit arithmetic. Dequeue advances head until it finds a byte
with bit 7 clear or reaches tail. Entries are not cleared after reading.
There is no invented capacity error: 256 outstanding writes alias head and
tail, making the queue appear empty exactly as byte pointers do.

`Queues::dispatch` returns work without executing it:

1. Immediate work if its pointer is not FF.
2. Active retaliation, otherwise the next valid retaliation queue entry.
3. The next valid advance-wait queue entry.
4. Active normal action, otherwise the next valid normal queue entry.

Pending advance work outranks an active normal actor. Active retaliation and
normal actor bytes with bit 7 set are ignored. An empty result is not a battle
victory. Dirty graphics, Veldt handling and battle-end checks surrounding this
selector remain separate work; the port does not claim the whole main loop.

## Timing integration

The scheduler owns ten actor timing records, the battle clock, and all three
queues. It connects the previously checked gauge arithmetic to source flag and
queue transitions:

- Input-side changes are supplied before `advance_time`, corresponding to
  GetPlayerActions. Wait-mode stopping and the two-frame gate follow C2/111B.
- Process actors 9 down to 0. Morph callback occurs before eligibility checks.
  Presence, Zinger and the shifted 3AA0 flag gates are preserved.
- Advance waits honor run-mode, zombie, jump and enemy-character
  checks. A completed wait sets 3AA0 bit 5 and enqueues a normal action.
- Gauge overflow clears the high gauge/wait-counter bytes and disables wait.
  Character escape checks happen before subsequent readiness flag decisions.
- Automatic readiness enqueues advance work and clears 3AA1 bit 0. Menu-ready
  characters preserve update/disabled-menu gates, set the source flags and
  emit an open-menu request with the control-menu bit from ExtraStatus.

`advance_time_with` executes explicit callbacks at their source call position.
Callbacks can mutate actors, queues and context before execution continues.
The no-effect callback collector is used for the bounded comparison fixtures.
This distinction matters: applying all status/escape effects after the actor
loop would produce incorrect scheduling when those effects change flags.

Callback events cover status counters, Morph counter, escape checks, menu
requests and graphics-buffer updates. Their full subsystems are not implemented
by this scheduler. The native test ensures callback mutations are observed in
the correct order. Source behavior of those external subsystems is not inferred
from that test.

The 2F45 context byte is named `running`, matching source escape code. The old
diagnostic name `advance_player` was misleading; the actual gate is unchanged.

## Command delay arithmetic

The target table is checked against all 32 numeric disassembly entries.
For commands below 1E hex, add the table delay to the current duration and
saturate at FE hex. Commands 1E and above leave the duration unchanged.
Fight and Magitek each add 16; Magic adds 32. The source's prior QueueAction
step can change a disabled FF duration to zero; callers must preserve that
separate step. Target selection and linked command-list creation following
the arithmetic are not implemented by `CommandDelays::add`.

## Reproduction and evidence limits

```powershell
python tools/extract_native_battle.py
.\tools\cargo-local.ps1 test --offline
.\tools\cargo-local.ps1 build --release --offline
python tools/verify_battle_scheduler.py
```

The native diagnostic exports compare 1024 ATB states, 512 dispatch states,
1792 delay cases and 1536 ring operations. Target checks execute 216,916
instructions with correct 65816 width, stack and bank behavior. They compare
actor flags/counters, queue contents and pointers, callback order and clock
state. Four retained traces link each opcode to CPU and file addresses.

Input/status/Morph/escape/graphics callbacks are explicitly intercepted in the
ATB checker. Escape fixtures have no escaped actors. Selector fixtures keep
the battle-end check nonterminal. Delay checking stops before targeting work.
The ring-operation check is an independent arithmetic model; dispatch queue
reads and ATB queue writes are compared with actual target instructions.
These are bounded component comparisons, not an original running battle.

The live game still pauses at the first battle request. Full character/monster
initialization, command payloads, action execution, AI cursor/retaliation,
status timers, live RNG call ordering, battle presentation, rewards and event
return remain incomplete. No queued action is converted into a fake victory.
