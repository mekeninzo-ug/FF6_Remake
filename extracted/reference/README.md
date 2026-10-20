# Shared references

- `ff6-disassembly-event/` and `ff6-disassembly-sound/` are upstream vanilla source references, not patched-ROM disassemblies. The event README documents that only a small portion of the event script has been reverse-engineered.
- `char_table/` contains the upstream English character/control-code maps used by the text decoder.
- `rip_list_en.json` records the upstream U.S. asset ranges and pointer tables.
- `ff6-mml-source/` is the disassembly repository's human-readable source MML. It is reference material; the ROM-derived music sequences are the per-ROM `audio/music/*.mml` exports.
- `patch_notes/` contains every `.txt` file from the project `Patch/` folder.
- `ff6-jp-text-templates/` contains Japanese text templates present in the upstream repository, kept out of the U.S. ROM asset folders.
- `lz_list.txt` and the tool README/license files document upstream format handling and provenance.

The vanilla disassembly commit is `813013276c952fdd27edcf7b8b86f17291542cbf`; its `romtools` submodule is pinned at `f023d6b2ebdafd5a23a79264a9a3775ff5ea537e`. `mfvitools` is pinned at `f675df1d312ca1729bbc0a95fe830740d3ac737d`.
