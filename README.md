# FF6 Native (work in progress)

A native Rust reconstruction project for Final Fantasy III (USA) / Final
Fantasy VI, targeting the supplied Ted Woolsey Uncensored Edition v3.07 + Bug
Fixes ROM. The app uses locally extracted/generated assets from
`extracted/twue-3.07/`; the ROM and patch are not distributed or loaded by the
running app.

## Current milestone

The Windows software renderer assembles the opening field maps from the
patched BG1/BG2/BG3 layouts, VRAM graphics, interleaved map tilesets, palettes,
and tile properties. The lore cards use maps 23, 20, 244, and 18 in the order
read from the event source; the subsequent dialogue runs on map 18, then map 19
loads at `(38, 58)`. Terra's scripted eight-tile march places her at `(38, 50)`
for DLG_11, followed by one tile north to `(38, 49)` before player control.
The native VM reads the 134-byte map-entry script from the target export,
structurally decodes 80 commands and executes 78 of them during live scheduling
(two are superseded before execution). It pauses for target DLG_11 and runs
parallel actor scripts with fixed-point movement and source-derived wait counters.
Terra, Biggs and Wedge walk on screen, with the camera following Terra smoothly.
Magitek vehicles and riders now use target formation tables, palette and
coordinate-based gait phases. Original DMA animation latency and sprite priority
remain unverified.
Player movement now advances in continuous pixels, with patched Cancel-dash
speed, smooth scrolling, and entrances checked after tile alignment.
Movement consults target directional passability data. Point and line exits
now connect the extracted Narshe and mine maps, including 128-column layouts.
The geometry-only diagnostic follows 121 input steps across maps 19â†’39â†’41â†’42â†’41.
Live walking now activates target Narshe triggers, including the backtracking
scene and first guards. Story play pauses at the first real battle request;
the battle system and map startup events remain unimplemented.
This opening now follows source-backed map and
dialogue order, while general camera scripts, fades, other vehicles, and
SPC music remain unimplemented. The title art is still a procedural
placeholder; the source title splash and animation have not been reconstructed.
Menus, battles, the rest of the event engine, and full progression are still
incomplete. The release build and 148 Rust tests pass. Native opening captures
and a three-frame Win32/GDI startup/shutdown smoke test also pass; original
emulator behavior has not yet been compared.

## Milestones

| Milestone | Status |
|---|---|
| ROM/data map and title/intro slice | ROM/asset audits, 148 Rust tests, release build, native captures and Win32/GDI smoke test pass. Original title animation and emulator comparison remain open. |
| Field movement, dialogue, events, maps, menus and party state | Ten bundled maps, point/line exits and a 121-step native traversal work. Narshe triggers and normal NPC properties work through the first battle request. General events, battles, remaining maps, menus and party state remain incomplete. |
| Battle, commands, magic, items, equipment, shops and progression | Target encounters, base character state, equipment, patched damage arithmetic and read-only gear/status pages work. Live combat, commands, equipment editing, shops and full progression remain incomplete. |
| World traversal, cutscenes, saving/loading, audio and ending | Prototype file save/load exists; target SRAM, full vehicle behavior, cutscenes, SPC music and ending are not implemented. |
| Patched game playable from start to finish | Not complete. |

The FF4 and FF5 sibling directories were empty during inspection, so no code
conventions were available to carry over. The app uses the Rust standard
library plus Windows User32/GDI APIs.

## Runtime approach

The display target is the SNES 256Ã—224 field framebuffer, composed in Rust
from the game's map, tile, palette, and actor data and presented through GDI
with nearest-neighbor scaling. Keyboard input currently comes from Win32 key
state polling. The planned audio path is a game-specific Rust port of FF6's
sound-driver/SPC700/DSP behavior, streamed as 32 kHz stereo PCM through WinMM
`waveOut`; `mfvitools` exports will serve as sequence/sample references and
comparison material. Audio output and SPC playback are not implemented yet.

## Build and run (Windows)

1. This workspace has Rust 1.99.1 under `.tools/` and existing Visual Studio
   C++ tools. `tools/cargo-local.ps1` uses that compiler without changing the
   system PATH. On another machine, install stable Rust with the MSVC target,
   Visual C++ Build Tools, and a Windows SDK, then use normal `cargo` commands.
