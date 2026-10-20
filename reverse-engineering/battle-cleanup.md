# Living masks and normal-action cleanup

## Mapping and evidence

`src/battle_cleanup.rs` translates UpdateDead C2/4AB9..4B52
(file 024AB9..024B52) and RemoveAllActions C2/0A0F..0A40.
Both code regions match clean ROM. Pinned battle--battle_end.asm and
battle--action_1.asm provide labels and structure. Existing native command
pool and scheduler types provide storage. Confidence is high within these
components, not the full AfterAction2 dispatcher or victory logic.

## Living and target masks

UpdateDead first removes untargetable bits from the override mask, seeds
valid targets with it, and rebuilds living ally/enemy masks. Present actors
are added to valid targets before Dead/Petrify/Zombie checks exclude them
from living counts. Monster dead flags also exclude living monsters.
Override targets can bypass presence/status checks; untargetable masks still
win. Revived/summoned eligibility then applies. Characters acting as enemies
move from the ally count to the enemy mask. The final counts include normal
monsters and enemy characters. This preserves the distinction between valid
targets and living combatants; it is not a generic HP-positive filter.

## Pending-action cleanup

RemoveAllActions frees the actor's normal command chain, retaining stale
payload bytes and all retaliation actions. Matching advance-wait entries
become FF without compacting the queue or changing its cursors. The source
uses a do-while unsigned index comparison: wrapped/empty ranges are not
normalized into a conventional active-ring scan. For example, head 250/tail
3 checks entry 250 and stops; head/tail 255 scans a full wrapped cycle. These
edge cases are retained and instruction-compared.

If ATB high byte is zero, it becomes FF while the low byte is retained and
flags0 is ANDed with D3. Characters append an immediate pose-reset command
44 with actor offset as its attack byte and existing working targets. MP
cost is zero and B1 bit 40 is consumed by the normal cost helper. Existing
immediate actions remain ahead of it. Monsters do not receive this command.

## Validation and integration

`python tools/verify_battle_cleanup.py` compares 16,384 living-mask cases and
1,024 action-removal cases against 3,355,150 actual target instructions.
It executes UpdateDead, RemoveAllActions, command removal, immediate command
storage and MP-cost handling without hooks. It compares masks/counts, pool
links, all heads and payloads, queue bytes, gauge/flags and MP flags. Captures
and selected traces remain local under captures/native-battle-cleanup.

A connected Rust test runs lethal Guard HP damage, full native status update,
normal-action cleanup and living-mask rebuilding. Dead commits, normal
commands disappear, retaliation remains, and the controlled living enemy
count becomes zero. Another checks character pose-reset appending. These
are component connections, not the complete post-action dispatcher, enemy
removal, victory, rewards or original runtime equivalence.

## Remaining scope

Full AfterAction1/AfterAction2 sequencing, menu/ATB refresh and other update
flag consumers, retaliation completion, target removal and victory/rewards
remain unfinished. Full command execution, rendering/audio and field return
also remain unfinished. The live story still pauses at the first battle.
