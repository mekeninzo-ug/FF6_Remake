# Command storage and player-input normalization

Source: verified clean/target ROMs and FF6 disassembly commit
`813013276c952fdd27edcf7b8b86f17291542cbf`, `battle/action_1.asm`,
`battle/action_2.asm` and `battle/battle_ram.inc`. Native implementation:
`src/battle_commands.rs`. These components are not yet connected to live
battle UI or action execution.

## CPU/file map

| Routine | CPU | File offset | Meaning |
|---|---|---|---|
| GetPlayerActions | C2/4D1F | 024D1F | Four-entry input buffer, advance queue and command creation |
| GetPlayerActionTargets | C2/4D6D | 024D6D | Copy target word |
| FixPlayerCmd | C2/4D77 | 024D77 | Cache command/attack, clear targets for Zombie/Confuse |
| FixPlayerAttack | C2/4D89 | 024D89 | Command-specific normalization, partially patched |
| AddToQueue allocator | C2/4E54 | 024E54 | Highest free index among 128 links |
| CreateImmediateAction | C2/4E91 | 024E91 | Immediate list |
| CreateRetalAction | C2/4EB2 | 024EB2 | Retaliation list and first-entry actor enqueue |
| CreateNormalAction | C2/4ECB | 024ECB | Actor's normal command list |
| Common CreateAction | C2/4EDF | 024EDF | Append link, cache cost and write payload |
| GetMPCost | C2/4F08 | 024F08 | Modified character costs / base monster costs |
| RemoveAction | C2/0301 | 020301 | Remove normal head, free link |

| WRAM | Structure |
|---|---|
| 7E/3184–3203 | 128 link bytes |
| 7E/32CC / 32CD + 2*actor slot | Normal / retaliation head |
| 7E/340A | Immediate head, shared with scheduler |
| 7E/3420 + 2*list index | Command/attack bytes |
| 7E/3520 + 2*list index | Little-endian target mask |
| 7E/3620 + 2*list index | Cached MP-cost byte; high byte is not written here |
| 7E/2BAE–2BCD | Four eight-byte player-input entries |
| 7E/3A6A | Input cursor, 0/8/16/24 |
| 7E/3084 | Attack-to-master-spell-index map |
| 7E/302C + 2*character slot | Spell-list pointers |
| DP B1 bit 40 | Consume-once MP-cost waiver |

These are HiROM CPU addresses and WRAM data structures, not flat 6502 code.
Queue actor offsets and command-list indices are distinct identifiers.

## List behavior

Allocation scans 127 down to 0 for a negative link, then writes a self-link.
Self-links terminate lists. Append follows links until a self-link or negative
entry, links that tail to the new index, and stores the payload in parallel
arrays. Normal creation changes the head only for a previously absent list.
Retaliation enqueues the actor once, when its first command is created.
Immediate creation preserves its first head and appends subsequent commands.

Removal advances the normal head or sets it to FF at the terminal node. Only
the link is freed; old payload bytes remain. Allocation reuses the highest
available slot. If the pool is full, the target overwrites index zero; native
allocation preserves this fallback rather than inventing a capacity error.

Malformed cycles and out-of-pool pointers fail explicitly instead of hanging
the native program or reading unrelated state. Those bounds are protective,
not confirmed original recovery behavior. Malformed-list errors are not
transactional.

## Cached MP costs

GetMPCost clears B1 bit 40 on every call and returns zero if it was set.
Only Magic (02), Lore (0C), X-Magic (17) and Summon (19) use spell costs.
Magitek (1D) therefore queues zero even if a supplied spell map has a cost.
Characters use the master-map index and already modified character cost;
monsters use MagicProp byte 5. A missing master index (FF) returns zero, which
does not authorize executing an unavailable spell. Unaligned character offsets
and missing modeled list entries fail explicitly.

The payload retains the selected cost. Recomputing it after equipment/status
changes would omit this source behavior. `MpCosts` takes spell lists as inputs;
spell ownership, cost modifiers, MP subtraction and insufficient-MP handling
remain separate work.

## Supported input path

An input entry stores character slot, command, first attack, first target word,
second X-Magic attack and second target word: 1+1+1+2+1+2 bytes. Negative
character slots mean no pending input. The cursor stops there rather than
searching later entries.

`PlayerInputs::apply` supports Fight, Magic, Magitek and X-Magic. It enqueues
advance work, clears source menu flags with 7B, marks the slot FF, advances the
cursor modulo 32 and appends normalized payloads. Magitek adds 83 hex (131) to
its menu-relative attack ID; byte overflow selects EE (`BATTLE`). Legal menu
availability is not checked by that arithmetic. X-Magic creates a first Magic
command and second X-Magic command, each with its own cached cost and targets.
Zombie or Confuse clears queued targets. A single cost waiver affects only
the first cost calculation.

Unsupported commands stop before consuming the entry. Item, Throw, Slots,
esper, Dance, Rage and Blitz normalization require additional source-specific
effects. This is a bounded input handler, not a working battle menu.

## Patched normalization

Creation, MP and removal code match clean bytes. The broader input region is
not wholly unchanged: C2/4DAF–C2/4DDA / file 024DAF–024DDA changes Item/Throw
and Slots branches. The four supported commands bypass them and are checked
against target instructions. Changed bytes are retained in
`battle-commands-validation.json`. Exact attribution and a full port of those
branches remain open; vanilla behavior is not substituted.

## Reproduction, confidence and gaps

```powershell
.\tools\cargo-local.ps1 test --offline
.\tools\cargo-local.ps1 build --release --offline
python tools/verify_battle_commands.py
```

Comparisons cover 100 creations, 64 removals, 140 allocations, 20,480 MP-cost
cases and 256 input cases. MP checks execute 351,196 target instructions;
input checks execute 111,216. Input validation compares the full buffer,
cursor, links/heads/payloads, advance queue and actor/global flags. The target
normalizer and GetMPCost execute for supported input cases, with supplied
modified spell lists. Standalone creation checks inject the cost return to
test storage separately. Three instruction/state traces link CPU/file offsets,
opcodes, registers and stack/width state.

Rust tests cover reuse, retaliation enqueue, immediate heads, cycle bounds,
Magitek conversion, two X-Magic payloads, cost waivers and target clearing.
Confidence is high for these bounded paths; no original battle runtime capture
is claimed. Live story still pauses at the first battle request. Full battle
initialization, menu availability, unsupported normalizers, action/hit/HP/status
resolution, AI progression, presentation/audio, rewards and event return remain
incomplete. A stored command does not execute or grant victory.
