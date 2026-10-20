# Field event script region

`event_script.bin` is the ROM slice mapped by the disassembly's `event_script` segment (`$CA:0000`, 0x2E600 bytes, through `$CC:E5FF`). `source_labels.csv` maps the labels available in the upstream partial disassembly to source addresses. The disassembly README says only a small portion is reverse-engineered; unlabelled bytes remain raw. On the patched side, use `event_byte_diff.csv` in `extracted/comparison/` for changed runs and nearest source labels.
