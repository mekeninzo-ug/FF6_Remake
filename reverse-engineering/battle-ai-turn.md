# Native AI attack emission

## Mapping and extraction

`src/battle_ai.rs` now executes a bounded normal AI turn using the target
ExecAI/DoNextAICmd path C2/1A2F..1B46, GetCmdForAI C2/1DBF..1DD7, and
FixRoulette C2/03B9..03C5. The extraction pipeline also exports the eleven
attack thresholds at C2/1DD8 and command IDs at C2/1DE3. Both tables match
clean ROM and are byte-checked before extraction. Pinned battle--ai_cmd.asm
and battle--action_1.asm provide labels; actual target bytes govern behavior.

## Supported execution and state

Native run_normal handles direct attacks 00..EF, F0 random attack triples,
F4 random battle-command triples, FD wait, and FE/FF end markers. F0/F4
selection uses the source table RNG and selected FE means no action.
Attack thresholds map the attack ID to a battle command. EE/EF map to
Fight, not a separate generic Special command. Lore/Roulette is rewritten
to internal command 30, preserving attack ID 140. Confusion clears targets.

The caller supplies the current loop/resume state. Wait stores the next
command position and block index; the following turn redirects at the source
comparison point, avoiding repeating earlier actions. End markers set the
saved block marker to FF. The caller receives emitted command/attack/targets
at the source preparation boundary. Target normalization, delay calculation
and command-list allocation remain caller responsibilities. Unsupported
conditional, targeting and effect opcodes return explicit errors, with no
invented substitute behavior. Earlier emitted effects are not rolled back.

## Validation

`python tools/verify_battle_ai_turn.py` executes actual target instructions
for all 256 attack mappings, 1,024 Guard turns and 2,048 labeled controlled
script cases, totaling 474,205 instructions. Guard cases vary all RNG seeds,
loop markers and Confusion. Controlled scripts exercise waits/resume,
explicit commands, selected FE, end markers and Roulette mapping. Fixture
bytes are supplied in checker memory; input ROM files remain unchanged.
The checker models the previously verified hardware multiply helper and
intercepts CalcCmdDelay and CreateDefaultAction at the emission interface.
It compares emitted actions, final RNG index and saved loop state. It does
not validate target selection or complete action storage in these comparisons.
Captures and traces are local under captures/native-battle-ai-turn.

A connected Rust test emits a Guard action, calculates its native command
delay and MP cost, and appends it to the existing native normal command pool:
Fight, attack EE or EF, controlled target mask 000F, MP cost zero, delay 16.
This is controlled component integration, not a full enemy turn in the app.
Confidence is high for compared paths. Other supported forms have controlled
instruction evidence; general conditional AI remains unimplemented.

## Remaining work

Conditionals, AI targeting and remaining AI effects, retaliation execution,
command execution, rendering, audio and the live battle loop remain unfinished.
The app still pauses at the first battle. Original runtime comparison remains
unavailable; queued actions do not establish victory, rewards or field return.

The callback now shares RNG with target preparation. The comparison additionally runs 1,024 targeted controlled script cases through actual CalcCmdDelay/ChooseTarget (Fight InitTarget and storage boundaries modeled), bringing its instruction total to 1,385,678. See [battle-targets.md](battle-targets.md).

The targeted comparison now also executes actual InitTarget; only final storage remains intercepted in those cases. The updated total is 1,491,662 instructions. See [battle-target-init.md](battle-target-init.md).
