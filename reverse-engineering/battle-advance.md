# Advance-wait preparation

Evidence: verified patched/clean ROMs and FF6 disassembly commit
`813013276c952fdd27edcf7b8b86f17291542cbf`, `battle/main_2.asm` and
`battle/action_1.asm`. Native implementation: `src/battle_advance.rs`.

## Address map

| Routine | CPU | File offset | Role |
|---|---|---|---|
| Advance dispatch | C2/2188–C2/220C | 022188–02220C | Deferred flag, preparation and presentation request |
| QueueAction prefix/list path | C2/031C–C2/037A | 02031C–02037A | Initialize wait, clear flags, prepare pending lists |
| ClearDef / ClearFlag1 | C2/0A41 / C2/0A43 | 020A41 / 020A43 | Clear defending bit |
| SetFlag1 | C2/5BAB | 025BAB | Set deferred advance flag |
| CalcCmdDelay | C2/03E4 | 0203E4 | Cached command/attack, delay, target preparation |
| Character AI entry | C2/02DC | 0202DC | Explicit unimplemented AI execution hook |
| Character autonomous path | C2/037B | 02037B | Not implemented here |
| InitTarget / ChooseTarget | C2/26D3 / C2/587E | 0226D3 / 02587E | Explicit target-system hooks |

The two main mapped regions are byte-identical between clean and target.
They are 65816 code, using byte actor offsets and width changes around word
payloads. Target scripts and payloads remain data, not CPU instructions.

## Native state and preparation

`AdvanceState` combines the existing scheduler, command lists, retaliation
flags, character AI-pointer high bytes, changed-target mask and cached
command/attack/targets/MP flags. Its defaults are an empty modeled state, not
a claimed full battle initializer. Callers dispatch advance work first, then
supply the corresponding actor slot to `prepare`.

The source stage performs these operations:

1. Set 3AA1 bit 7. If actor flags 3AA0 contain 10 or 40 hex, defer immediately;
   do not reset wait or change cached command state.
2. Otherwise clear that deferred flag and set 3AA1 bit 0. Clear the scratch
   target word, change a disabled FF wait duration to zero, clear defending,
   and clear Runic/Retort retaliation bits (mask FA).
3. A monster with no pending command receives a normal-list RANDOM action
   (command 1F), zero targets and zero MP cost. Its unused attack byte is
   preserved from cached state, matching source. Its AI is expanded later at
   execution, not chosen early during preparation.
4. A character clears its bit from the changed-target mask. A valid AI pointer
   invokes the AI hook. An ordinary character's existing command list is
   traversed in order, accumulating command delays and storing target updates
   back into each queued payload. Cached MP costs are not recomputed.
5. Commands below 1E invoke target initialization and, if the result is zero,
   target choice. Commands 1E and above skip those operations.
6. A prepared character command below 1E requests advance presentation using
   the first pending command and its targets. This does not execute damage.

The source sums delays across pending commands. X-Magic's two commands can
therefore contribute separate delays. Delay saturation and linked-list self
terminators use the previously checked implementations.

AI, target initialization/choice and presentation are source-position callbacks
that can mutate native state. They are not guessed here. The callback-free
character autonomous path and Jump's hide/jump/presentation changes fail
explicitly as unsupported. Corrupt list cycles also fail rather than looping.
Those failures can occur after the known preparation prefix has changed state;
the function is not transactional.

## Connected timing path

A native test now follows a real Magitek input packet through:

```text
PlayerInputs -> advance actor queue -> prepare command -> wait counter
             -> normal action queue, retaining attack/targets/cached cost
```

The controlled test uses a 16-unit Magitek delay and a supplied 512 gauge
increment. It reaches the action queue after sixteen two-frame ticks. This is
a component integration test, not proof that opening-battle initialization
has supplied those exact gauge values or that the action executes.

The scheduler context's 2F45 byte is now named `running`: source comments and
escape code identify it as run mode. Earlier diagnostic code called it
`advance_player`; renaming changes no gate behavior. During run mode, ordinary
player advance counters are held unless the source's Zombie, jumping or
enemy-character exceptions apply. The original scheduler comparisons are rerun.

## Reproduction and evidence limits

```powershell
.\tools\cargo-local.ps1 test --offline
.\tools\cargo-local.ps1 build --release --offline
python tools/verify_battle_advance.py
```

The verifier compares 512 controlled cases with actual target advance/queue
instructions (27,023 instructions). It checks actor wait/flags, retaliation
flags, cached state, MP flags, complete list links/heads/payloads and callback
order. Four retained traces link CPU/file addresses, opcodes, registers,
width flags and stack state. Both mapped regions are compared with the clean
ROM before running the checker.

AI execution and graphics callbacks are intercepted. Target callbacks supply
explicit fixture transformations and choices, not original target selection
or RNG. Character autonomous and Jump paths are excluded and reported as
unimplemented. Matching these fixtures does not prove those external systems.

The live story still pauses at the first battle request. Full battle
initialization, targeting, AI expansion, normal/immediate/retaliation action
execution, hit/HP/status application, rendering/audio, rewards and event return
remain incomplete. Preparation or action enqueue never grants a fake victory.
