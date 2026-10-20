# Ordinary attack accuracy

## Target mapping

Native `src/battle_accuracy.rs` implements C2/22FB..239B, file offsets
0222FB..02239B. Primary labels and structure are from pinned
`battle--check_hit.asm`; actual target bytes govern behavior. Ten bytes differ
from clean: 02232D and 022340, 022342..022349. These select physical Evade
instead of Magic Evade and retain separate magical handling. This is the
patch target's Evade fix, not the unpatched US behavior.

Inputs `evade` and `magic_evade` are battle working values at 3B54/3B55,
not the equipment screen's raw percentages. The caller must initialize them
through the target's battle stat setup. Substituting menu values is wrong.

## Order and arithmetic

Petrify, Sleep, Stop or Frozen automatically pass this stage. So do the
existing guaranteed-hit target mask and hit rate FF. Physical Retort also
passes. Physical Image always misses, consuming one RNG byte; below 40 hex
requests Image removal, otherwise leaves it. This occurs after auto-hit gates.

Physical Blind and target Beads each halve hit rate. Target Blind, Zombie,
Confuse, Slow or Reraise apply a quarter increase, including the carry from
the source's second shift, saturating at FF. Poison, Sap, Near Fatal or Haste
then reduce the result by its floor quarter. Magical attacks bypass these
physical adjustments. The working block value times adjusted hit rate,
shifted right eight, is compared to RandA(100); equality misses. RNG uses the
extracted target table and preserves the source's consumption count.

## Validation and limits

`python tools/verify_battle_accuracy.py` compares 16,384 controlled cases and
694,708 actual target instructions, including CheckStatus, RNG and final
hit/miss branch. Only the separately verified hardware multiplication helper
is intercepted. Result, Image removal and final RNG index are compared.
The trace stops before miss animation; captures remain local under
captures/native-battle-accuracy. Confidence is high for this bounded stage.

Earlier CheckHit Vanish, instant-death protection, undead eligibility,
Interceptor/Golem, special accuracy and level/stamina gates remain outside
this routine. Miss animations and side effects also remain outside. This
must not yet be used as a complete attack hit resolver. Live combat remains
unimplemented; these comparisons do not establish original runtime fidelity.

The earlier gates now exist as a separate native component; see [battle-hit-gates.md](battle-hit-gates.md). Miss processing still prevents treating the pair as a complete resolver.

Miss processing and stage integration now exist in [battle-hit.md](battle-hit.md); reflection continuation and rendering remain outside CheckHit.
