# FF6 ROM extraction

Two independent ROM dumps are in this folder:

- [Vanilla U.S. v1.0](vanilla/README.md)
- [TWUE v3.07 + Bug Fixes](twue-3.07/README.md)
- [Comparison report](comparison/REPORT.md)
- [Patch notes versus ROM evidence](comparison/patch_notes_vs_rom.md)
- [Shared references and provenance](reference/README.md)

Each ROM folder contains the mapped upstream assets under `assets/`, indexed source addresses under `provenance/`, decoded PNG/CSV previews under `previews/`, audio exports under `audio/`, and the raw field event-script slice under `event/`. Build-form files are under `vanilla/build/assets/` for the clean dump and `twue-3.07/build/en/assets/` for the patched dump; those paths are not symmetric. The patched folder also preserves the full `$F0–$FF` expansion banks and decoded text candidates under `expanded/`.

`comparison/asset_diff.csv` lists changed mapped assets; `comparison/text_changes.diff` contains decoded text diffs; `comparison/event_byte_diff.csv` lists event-script byte ranges; `comparison/audio_diff_summary.json` records sequence/sample comparisons. `tools/render_graphics.py` is the local renderer used for planar graphics, palettes, screen maps, and field/world tilemap previews.
