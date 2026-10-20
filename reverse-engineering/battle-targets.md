# Native battle target selection

## Mapping and evidence

`src/battle_targets.rs` translates ChooseTarget C2/587E..58F9 and its
masking, presence, retarget and battle-type helpers through C2/5A82
(file 02587E..025A82). These code bytes match clean ROM. Primary labels and
structure come from pinned battle--choose_target.asm. This component accepts
caller-initialized restriction BA and cursor BB bytes; InitTarget is separate.
Confidence is high within this bounded selector and valid native contexts.

## Selection rules

Self-target cursor 02 immediately uses the actor mask without filtering or
RNG. Otherwise targets pass through global availability, revived/summoned,
jump/seize and command restrictions. A one-use mask bypass flag is consumed
at the same source point. Presence filtering removes absent actors, Dead
characters, Dead/Petrify/Zombie monsters, and hidden actors unless the attack
allows the source resurrection-target path.

Retargeting derives sides from actor identity, enemy-character membership,
Charm, Zombie, Confusion and reflection. Existing valid single-target masks
are randomly narrowed; group and multi-target masks can remain intact.
Side/pincer formations restrict accessible groups. Reflected attacks can
prefer the previous attacker. Backup targets and random multi-target/single-
target choices preserve the source order and RNG consumption. The source
uses a different target bit layout for characters (bits 0..3) and monsters
(bits 8..13), retained by the native implementation.

## Validation

`python tools/verify_battle_targets.py` compares 16,384 controlled cases with
2,716,798 actual target instructions. It executes ChooseTarget, all its
helpers and actual table RNG, intercepting only the separately verified
hardware multiply helper. Final mask, consumed bypass flags and RNG index
match. Selected instruction traces and native CSV remain local under
captures/native-battle-targets. Focused Rust tests cover side/pincer random
branch ordering, self targeting, and living-character selection by a Guard.

The connected Guard test now emits an AI attack, selects a living target
from controlled opening party membership, calculates delay/MP cost, and
stores the command in the native pool. The AI callback shares its mutable
RNG so target draws occur between AI attack draws, rather than after the
whole script. This preserves the required source call order.

The AI-turn comparator adds 1,024 controlled one/two-attack script cases
that execute actual CalcCmdDelay and ChooseTarget, including Confusion.
The combined comparison now executes InitTarget as well; final storage remains a modeled boundary.
It compares emitted targets, final delay, saved loop state and RNG index.
This is combined instruction evidence under supplied target initialization,
not a complete enemy action or original runtime battle.

## Remaining work

AI targeting commands,
reflection continuation, command execution, downstream status-flag processing, battle rendering,
audio, rewards and field return remain unfinished. The story still pauses
at the first battle. No original PPU/runtime comparison is claimed.

Target initialization now exists in [battle-target-init.md](battle-target-init.md), and the Guard integration uses it before selection.

Complete status update is now available in [battle-status-update.md](battle-status-update.md); downstream flag consumers and full battle execution remain unfinished.
