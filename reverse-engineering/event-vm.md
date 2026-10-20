# Native event VM evidence

## Scope and addresses

`src/event_vm.rs` interprets FF6 game event data, not 65816 instructions. The
implemented entry is `event_main.asm::_cc9a4f`, CPU `$CC:9A4F`, file offset
`0x0C9A4F`, event-block offset `0x029A4F`. Its 134 bytes end at exclusive
`$CC:9AD5` / file `0x0C9AD5`. DLG_11 yields at event offset `0x029AB6`.

The pinned vanilla source is commit
`813013276c952fdd27edcf7b8b86f17291542cbf`. A separate Python encoder in
`tools/verify_opening_event.py` assembles the selected source macros, compares
all bytes against both supplied ROMs and event exports, and matches all 80
structural decoder spans. `opening-event-validation.json` records source/executable
digests, states and effects. Confidence is high for these bytes and endpoints;
no original emulator timing or presentation comparison has been performed.

## Supported subset

Event commands: object blocks `$00–34` (length bit 7 means awaited), passability
`$36/$78`, visibility `$41/$42`, camera lock `$38/$39`, party `$3C`, vehicles
`$44`, sorting `$45`, dialogue `$4B`, fade `$59`, map load `$6B`, gil `$84`,
fixed waits `$91–95`, variable wait `$B5`, switches `$D0–DD`, song fade `$F1`,
SPC command `$F6`, and return `$FE`. Fixed wait `$95` is 120 frames.

Object commands: poses `$00–7F`, cardinal movement `$80–9F`, speed `$C0–C5`,
direction `$CC–CF`, visibility `$D0/$D1`, position `$D5`, party-position copy
`$D7`, wait `$E0` (operand times four frames), and block end `$FF`. The pinned
`obj.asm::ObjCmd_d7` copies party coordinates to the selected actor; its source
annotation `7a65` is not independently mapped to a target ROM routine here.
Unknown/truncated commands fail closed with CPU/file address diagnostics.
The game retains a fault latch after its error message is dismissed, preventing
execution from continuing through operand bytes after an unsupported command.

The entry context is explicitly seeded from preceding `_cc985b`: party
Wedge/Vicks/Terra, normal movement speed, Magitek rider vehicle flags and event
bit `$01CC`. The entry clears `$01CC`, grants 3,000 gil, selects party
Terra/Wedge/Vicks, yields DLG_11 with Terra `(38,50)`, then returns with Terra
`(38,49)`, companions hidden at `(38,50)`, and bit `$01C1` set.

## Live scheduling

The game uses `EventVm::scheduled` and a nominal 60 Hz accumulator. Pinned
`reset.asm::FieldLoop` dispatches events, updates object commands, then moves
positions. `obj.asm::UpdateObjActions` updates six active slots each frame;
`FirstObjTbl2` is `$00,$0C,$18,$24`, so each slot updates once per four frames.
`_c0714a` places active party members first; this entry's three party members
therefore use phase zero. Enabled active-party members precede the camera, other characters and NPCs;
full per-character map ownership remains unported.

`CalcObjMoveDir` multiplies cardinal rate `$0040` by `ObjMoveMult` values
`1,2,4,8,16,32`. These are 1/256-pixel units per frame, giving normal speed
one pixel per frame and 16 frames per tile. Object `$E0` stores an update counter:
its operand counts four-frame updates, followed by the next update before the
next command. Global pause commands re-enter `ContinueEvent`, consuming the
first counter decrement immediately. Awaited scripts resume the event on the
next event tick after object completion. Sort requests wait for the next
four-frame sort boundary. Map load yields a field tick; actual loader duration
and startup behavior remain unmodeled.

`event.asm::InitObjScript` replaces the selected object's script pointer and
clears remaining step count. The entry replaces Vicks' asynchronous direction
block before its next object update. Thus the structural decoder visits 80
commands, while the live scheduler executes 78; source offsets `0x029A88` and
`0x029A89` (direction/end) are superseded. Diagnostics retain the skipped span.
An in-flight tile and wait counter survive replacement. Collision state was
renamed `collision_enabled`: event `$36` clears pass-through bit `$10`, whereas
`$78` sets it. Collision enforcement is not implemented yet.

`verify_opening_timeline.py` compares every live command to the independently
source-encoded boundaries, checks normal-speed displacement and total travel,
checks global/object wait intervals and verifies actual game-loop actor rows
against the scheduler diagnostic. Captures show all three actors and smooth
camera movement. See `opening-timeline-validation.json` for the current frame
counts, digests and limitations. This is native implementation evidence, not
original emulator timing validation.

## Limits and persistence

Fades, music and SPC commands remain diagnostic requests. Actors now render
Magitek bodies and riders from target formation/frame tables, graphics and
palettes; see [magitek-sprites.md](magitek-sprites.md). Exact DMA animation
latency and PPU sprite priority remain unverified. General camera scripts, object collision, other object slots,
General calls/branch usage, battles and the full event graph remain open.
Calls/repeated calls, OR/AND switch tests, object lifetime and tile-trigger
dispatch are now implemented for the covered scenes; see [field-events.md](field-events.md).
The preceding lore cards and exchange still use manually sequenced source-backed
content. The field camera is a smooth follow approximation, without full SNES
clipping/parallax. Original refresh differs from the nominal 60 Hz clock.

Version 8 local saves serialize modeled VM/actor state directly, including
call frames, object tasks, pending battle, flags and field display slots versus
actual character membership. Diagnostic histories are omitted. Older local
formats retain their legacy opening-replay migration. This is not SRAM format
compatibility. See [field-events.md](field-events.md) for the snapshot scope.

Player-controlled movement retains its separate fixed-point step state and
logical clock. See [player-movement.md](player-movement.md).

The opening context now uses GameStart's actual soldier palette 1 and retains
character membership separately from `$3C` display slots. Position/copy
commands `$D5/$D7` yield one object update rather than continuing immediately;
the refreshed native opening trace has 417 frames. See [field-events.md](field-events.md).
