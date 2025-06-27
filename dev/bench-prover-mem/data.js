window.BENCHMARK_DATA = {
  "lastUpdate": 1751043245872,
  "repoUrl": "https://github.com/kkrt-labs/cairo-m",
  "entries": {
    "Cairo-M Prover Memory Benchmarks": [
      {
        "commit": {
          "author": {
            "email": "60658558+enitrat@users.noreply.github.com",
            "name": "Mathieu",
            "username": "enitrat"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7cc842c1b192964ce97d5a16fa6998cfee105992",
          "message": "ci: add benchmarks on prover crate (#121)\n\n* dev: add benchmarks on prover\n\n* fix ghpage issue\n\n* fixes\n\n* updat n_iter to 100k\n\n* count vm adapter in benchmark",
          "timestamp": "2025-06-27T11:51:29+02:00",
          "tree_id": "9c35e2405045debf9490749a5bf126b4e3c39888",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/7cc842c1b192964ce97d5a16fa6998cfee105992"
        },
        "date": 1751018410576,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 1964610240,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "clement0walter@gmail.com",
            "name": "Clément Walter",
            "username": "ClementWalter"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ca49f5961620aead146de363645301287a601382",
          "message": "Implement all opcodes (#113)\n\n* Boilerplate opcode components\n\n* Implement all components\n\n* Fix rebase\n\n* Fix num interaction col\n\n* Use enabler in lookup instead of one\n\n* Remove previous store_deref_fp\n\n* Fix trunk\n\n* Add jnz_*_taken opcodes, to be implemented\n\n* Implement jnz opcodes\n\n* Derive Debug in (Interaction)Claim\n\n* Fix opcodes::range_check_20 only uses first opcode\n\n* Use one for range_check mult\n\n* Remove dbg\n\n* Fix missing opcodes in claimed_sum gathering\n\n* Add store_inplace\n\n* Fix ret lookup order\n\n* Fix call writes to op0\n\n* Fix call writes to op0 + 1\n\n* Fix inplace use op0 instead of op0_prev\n\n* feat(prover): add constraint checker (#122)\n\n* add constraint checker\n\n* rebase\n\n* add component trackign\n\n* Fix inplace num and initial logup sum\n\n* Fix column order in store_deref_fp\n\n* Fix dst value in store_deref_fp\n\n* Remove dbg\n\n* Fix StateData became ExecutionBundle\n\n* Update debug tools\n\n---------\n\nCo-authored-by: Antoine Fondeur <antoine.fondeur@gmail.com>",
          "timestamp": "2025-06-27T11:53:00+02:00",
          "tree_id": "a18f29f2cf667b8160520fc39f2cb72f528b97fa",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/ca49f5961620aead146de363645301287a601382"
        },
        "date": 1751018737743,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2428608496,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "obatirou@gmail.com",
            "name": "Oba",
            "username": "obatirou"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "874c88d63b6fb0d20184e9dfb34624d50d3d5102",
          "message": "feat(prover): profile (#125)\n\n* profile\n\n* Refactor",
          "timestamp": "2025-06-27T17:43:48+02:00",
          "tree_id": "94aceb040f3e759a3f2c97fe67c7c6d9465fe090",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/874c88d63b6fb0d20184e9dfb34624d50d3d5102"
        },
        "date": 1751039794113,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2428608464,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "66871571+Eikix@users.noreply.github.com",
            "name": "Elias Tazartes",
            "username": "Eikix"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "99f983c71dfe973efa9a434556ecb2dca174a1c0",
          "message": "rebase stwo (#129)",
          "timestamp": "2025-06-27T18:41:14+02:00",
          "tree_id": "db8cc53d2db718c9d12ee6257f47088eed09abda",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/99f983c71dfe973efa9a434556ecb2dca174a1c0"
        },
        "date": 1751043245832,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2428608464,
            "unit": "bytes"
          }
        ]
      }
    ]
  }
}