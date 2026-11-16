# Agent Instructions & Project Invariants

All AI agents working on RAnki must strictly read and adhere to `CONSTRAINTS.md` and `HANDOFF.md`.
Never weaken, suppress, or modify any threshold or gate in `CONSTRAINTS.md` to make a change or CI run pass.
Never touch production data at `D:\anki_data\`; all tests must target `D:\anki_data_test\`.
All files must use Unix LF line endings.
