# Independent verification: spelling.athletes

- Result: **PASS**, 29/29 live cases; mismatches, candidate errors, oracle errors, and timeouts: 0.
- Started: 2026-11-06T21:06:41.299292+00:00; finished: 2026-11-06T21:06:55.169176+00:00; independent harness wall time: 14.210 s.
- Pinned Python baseline: Proselint 0.16.0 at `dbed789caae662d06c7c8a5a13dd31f1acd36f5c`; source tree SHA-256 `73d46eb1a461a379eac0e3c3418e1f941b120a19e13d5ae0f4413ea2cee3b831`; executable SHA-256 `b5f88a67377a8f3c32e1bffc41ca96fd8c111b127427459ffb2184e74c99af77`.
- Candidate fingerprint: `b2cfa57b4e9e8ce4a6d3caee401af6e75283a89bcfd1292af28e671278531f65`; attested binary SHA-256 `5e40fa722a8567cc9a036b499d3b674274df7609dadde7fb688485a2609983e1`. The source fingerprint was identical before and after the run and matched the build attestation.
- Independently inspected `proselint/checks/spelling/athletes.py`, `registry/checks/types.py` (`PreferredFormsSimple`), `registry/checks/engine.py` (`WORDS_IN_TEXT`), and the default `allow_quotes=False` flag.
- Scope: selected-rule parity for the 29 recorded cases only; this does not establish default-config or full CLI parity.

Machine report: [`independent-spelling-athletes-live-20261106.json`](independent-spelling-athletes-live-20261106.json).
