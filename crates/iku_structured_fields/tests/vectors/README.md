# Structured field test vectors

JSON test vectors from https://github.com/httpwg/structured-field-tests,
commit `4a4782f8234512af133f6f57c4ca998578b57110`, copied unchanged.

License: BSD 3-Clause (Copyright (c) 2018- IETF Trust and the persons
identified as authors of the code); the license text is in `LICENSE.md` here,
as the license requires for redistribution.

Not copied: `serialisation-tests/` (values that this package cannot construct),
`large-generated.json` (about 700 KB, inputs near the size limits),
`generate.py` and the JSON schemas. The tests are in `../httpwg.rs`; all 1596 vectors pass, and the skip table
there is empty.