2. Generate the v2 runtime bundles for maps 18, 19, 20, 21, 23, 39, 41, 42, 43
   and 244. Regenerate older bundles to include entrance records and explicit
   MapProp dimensions. This reads
   the patched ROM and never writes to it:

   ```powershell
   python tools/extract_native_field.py
   python tools/extract_native_sprites.py
   python tools/extract_native_events.py
   python tools/extract_native_battle.py
   python tools/extract_native_characters.py
   python tools/extract_native_items.py
   python tools/extract_native_magic.py
   ```

3. From the project root, run `.\tools\cargo-local.ps1 run --release --offline`,
   or launch the existing `target/release/ff6-native.exe`.
4. Keep `extracted/twue-3.07/` in place for decoded graphics and map assets. No
   ROM or patch file is needed by the running app after extraction.
5. Use arrow keys to move, `Z` or `Enter` to confirm, `X` to cancel/hold for
   dash, and `Esc` or `I` for the menu. `Shift` is another dash binding.
   `F5` and `F9` save and load
   the current prototype state.

To reproduce the current implementation checks:

```powershell
.\tools\cargo-local.ps1 test --offline
.\tools\cargo-local.ps1 build --release --offline
python tools/verify_field_events.py
python tools/verify_battle_data.py
python tools/verify_battle_rng.py
python tools/verify_battle_scheduler.py
python tools/verify_battle_commands.py
python tools/verify_battle_advance.py
python tools/verify_action_init.py
python tools/verify_elemental_damage.py
python tools/verify_damage_application.py
python tools/verify_battle_accuracy.py
python tools/verify_battle_hit_gates.py
python tools/verify_battle_hit.py
python tools/verify_battle_equipment.py
python tools/verify_battle_actor_stats.py
python tools/verify_battle_rage.py
python tools/verify_battle_ai_init.py
python tools/verify_battle_ai_turn.py
python tools/verify_battle_targets.py
python tools/verify_battle_target_init.py
python tools/verify_battle_resources.py
python tools/verify_battle_attack_prep.py
python tools/verify_battle_status_requests.py
python tools/verify_battle_status_transition.py
python tools/verify_battle_status_update.py
python tools/verify_battle_cleanup.py
python tools/verify_battle_readiness.py
python tools/verify_battle_menu_state.py
python tools/verify_battle_after_effects.py
python tools/verify_battle_morph.py
python tools/verify_characters.py
python tools/verify_equipment.py
python tools/verify_damage.py
python tools/verify_player_movement.py
python tools/verify_magitek.py
python tools/verify_opening_timeline.py
python tools/verify_native_opening.py
python tools/verify_field_entrances.py
python tools/verify_field_traversal.py
.\target\release\ff6-native.exe --smoke-window
```

The capture command saves eight native BMP frames and a state trace under
`reverse-engineering/captures/native-opening/`. It compares the player-control
frame against the independent Python asset renderer, excluding Terra's actor
rectangle. This verifies agreement between implementations, not original PPU
behavior. `--smoke-window` creates a real GDI window, draws three frames and
closes through the Windows message loop.

Version 9 prototype saves add checksummed character records and retain modeled VM/actor state directly, including
call frames, running object scripts, dialogue, waits and the pending battle
request. Partial player motion, membership, clocks and vehicle flags also
survive restore. Versions 2 through 8 remain readable through legacy paths.
This is a local format, separate from cartridge SRAM. The clock is nominal
60 Hz; original hardware timing has not been compared.

The start sequence's map IDs and opening dialogue labels are sourced from the
local event disassembly and checked against clean/target event-script bytes.
The `(26, 50)` position on map 21 remains a diagnostic demo tile; the
source-backed opening proceeds through map 19 at `(38, 58)`. General NPC and vehicle
objects, exact cutscene timing, startup events, world/parent transitions and
remaining field behavior are not yet
implemented. The app does not yet play the ROM's SPC music. See
`reverse-engineering/` for the format and evidence record. Run the ROM, asset,
sprite-frame, opening-text, and field-map comparison audit with:

```powershell
python tools/verify_rom_assets.py
```

The ROMs, IPS and extracted copyrighted content are local research inputs and
must remain outside any distributable build.
