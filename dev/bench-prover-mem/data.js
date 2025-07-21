window.BENCHMARK_DATA = {
  "lastUpdate": 1753107099669,
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
          "id": "a84161c9dc221e0f1f0b76dbef4cfafd44e27830",
          "message": "feat(prover): clear and extract inputs as traces are written (#126)\n\n* clear and extract inputs as traces are written\n\n* remove capacity\n\n* improve runner import memory\n\n* fix comment\n\n* fix\n\n* fix bench\n\n* fix bench\n\n* fix bench\n\n* fix rebase\n\n* fix (again)\n\n* fix",
          "timestamp": "2025-06-27T20:52:59+02:00",
          "tree_id": "610cc75cb3577e34eeeea93281252f7dada3ff25",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/a84161c9dc221e0f1f0b76dbef4cfafd44e27830"
        },
        "date": 1751051150269,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2276004780,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "71888134+zmalatrax@users.noreply.github.com",
            "name": "malatrax",
            "username": "zmalatrax"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "bb1b35a8e4c80cfe72016d0d67ba6401b23d7723",
          "message": "feat(prover): add PCS config (#137)\n\n* feat: add PCS config\n\n* feat: add hardcoded 96 bit stark security level\n\n* refactor: add pcs config to prove and verify function signatures\n\n* feat: update pcs config to 16 pow bits",
          "timestamp": "2025-07-07T16:37:08+02:00",
          "tree_id": "9529e4155aa1888d2f070b8d670fe4fde8607671",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/bb1b35a8e4c80cfe72016d0d67ba6401b23d7723"
        },
        "date": 1751899799526,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2276138860,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "antoine.fondeur@gmail.com",
            "name": "Antoine Fondeur",
            "username": "AntoineFONDEUR"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "e74fdc5d105588cc1be094fcb8427884524f4ca1",
          "message": "feat(prover): unit constraint checks (#127)\n\n* unitary constraint checks\n\n* refacto\n\n* remove common.rs file\n\n* remove mod.rs files\n\n* rebase\n\n* fix compatibilty with rebase stwo\n\n* removed individual tests for each opcode",
          "timestamp": "2025-07-07T17:14:31+02:00",
          "tree_id": "d61415427ffdada4518a788d3663ade7d7f78099",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/e74fdc5d105588cc1be094fcb8427884524f4ca1"
        },
        "date": 1751902054477,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2276138348,
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
          "id": "e3bd6a375d386b2817e9591e156cc87f75b2b8b8",
          "message": "soundness(prover): Split relation column using mul (#128)\n\n* Split rel column using op0 * op1\n\n* Split rel column using op0 * imm\n\n* Split rel column using op0 * imm in jmp_abs_mul_fp_fp\n\n* Split rel column using op0 * imm in jmp_rel_mul_fp_fp\n\n* Split rel column using op0 * imm in jmp_abs_mul_fp_imm\n\n* Split rel column using op0 * imm in jmp_rel_mul_fp_imm",
          "timestamp": "2025-07-07T17:14:48+02:00",
          "tree_id": "6d1bc4d35b134f5fa8e228e269c7131d7bd8a4f9",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/e3bd6a375d386b2817e9591e156cc87f75b2b8b8"
        },
        "date": 1751902062092,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2276156780,
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
          "id": "ead6a8ce88586c791e165cf0ee9d96633fb478ce",
          "message": "fix(prover): Add None for pcs config (#138)",
          "timestamp": "2025-07-08T14:20:06+02:00",
          "tree_id": "edbf30ad348c53d3f351594aa51ed6c82fd5b5fb",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/ead6a8ce88586c791e165cf0ee9d96633fb478ce"
        },
        "date": 1751977993805,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2276156780,
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
          "id": "be06b174546ac20900ab1ca4fdf3ab152de150a1",
          "message": "feat(prover): Use macro to define opcodes (#141)\n\n* Add None for pcs config\n\n* Add a macro to add opcodes",
          "timestamp": "2025-07-08T18:02:03+02:00",
          "tree_id": "e521f073f1cfad4ce4854731469ddcada035214e",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/be06b174546ac20900ab1ca4fdf3ab152de150a1"
        },
        "date": 1751991304717,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2276156268,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "antoine.fondeur@gmail.com",
            "name": "Antoine Fondeur",
            "username": "AntoineFONDEUR"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "424ceaef732711353191c792e60e804ec798518d",
          "message": "refacto: remove unused opcodes (#140)\n\n* rebase\n\n* remove numbers of opcodes",
          "timestamp": "2025-07-09T13:18:43+02:00",
          "tree_id": "b695dc5f6d4188571b082da6f8945b05f8607436",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/424ceaef732711353191c792e60e804ec798518d"
        },
        "date": 1752060647975,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2275619180,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "antoine.fondeur@gmail.com",
            "name": "Antoine Fondeur",
            "username": "AntoineFONDEUR"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "053338b3824c34d78db60933c9dee397f6a0c0bf",
          "message": "feat(prover): prepare inputs to Merkle component (#139)\n\n* non optimal memory-wise merkle tree building\n\n* remove out.txt file\n\n* updated memory boundaries, hash on M31 and add hash as generic\n\n* switch to depth rather than layer\n\n* remove root hash from node data vec",
          "timestamp": "2025-07-09T15:09:23+02:00",
          "tree_id": "8fd081290d9c37989cdf61e6cd11b73cdfc9dfbb",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/053338b3824c34d78db60933c9dee397f6a0c0bf"
        },
        "date": 1752067303719,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2275624168,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "antoine.fondeur@gmail.com",
            "name": "Antoine Fondeur",
            "username": "AntoineFONDEUR"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d89b7cdf617e26e82a86ce5a82c5b26c8388e9d1",
          "message": "feat(prover): add a Merkle relation and update the PublicData (#143)\n\n* added roots to public data\n\n* rebase\n\n* add the merkle relation",
          "timestamp": "2025-07-10T10:57:03+02:00",
          "tree_id": "3bdbf5d4a0eb53fa7da0dcfb42cf0c027dfda53a",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/d89b7cdf617e26e82a86ce5a82c5b26c8388e9d1"
        },
        "date": 1752138580509,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2275624168,
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
          "id": "e507a29d15e349b2676b8469054efe58067aaf02",
          "message": "Rename RangeCheck_20 to RangeCheck20 for case consistency (#152)",
          "timestamp": "2025-07-11T16:43:49+02:00",
          "tree_id": "f9a85ed4b87f2ea86afde1b1f94862346892873e",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/e507a29d15e349b2676b8469054efe58067aaf02"
        },
        "date": 1752245765838,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2275624148,
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
          "id": "50e16a1673d96f2b1c1865e39a9e32ff2ab675f7",
          "message": "Get relations as input and not list of relations (#153)",
          "timestamp": "2025-07-11T17:33:26+02:00",
          "tree_id": "6b9030302730685984191db943496200b1195326",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/50e16a1673d96f2b1c1865e39a9e32ff2ab675f7"
        },
        "date": 1752248741406,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2275624148,
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
          "id": "adde39448877736fdcc2a958b118efff02f6dd66",
          "message": "chore: fmt (#160)\n\nCo-authored-by: enitrat <msaug@protonmail.com>",
          "timestamp": "2025-07-16T14:01:05+02:00",
          "tree_id": "d79d668ed4acb0e2042c2b21b9cefc4d17fbf62e",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/adde39448877736fdcc2a958b118efff02f6dd66"
        },
        "date": 1752668040156,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2275624072,
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
          "id": "b7f2a7363e6c346c63edbd6e61513b2efdd59239",
          "message": "refactor: remove opcode_id column from opcode components (#157)\n\n- Remove redundant opcode_id column from trace masks in evaluate() functions\n- Replace opcode assertion constraints with direct constant usage in memory relations\n- Update header documentation to remove opcode_id column and constraint descriptions\n- Reduce N_TRACE_COLUMNS constants to reflect column removal\n- Use compile-time Opcode constants directly in memory relations instead of runtime opcode_id\n\nBenefits:\n- Reduced memory usage (one fewer column per component)\n- Simplified logic by removing redundant opcode validation\n- Better performance with compile-time constants\n- Cleaner, more maintainable code structure\n\nComponents refactored:\n- store_imm.rs: 11 → 10 columns\n- store_deref_fp.rs: 14 → 13 columns\n- store_double_deref_fp.rs: 15 → 14 columns\n- store_add_fp_imm.rs: 13 → 12 columns\n- store_add_fp_imm_inplace.rs: 11 → 10 columns\n- store_add_fp_fp.rs: 15 → 14 columns\n\nAll tests pass and functionality is preserved.",
          "timestamp": "2025-07-16T15:29:02+03:00",
          "tree_id": "e28e7bd0e7913e521c048d250541af6f0ad60eb4",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/b7f2a7363e6c346c63edbd6e61513b2efdd59239"
        },
        "date": 1752669700578,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2261456744,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "34771985+gael-bigot@users.noreply.github.com",
            "name": "Gaël Bigot",
            "username": "gael-bigot"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "dcc6828f09f251d6a5ea38d239b10b3fc856771f",
          "message": "fix(compiler): avoid inplace store operations (#155)\n\n* fix(compiler): avoid inplace store operations\n\n* fix prover test + claude critical issues\n\n* refactor + tests: apply suggestions\n\n* changed fib_loop throughout codebase to avoid b = a + b",
          "timestamp": "2025-07-16T14:44:09+02:00",
          "tree_id": "ba107477cb6803a93c3434bb3705b22c7d1bfa58",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/dcc6828f09f251d6a5ea38d239b10b3fc856771f"
        },
        "date": 1752670689878,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2478509336,
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
          "id": "acaa7bae104a5bb1430a2a90459b2d4519b3e010",
          "message": "remove unused columns (#163)",
          "timestamp": "2025-07-17T11:24:30+02:00",
          "tree_id": "9a0eeb616795c0027e03467a75492bcdec9df348",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/acaa7bae104a5bb1430a2a90459b2d4519b3e010"
        },
        "date": 1752745102949,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2470638200,
            "unit": "bytes"
          }
        ]
      },
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
          "id": "fb12be98e385d392af128555aa42fa05afdb548b",
          "message": "feat(prover): dump proof output (#168)\n\n* feat(prover): dump proof output\n\n* feat(prover): dump proof output to json file\n\n* recs",
          "timestamp": "2025-07-17T12:32:57+02:00",
          "tree_id": "c9891af36942431c180bb3bfcbff82ca09623fb1",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/fb12be98e385d392af128555aa42fa05afdb548b"
        },
        "date": 1752749179676,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2338524568,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "antoine.fondeur@gmail.com",
            "name": "Antoine Fondeur",
            "username": "AntoineFONDEUR"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b757a5544089d0ced055c16e706dc29e49f30ccc",
          "message": "feat(prover): add the merkle component (#149)\n\n* added merkle component\n\n* removed generic hash in prover_cairo_m\n\n* review modifications\n\n* deleted poseidon file",
          "timestamp": "2025-07-18T10:00:09+02:00",
          "tree_id": "f630569d492d02b3d881b158347a2e6575a58f09",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/b757a5544089d0ced055c16e706dc29e49f30ccc"
        },
        "date": 1752826384203,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2338642400,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "antoine.fondeur@gmail.com",
            "name": "Antoine Fondeur",
            "username": "AntoineFONDEUR"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7878461f261d5a405a81e23d72b28937a563a3c4",
          "message": "created utils folder and removed inplace operations (#171)",
          "timestamp": "2025-07-18T15:52:08+02:00",
          "tree_id": "a90e751bdc7b968c86949f9d6482cf46dc097404",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/7878461f261d5a405a81e23d72b28937a563a3c4"
        },
        "date": 1752847502603,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2314470146,
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
          "id": "52df73080408295fdf5628f1fff79e63cb5db784",
          "message": "feat(prover): Remove StoreDerefFp opcode and replace with StoreAddFpImm (#174)\n\n* Fix typos\n\n* Vibecode it",
          "timestamp": "2025-07-18T17:06:34+02:00",
          "tree_id": "bc4a33b59ac85c1336128bd44c47a66cd1cbdffe",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/52df73080408295fdf5628f1fff79e63cb5db784"
        },
        "date": 1752851932274,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2228998819,
            "unit": "bytes"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "155267459+reallesee@users.noreply.github.com",
            "name": "Micke",
            "username": "reallesee"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "d0ea827c5505bdd49f8ab02e1ea5660ae6ac15b2",
          "message": "refactor(prover): optimize array filling in Enabler::packed_at (#176)",
          "timestamp": "2025-07-21T12:01:51+02:00",
          "tree_id": "3115627bba230e4ef3315f4d6f916f601f693ff7",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/d0ea827c5505bdd49f8ab02e1ea5660ae6ac15b2"
        },
        "date": 1753092862743,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2228999331,
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
          "id": "9a9b47179d84b13b29016be0cea9ae8638d75c15",
          "message": "feat(prover): Consolidate JnzFpImm opcode implementation (#175)\n\n* Vibecode it\n\n* Add PR claude command\n\n* Make sure that taken is the bool version of op0 * op0_inv\n\n* Add pc_new col to save one interaction",
          "timestamp": "2025-07-21T15:59:13+02:00",
          "tree_id": "4435311b9a662c38c2d7041ccbb9d535aac113d8",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/9a9b47179d84b13b29016be0cea9ae8638d75c15"
        },
        "date": 1753107099628,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2220053156,
            "unit": "bytes"
          }
        ]
      }
    ]
  }
}