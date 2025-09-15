window.BENCHMARK_DATA = {
  "lastUpdate": 1757948263404,
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
          "id": "0f49972b20474c255c1877c66d601543636abc9a",
          "message": "feat(runner): split execution for continuation (#164)\n\n* feat(runner): split execution for continuation\n\n* review fixes\n\n* use vec with capacity for memory serialization\n\n* added doc for ExecutionStatus\n\n* review modifs\n\n* updated the vm benchmark\n\n* remove cloning for single segment executions\n\n* replace n_steps by max_steps",
          "timestamp": "2025-07-22T11:52:46+02:00",
          "tree_id": "4a63737008d11d2861d0d6cab0c3ea3bbff40ec8",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/0f49972b20474c255c1877c66d601543636abc9a"
        },
        "date": 1753178718810,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2220054712,
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
          "id": "eee15dbe986c752264990425790bd3af8ba3e96d",
          "message": "feat(prover): add public addresses (#185)\n\n* add public addresses\n\n* fixed typo for public addresses\n\n* added comment",
          "timestamp": "2025-07-23T17:23:39+02:00",
          "tree_id": "59434d35afa0605359ef9f30681aa9f159a72ab0",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/eee15dbe986c752264990425790bd3af8ba3e96d"
        },
        "date": 1753285009933,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2220055736,
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
          "id": "81a4c011c9ddd434d2078b9828e90f8d28022c27",
          "message": "feat(prover): implement the poseidon hash (#183)\n\n* implement poseison hash\n\n* remove snapshots\n\n* review modifications\n\n* review modifications\n\n* review modifications\n\n* fixes\n\n* Update crates/prover/src/utils/poseidon/poseidon_params.rs\n\nCo-authored-by: Antoine Fondeur <antoine.fondeur@gmail.com>\n\n* Update crates/prover/src/utils/poseidon/poseidon_params.rs\n\n---------\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>",
          "timestamp": "2025-07-25T12:13:16+02:00",
          "tree_id": "d557c8c14b2f54f4ef0d8659ddba2538a84dddec",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/81a4c011c9ddd434d2078b9828e90f8d28022c27"
        },
        "date": 1753439151427,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2220055736,
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
          "id": "1d34c87b6e87b3472299b1349963fa3ebca94d80",
          "message": "feat(prover): factor components with equal lookup operations (#200)\n\n* Update macro to support list of opcodes\n\n* Merge jmp imm opcodes\n\n* merge store_fp_fp\n\n* Fix tests\n\n* merge store_fp_imm\n\n* Use saturatin_sub",
          "timestamp": "2025-07-31T11:06:40+02:00",
          "tree_id": "8e1efe189c59ba8094b0f865e90e2fa5315de6d8",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/1d34c87b6e87b3472299b1349963fa3ebca94d80"
        },
        "date": 1753953623429,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2382901023,
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
          "id": "7a3e6806ebd7c46b1d640754526eedcad75e3317",
          "message": "feat(prover): add clock update component (#199)\n\n* added update clock data to prover input\n\n* added update clock component\n\n* fixes\n\n* review modifs\n\n* remove useless rangecheck lookup\n\n* update cli for release mode\n\n* Update crates/prover/src/components/clock_update.rs\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>\n\n* Update crates/prover/src/components/clock_update.rs\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>\n\n* Delete crates/prover/src/preprocessed/poseidon_rc.rs\n\n---------\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>",
          "timestamp": "2025-07-31T12:00:11+02:00",
          "tree_id": "ad991379b797cc24f50b25af3c63c1eb3d503b35",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/7a3e6806ebd7c46b1d640754526eedcad75e3317"
        },
        "date": 1753956801335,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2382911113,
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
          "id": "f04c147ad2842ed89f467cb4939d7d86d499c8cf",
          "message": "feat(prover): poseidon2 component (#205)\n\n* add poseidon2 component\n\n* review modifs\n\n* patch thing\n\n* fix naming inconsistency between X_value and value_X\n\n* trunk\n\n* delete unused P and P_MODULULS_BIT\n\n* Apply suggestion from @AntoineFONDEUR\n\nCo-authored-by: Antoine Fondeur <antoine.fondeur@gmail.com>\n\n* Update crates/prover/src/components/poseidon2.rs\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>\n\n* Update crates/prover/src/components/poseidon2.rs\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>\n\n* Update crates/prover/src/components/poseidon2.rs\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>\n\n* review\n\n---------\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>",
          "timestamp": "2025-07-31T17:19:59+02:00",
          "tree_id": "a3a664e8ab9197d95afd10f41c3cd15b2d099a9d",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/f04c147ad2842ed89f467cb4939d7d86d499c8cf"
        },
        "date": 1753976071847,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2384499479,
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
          "id": "84d59b1395dcccb3b6ed42f87408a13336746c78",
          "message": "epic: support variable instruction size (#195)\n\n* feat(common): variable instructions size (#178)\n\n* feat(common): variable instructions size\n\n* remove StoreDerefFp\n\n* adds comments for define_instruction\n\n* feat(compiler): Adapt Compiler Codegen for Variable-Sized Instructions (#179)\n\n* feat(compiler): Adapt Compiler Codegen for Variable-Sized Instructions\n\n* refactor: remove hardcoded opcode ids\n\n* ci: run tests on feature branches\n\n* refactor\n\n* add snapshot variable instructions\n\n* feat(runner): Adapt Runner for Variable-Sized Instructions (#182)\n\n* feat(prover): Adapt Prover Adapter and Bundles for U32 Support (#196)\n\n* feat(prover): Adapt Prover Adapter and Bundles for U32 Support\n\n* refactor\n\n* update store_imm documentation\n\n* PR comments\n\n* panic on failure to get operand type\n\n---------\n\nCo-authored-by: enitrat <msaug@protonmail.com>\n\n* remove unused variable\n\n* fix rebase\n\n---------\n\nCo-authored-by: enitrat <msaug@protonmail.com>\nCo-authored-by: Antoine FONDEUR <antoine.fondeur@gmail.com>",
          "timestamp": "2025-08-01T14:33:16+02:00",
          "tree_id": "21bcc1fdfffdfff537d7be210e38e9cd86456a2c",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/84d59b1395dcccb3b6ed42f87408a13336746c78"
        },
        "date": 1754052443577,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2413859951,
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
          "id": "44356825d3ef6ae29edd4d7297fc28e64e4b2e0e",
          "message": "dev: trunk fmt all (#216)",
          "timestamp": "2025-08-01T19:01:33+02:00",
          "tree_id": "902c7b277a6ca28677391722bc9d0856346937f6",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/44356825d3ef6ae29edd4d7297fc28e64e4b2e0e"
        },
        "date": 1754068505841,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2413859951,
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
          "id": "96d7f071c012ffb3a1d5180629efb2f3a4fa9ff5",
          "message": "doc(prover): add documentation for prover crate (#225)\n\n* first batch of comments\n\n* add debug_tools comments\n\n* add a readme for components and some doc for range_check\n\n* fix trunk\n\n* fix typo\n\n* review fix\n\n* keep trunk quiet\n\n* fix",
          "timestamp": "2025-08-07T14:38:34+02:00",
          "tree_id": "441b0e274e2efb5ffa89b03e04a7b7cf619acf91",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/96d7f071c012ffb3a1d5180629efb2f3a4fa9ff5"
        },
        "date": 1754571129707,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2413860443,
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
          "id": "07282de0487caaaf850c199454e13700a9dbe8f6",
          "message": "feat: implement comprehensive mdtest markdown-based testing system (#226)\n\n* feat(test): centralize test data across workspace with auto-discovery\n\nBREAKING CHANGE: Test structure reorganization\n\n## Changes\n- Created centralized `test_data/` directory at workspace root\n- Added `cairo_m_test_utils` helper crate for test fixture management\n- Implemented auto-discovery tests for codegen and MIR using insta's glob\\! macro\n- Removed duplicate test data from individual crates\n- Updated all test crates to use shared fixtures\n\n## Benefits\n- Single source of truth for all test fixtures\n- Automatic test generation for all fixtures in test_data/\n- No more duplicate test files across crates\n- Easier maintenance and better discoverability\n- All snapshots generated in a single test run\n\n## Test Utils API\n- `fixture_path(name)` - Get path to fixture\n- `read_fixture(name)` - Load fixture contents\n- `discover_all_fixtures()` - List all fixtures\n- `test_data_path()` - Get test_data directory path\n\n* feat: add mdtests markdown files\n\n* feat: add util files to discover & parse mdtests\n\n* feat: implement diff-testing in runner for mdtests\n\n* dev: expand pattern to convert to valid rust in tests\n\n* refactor(test): eliminate duplication in mdtest runners with generic infrastructure\n\n- Created generic MdTestRunner to eliminate ~70% code duplication between MIR and codegen test files\n- Fixed parser edge cases when Cairo-M blocks appear before headings\n- Removed noisy eprintln\\! statements from test output\n- Documented Location struct's line number approximation limitation\n- Reduced test files from ~140 lines to ~40 lines each\n\nThe refactoring improves maintainability by centralizing test running logic while preserving all existing functionality.\n\n* feat(mdtest): add comprehensive test coverage and support multiple snippets per section\n\n- Add support for multiple Cairo-M snippets in same markdown section with automatic numbering\n- Implement H3 heading support for more granular test naming\n- Add automatic renaming of main functions in Rust code to avoid conflicts\n- Create extensive new test files covering arrays, expressions, multiple functions, mutual recursion, optimization patterns, error handling, and opcodes\n- Update mdtest README with comprehensive documentation on test generation, annotations, and best practices\n- Fix parser to properly handle multiple tests per section without Rust code duplication\n\nThis completes the mdtest migration from scattered MIR/Codegen test files to a unified markdown-based testing system that serves as both documentation and validation.\n\n* refill snapshots\n\n* fmt\n\n* update snapshot naming\n\n* fix last tests\n\n* fix benchmarks",
          "timestamp": "2025-08-07T17:12:14+02:00",
          "tree_id": "b8517812aa33b71dc278cc721da39f450c89075e",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/07282de0487caaaf850c199454e13700a9dbe8f6"
        },
        "date": 1754580401767,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2413860454,
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
          "id": "1a50c20aaaa42f0648c590185b1b99d67b00b455",
          "message": "feat(prover): updated public memory (#189)\n\n* update public memory and refacto merkle tree\n\n* fix rebase\n\n* fix: fix rebase\n\n---------\n\nCo-authored-by: malatrax <71888134+zmalatrax@users.noreply.github.com>",
          "timestamp": "2025-08-20T14:05:42+02:00",
          "tree_id": "7a1424080579b64fa9e2e3a42e9b110f610f0b05",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/1a50c20aaaa42f0648c590185b1b99d67b00b455"
        },
        "date": 1755692498561,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2630895350,
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
          "id": "c84a89b772b092b0b47a089ca4ea1fafb135c938",
          "message": "feat: add program id (#232)\n\n* feat: add program id\n\n* fix: use poseidon2 hash",
          "timestamp": "2025-08-21T14:33:33+02:00",
          "tree_id": "02d59dc4ba2be20fc6a18a96afb404fc8257b6d7",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/c84a89b772b092b0b47a089ca4ea1fafb135c938"
        },
        "date": 1755780505285,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2630895862,
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
          "id": "49c46099a63aa946762f1c4c0c4c354932af85af",
          "message": "refactor(common): replace CairoMSerialize with typed ABI codec layer (#237)\n\n* refactor(abi): replace CairoMSerialize with typed ABI codec layer\n\nReplace the old slot-based serialization system with a proper typed ABI layer\nthat provides better type safety, validation, and maintainability.\n\n## What Changed\n\n- **New ABI codec system** (`abi_codec.rs`):\n  - Typed `AbiType` enum with manual serde for stable JSON format\n  - Type-safe encoding/decoding with `CairoMValue` and `InputValue` enums\n  - Comprehensive validation (bool values 0/1, u32 range checks)\n  - CLI argument parser supporting nested tuples/structs\n  - Proper error types (`AbiCodecError`) instead of anyhow in library code\n\n- **Improved compiler integration**:\n  - MIR types now map directly to ABI types\n  - Program metadata includes typed parameter/return information\n  - Entrypoints carry full ABI slot metadata\n\n- **Enhanced runner/prover**:\n  - Typed value handling throughout execution\n  - Better CLI documentation with examples\n  - Support for tuple syntax: `(1,2,3)` or `[1,2,3]`\n\n- **Testing infrastructure**:\n  - Comprehensive proptest-based round-trip testing\n  - Structured test organization (edge_cases, parser, integration)\n  - Removed redundant tests covered by property testing\n  - Differential test harness supports nested structures\n\n## Why\n\nThe previous `CairoMSerialize` system used untyped \"slot math\" which was error-prone\nand difficult to extend. The new ABI layer provides:\n- Type safety at encode/decode boundaries\n- Better error messages for users\n- Foundation for future features (arrays, more complex types)\n- Cleaner separation between compiler and runtime concerns\n\n## Notes\n\n- Fixed-size arrays marked as unsupported with TODO (Linear issue CORE-1118)\n- All tests passing, including property-based tests\n- Backwards compatible JSON format for programs\n\n* refactor(runner): simplify API to return RunnerOutput directly\n\nRemove tuple return type from run_cairo_program. The function now returns\nRunnerOutput directly, which contains both decoded return_values (Vec<CairoMValue>)\nand the VM state. This simplifies the API and makes it more intuitive to use.\n\nThe raw M31 values are no longer included in RunnerOutput since they can be\nderived from the decoded values when needed for proof generation.\n\nBREAKING CHANGE: run_cairo_program now returns RunnerOutput instead of\n(Vec<CairoMValue>, RunnerOutput). Update call sites to access return_values\nfrom the RunnerOutput struct.\n\n* fix prover tests\n\n* add into impl for InputValue::Number and u32\n\n* Update crates/common/src/abi_codec.rs\n\nCo-authored-by: Oba <obatirou@gmail.com>\n\n* Update crates/common/src/abi_codec.rs\n\nCo-authored-by: Oba <obatirou@gmail.com>\n\n* suggestions\n\n* remove unused import\n\n---------\n\nCo-authored-by: Oba <obatirou@gmail.com>",
          "timestamp": "2025-08-25T16:23:36+02:00",
          "tree_id": "b7ae3fec35b1e84304e2d32b1b1b6be1e3a06560",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/49c46099a63aa946762f1c4c0c4c354932af85af"
        },
        "date": 1756132648822,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2415357506,
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
          "id": "76497906a59705be4dd0fa47b5403974ab39cdc6",
          "message": "Fix cspell configuration and correct typos across codebase (#252)\n\n* Update config and fix all\n\n* Take stwo typo into account\n\n* Just run trunk check --all --fix\n\n* Move cspell config to root to have it working from trunk and plain cspell checker\n\n* Remove wrong capital letter\n\n---------\n\nCo-authored-by: Mathieu <60658558+enitrat@users.noreply.github.com>",
          "timestamp": "2025-08-26T18:38:27+03:00",
          "tree_id": "541e4db86ba9669bda810e2669437d6438efd4c1",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/76497906a59705be4dd0fa47b5403974ab39cdc6"
        },
        "date": 1756223583443,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2415357506,
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
          "id": "3f94ce4b65dc87087369b891a5ff92ad70d49666",
          "message": "Add compile-time check for opcode AIR implementation coverage (#260)\n\n* Remove useless const in macro\n\n* Re-add compile check for opcode AIR implementation",
          "timestamp": "2025-08-28T10:51:15+03:00",
          "tree_id": "c40efa9031edf51c69be95731c8020d4954a9929",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/3f94ce4b65dc87087369b891a5ff92ad70d49666"
        },
        "date": 1756368328099,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2415357506,
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
          "id": "f74c42057d81ae44442b9508cbe3184e12dd5740",
          "message": "refacto: remove unused opcodes (#262)",
          "timestamp": "2025-08-28T11:17:43+03:00",
          "tree_id": "2ceb4fb97e1dfc5b84d9ce3f697d9effc435acda",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/f74c42057d81ae44442b9508cbe3184e12dd5740"
        },
        "date": 1756369942883,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2415357506,
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
          "id": "72acbfb169187277ce4ec12fa4c6b80fc0362c10",
          "message": "feat(prover): add assert_eq components (#263)\n\n* feat: add assert_eq opcodes (un-tested)\n\n* doc: fixed comments\n\n* typo: fix copy pasting typos",
          "timestamp": "2025-08-28T12:04:22+03:00",
          "tree_id": "43bc5a2184b26e8fb708713aeb013c0012c9fa34",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/72acbfb169187277ce4ec12fa4c6b80fc0362c10"
        },
        "date": 1756372774655,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2415412896,
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
          "id": "a86e3a5c7dc567537bce4a3e8d5c7990c024c0e2",
          "message": "feat(prover): add range_check 8 and 16 (#261)\n\n* feat: add range_check 8 and 16\n\n* refacto: define macros in the mod.rs file\n\n* feat: add trait for range_checks\n\n* refacto: deal with range_check_20 the same way as 8 and 16 are dealt with\n\n* refacto: import trait in opcodes\n\n* rebase: rebase fix",
          "timestamp": "2025-08-28T15:47:19+02:00",
          "tree_id": "352c739f2d71f8263b89435e8e86084bfc30b58b",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/a86e3a5c7dc567537bce4a3e8d5c7990c024c0e2"
        },
        "date": 1756389763585,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2422804050,
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
          "id": "992e7a68ef20d17155ee249ee3db6a3650dd48e8",
          "message": "feat(prover): add u32_store_imm (#269)\n\n* feat: add u32_store_imm (tested)\n\n* rebase: fix rebase\n\n* typo: copy-paste typo",
          "timestamp": "2025-08-28T16:09:01+02:00",
          "tree_id": "73380e3be360edd415b296a9fd46df77f01447c7",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/992e7a68ef20d17155ee249ee3db6a3650dd48e8"
        },
        "date": 1756391071995,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2422836705,
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
          "id": "80b8e1ee10ce6591ddf6117a64b5f307656ce24d",
          "message": "feat(compiler-runner): simplify instruction set by removing redundant opcodes (#265)\n\n* refactor: simplify instruction set by removing redundant opcodes\n\n- Remove felt arithmetic immediate opcodes that can be derived:\n  - StoreSubFpImm -> StoreAddFpImm with negated immediate\n  - StoreDivFpImm -> StoreMulFpImm with inverse immediate\n\n- Remove U32 arithmetic immediate opcodes:\n  - U32StoreSubFpImm -> U32StoreAddFpImm with two's complement\n\n- Remove U32 comparison opcodes that can be derived:\n  - U32StoreNeqFpFp -> 1 - U32StoreEqFpFp\n  - U32StoreGtFpFp -> U32StoreLtFpFp with swapped operands\n  - U32StoreGeFpFp -> 1 - U32StoreLtFpFp\n  - U32StoreLeFpFp -> U32StoreGeFpFp with swapped operands\n\n- Remove U32 comparison immediate opcodes that can be derived:\n  - U32StoreNeqFpImm -> 1 - U32StoreEqFpImm\n  - U32StoreGtFpImm -> 1 - U32StoreLeFpImm\n  - U32StoreGeFpImm -> 1 - U32StoreLtFpImm\n  - U32StoreLeFpImm -> U32StoreLtFpImm with biased immediate\n\nUpdate codegen to compile removed opcodes into optimized sequences\n\n* update tests\n\n* make better comments for CASM instrs\n\n* fmt\n\n* refactor(codegen): modularize CasmBuilder into focused submodules\n\nSplit the monolithic builder.rs (2933 LOC) into specialized modules:\n- aggregates: struct/tuple operations (855 LOC)\n- calls: function call handling (427 LOC)\n- felt: field arithmetic operations (372 LOC)\n- u32_ops: unsigned integer operations (379 LOC)\n- store: memory/register operations (302 LOC)\n- ctrlflow: control flow constructs (277 LOC)\n- normalize: value normalization (156 LOC)\n- opcodes: instruction emission (115 LOC)\n- emit: label/touch utilities (36 LOC)\n- asserts: assertion helpers (22 LOC)\n\nThis improves code organization, maintainability, and compile times\nwhile preserving all existing functionality.\n\n* refactor(mir): remove broken optimization passes and simplify MIR pipeline\n\n      - Remove broken mem2reg passes: const_fold, ssa_destruction, var_ssa, lower_aggregates\n      - Simplify passes.rs by removing ~3000 lines of unused optimization code\n      - Update test infrastructure and snapshots to match simplified pipeline\n      - Clean up aggregate instruction tests and lowering logic\n      - Remove associated test files for deleted passes\n\n      This continues the MIR refactoring to focus on stable, working functionality\n      while removing complex optimization passes that were causing correctness issues.\n\n* test(codegen): add comprehensive property-based testing for CasmBuilder\n\n- Add test_support module with simple execution model for validating generated CASM\n- Implement property-based tests for all builder modules (aggregates, felt, store, u32_ops)\n- Test edge cases including overflow, division by zero, and boundary conditions\n- Add proptest regression files to catch future regressions\n- Update test snapshots to reflect improved codegen output\n- Remove obsolete WORK_PLAN.md documentation\n\nThis ensures CasmBuilder generates correct CASM instructions across all operations\nand edge cases, significantly improving codegen reliability.\n\n* restore instruction tests\n\n* cleanup\n\n* add legalize pass on MIR rather than in-place replacing instructions\n\n* code cleanup\n\n* some more factorization\n\n* update comments",
          "timestamp": "2025-08-29T18:25:03+02:00",
          "tree_id": "07b3a1670c48e54b0f10aa0ce943b0ee6f280390",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/80b8e1ee10ce6591ddf6117a64b5f307656ce24d"
        },
        "date": 1756485680345,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2422836705,
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
          "id": "675fb95c1096e567a5f1c4f7c248b68bf1ffb4c9",
          "message": "feat: add u32_store_add_fp_imm (#271)",
          "timestamp": "2025-09-01T09:46:57+02:00",
          "tree_id": "0e0d634665021cf45b0ede9239a92e5423b41a76",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/675fb95c1096e567a5f1c4f7c248b68bf1ffb4c9"
        },
        "date": 1756713751308,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2422885776,
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
          "id": "4f97acb42a61aec48069d1f18239f3d203a72339",
          "message": "feat(prover): add u32_store_mul_fp_imm (#272)\n\n* feat: add u32_store_mul_fp_imm\n\n* Update crates/prover/src/components/opcodes/u32_store_mul_fp_imm.rs\n\n* Update crates/prover/src/components/opcodes/u32_store_mul_fp_imm.rs",
          "timestamp": "2025-09-01T11:46:53+03:00",
          "tree_id": "92efd13dbce3277e01b72f84fe4551b934f131d4",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/4f97acb42a61aec48069d1f18239f3d203a72339"
        },
        "date": 1756717368915,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2422944591,
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
          "id": "235f7b6fde76396f6661891c1fbdbc19b90ba807",
          "message": "feat(compiler): add flag to disable opt passes (#286)\n\n* feat(compiler): add flag to disable opt passes\n\n* fix ci",
          "timestamp": "2025-09-01T12:49:01+02:00",
          "tree_id": "b4407779bff1a58a9c0aa2a84a7d124cee94411c",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/235f7b6fde76396f6661891c1fbdbc19b90ba807"
        },
        "date": 1756724684867,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2422955855,
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
          "id": "da9cf5f3459d1d3a774a288b8e2cead423d656d3",
          "message": "refactor(prover): fix adapter memory access (#284)\n\n* refactor(prover): simplify memory access tracking and value representation\n\n- Replace MemoryValue enum with direct M31 values for cleaner memory representation\n- Refactor instruction memory_accesses() to use macro-based limb counting\n- Remove unnecessary data type conversions in memory adapter\n\n* adapt u32_fp_imm opcodes\n\n* fix opcode tests\n\n* fix columns order\n\n* have consistent ordering for columns\n\n---------\n\nCo-authored-by: Antoine FONDEUR <antoine.fondeur@gmail.com>",
          "timestamp": "2025-09-01T14:31:00+02:00",
          "tree_id": "59ce7f0045606ba576a12909cd7163e09f81a13c",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/da9cf5f3459d1d3a774a288b8e2cead423d656d3"
        },
        "date": 1756730802526,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2439755615,
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
          "id": "71417e1ee3b09a547db80446a4ed6a7a8b72ae6d",
          "message": "feat: add u32_store_eq (#285)\n\n* feat: add u32_store_eq_fp_fp\n\n* remove out.txt\n\n* feat: add u32_store_eq_fp_imm\n\n* rebase\n\n* typo: fix copy paste errors",
          "timestamp": "2025-09-01T17:44:14+03:00",
          "tree_id": "3762d6ed16002be15c51d5cc504074678439616e",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/71417e1ee3b09a547db80446a4ed6a7a8b72ae6d"
        },
        "date": 1756738811334,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2440000007,
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
          "id": "c363e4bf0a91ca814442dce190de7e9cd9f462c6",
          "message": "refactor(prover): refactor instructions operand accesses (#288)\n\n* refactor(prover): extract memory access tracking into dedicated module\n\n- Create dedicated AccessLog module for memory access tracking\n- Move memory access logic from ExecutionBundle to AccessLog\n- Simplify ExecutionBundle by delegating to AccessLog\n- Fix memory operations to use new access tracking system\n- Update all opcode components to use new memory interface\n\nThis refactoring improves code organization by separating concerns:\nmemory access tracking is now handled independently from execution\nbundle management, making the codebase more maintainable.\n\n* fix conflicts of rebase\n\n* some cleanup",
          "timestamp": "2025-09-02T12:20:06+02:00",
          "tree_id": "a9453d45310b375d7df55bc46486cfb54a7a94f0",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/c363e4bf0a91ca814442dce190de7e9cd9f462c6"
        },
        "date": 1756809344933,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2377096455,
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
          "id": "609829fa6cde5a182b1df7ee12adc844ef617205",
          "message": "bug(prover): add SHA256 test and fix double deref memory accesses (#289)",
          "timestamp": "2025-09-02T16:10:57+02:00",
          "tree_id": "1e47fee98d9c5b79ffd71d379eed931a6b456942",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/609829fa6cde5a182b1df7ee12adc844ef617205"
        },
        "date": 1756823261877,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2377095975,
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
          "id": "2c20fb778bcbaab2b6e785956db8224745b7dd03",
          "message": "feat(prover): add u32_store_lt_{fp_fp/imm} (#270)\n\n* feat: add u32_store_lt_fp_fp and u32_store_lt_fp_imm\n\n* rebase: add range_check_20 implementation to new opcodes\n\n* typo: copy-paste typos\n\n* delete out.txt\n\n* fix rebase and bugs\n\n* remove dbg\n\n* rebase fix\n\n* switch from prev_lo/hi_clock to prev_clock_lo/hi",
          "timestamp": "2025-09-03T10:28:31+02:00",
          "tree_id": "3ac20f6cb087c97d53efc524f114de8b1af132f8",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/2c20fb778bcbaab2b6e785956db8224745b7dd03"
        },
        "date": 1756889155923,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2377219109,
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
          "id": "cdf0ddbe3ac8c1ba3dcbe30f72838ba3cef78d4d",
          "message": "feat(prover): add tests for u32 arithmetics and fix bugs (#290)\n\n* bug: fix u32_store_imm on 2 limbs\n\n* bug: fix u32_store_mul_fp_imm by removing 16bit limbs\n\n* bug: remove 16bit limb addition in u32_store_eq_fp_fp/imm\n\n* feat: add range_check for mul carry in div, added tests\n\n* remove old u32 tests\n\n* fix carries in u32_store_div\n\n* rebase\n\n* typo",
          "timestamp": "2025-09-03T12:27:18+03:00",
          "tree_id": "6397fe2fd60b2909f7fd062449e95ef90b3249dd",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/cdf0ddbe3ac8c1ba3dcbe30f72838ba3cef78d4d"
        },
        "date": 1756892660622,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2377236656,
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
          "id": "6e445f6200983bcab753970e14254fac854574b7",
          "message": "feat(prover): add double_derefs and store_frame_pointer (#294)\n\n* feat: added files and boiler plate with AIR in comments (code not matching)\n\n* feat: added double_derefs (not fully tested)\n\n* feat: add store_frame_pointer and fix VM reading order\n\n* typo: fixed copy paste typo\n\n* merge double-deref-fp-fp\n\n* merge double_deref_fp_imm\n\n* merged opcodes\n\n* update tests\n\n* remove debugs\n\n* Fix component docstring\n\n---------\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>",
          "timestamp": "2025-09-03T19:38:15+03:00",
          "tree_id": "1bac71094ceb20c1f5513582e9585e35a1da305a",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/6e445f6200983bcab753970e14254fac854574b7"
        },
        "date": 1756918530525,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2377297678,
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
          "id": "fcebc4c498ff245ea61ebf5c70fa0d345287260a",
          "message": "refactor: remove AssertEqFpFp (#297)",
          "timestamp": "2025-09-04T13:01:14+02:00",
          "tree_id": "4845935832a5363b4cb1f1541d7ce48299ea6a95",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/fcebc4c498ff245ea61ebf5c70fa0d345287260a"
        },
        "date": 1756984691918,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2377285903,
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
          "id": "eb202263e672d82fd272a8733bcc964ef82ae831",
          "message": "feat(prover): add u32_store_x_fp_fp (#291)\n\n* feat: added u32_store_x_fp_fp (untested with real examples)\n\n* feat: add and test store_add and store_div\n\n* feat: add and test store_mul\n\n* feat: add and test store_sub\n\n* test: tested store_sub\n\n* Update crates/prover/src/components/opcodes/u32_store_sub_fp_fp.rs\n\n* Update crates/prover/src/components/opcodes/u32_store_sub_fp_fp.rs\n\n* Update crates/prover/src/components/opcodes/u32_store_sub_fp_fp.rs",
          "timestamp": "2025-09-04T16:59:42+03:00",
          "tree_id": "1d3b6f72692ae925093c2d834910c7ec2c9c975b",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/eb202263e672d82fd272a8733bcc964ef82ae831"
        },
        "date": 1756995472983,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2377668849,
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
          "id": "87b854ed16c711a248878632f73b1a670bc9aa35",
          "message": "feat(prover): add bitwise preprocessed column (#298)\n\n* feat: add bitwise macro\n\n* feat: add col_index to bitwise traces\n\n* stack bitwise operators in the preprocessed trace\n\n* change API to pick word sizes\n\n* clippy warnings\n\n* use preprocessed trace in bitwise\n\n* review",
          "timestamp": "2025-09-05T10:28:17+02:00",
          "tree_id": "f3fd417436606422173bf944daa9db3854b171a3",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/87b854ed16c711a248878632f73b1a670bc9aa35"
        },
        "date": 1757061937696,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2411237652,
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
          "id": "49ed9a7c20ed73b5be2be8dc99bc965d515457a5",
          "message": "feat(prover): add u32_store_bitwise_fp_fp (#302)\n\n* feat: add and test bitwise_fp_fp\n\n* remove assert eq fp fp\n\n* typo\n\n* removed uneccessary constraint\n\n* remove res computation\n\n* rename\n\n* remove unecessary variables",
          "timestamp": "2025-09-05T13:48:58+03:00",
          "tree_id": "43a7d5a663ad55e29442576f2727590636e386ae",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/49ed9a7c20ed73b5be2be8dc99bc965d515457a5"
        },
        "date": 1757070417329,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2409779470,
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
          "id": "495e91b1493ff459d20bfe29d28af67d4c5160be",
          "message": "feat: add u32_store_bitwise_fp_imm (#304)",
          "timestamp": "2025-09-05T15:08:52+03:00",
          "tree_id": "98b85b12d3fd088b6da42ba4082aee46e5b60475",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/495e91b1493ff459d20bfe29d28af67d4c5160be"
        },
        "date": 1757075212031,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2409846632,
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
          "id": "4a4221d4fc98b33974ea25626c3c8e6ea599ce63",
          "message": "dev: add benchmarks for sha256 (#308)\n\n* dev: add benchmarks for sha256\n\n* suggestions",
          "timestamp": "2025-09-05T18:58:58+02:00",
          "tree_id": "02559426b9027b54c580a54ba6a025694f4ae43b",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/4a4221d4fc98b33974ea25626c3c8e6ea599ce63"
        },
        "date": 1757092745981,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2409846632,
            "unit": "bytes"
          },
          {
            "name": "sha256_1kb_prove_peak_mem",
            "value": 2957774883,
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
          "id": "1577f99fc685bb15c8bf0e373dd75cecf82d9f97",
          "message": "feat: write values of const fixed-size arrays in compiled program (#307)\n\n* feat(compiler): support constant arrays with read-only data segment\n\nAdd support for constant arrays that are stored in a read-only data segment\nafter the bytecode. This enables efficient memory usage for immutable arrays\nby deduplicating identical constant arrays and placing them in a separate\nrodata section. The implementation tracks const arrays from MIR through\ncodegen, generates rodata blobs with labels, and updates the program format\nto include both instructions and data values.\n\nKey changes:\n- Add is_const flag to MIR MakeFixedArray instruction\n- Implement rodata blob tracking and deduplication in codegen\n- Update Program structure to use ProgramData enum for instructions and values\n- Add LoadConstAddr instruction for loading rodata addresses\n- Update VM to handle new program format with data section\n- Remove obsolete type cast test snapshots\n\n* skip zero-writes",
          "timestamp": "2025-09-08T11:23:15+02:00",
          "tree_id": "2d548fdc59bfa4177b965ab505ff8c939e71eab9",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/1577f99fc685bb15c8bf0e373dd75cecf82d9f97"
        },
        "date": 1757324564889,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2409846632,
            "unit": "bytes"
          },
          {
            "name": "sha256_1kb_prove_peak_mem",
            "value": 2171452867,
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
          "id": "d30841f46f2a9c667689fc21e2fa7b3313549ccd",
          "message": "Add tree widths logging to prover (#311)\n\n* Add tree widths info! in prover\n\n* Fix tracing initialization and improve log message\n\n- Use try_init() instead of init() to avoid panics when tracing is already initialized\n- Improve log message to be more descriptive about tree widths\n\nAddresses review feedback from Claude\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* Move subscription in test\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-09-08T17:50:00+03:00",
          "tree_id": "5072560e53d700ccc914b25e6f5c2733d90897ee",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/d30841f46f2a9c667689fc21e2fa7b3313549ccd"
        },
        "date": 1757344196671,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2409847112,
            "unit": "bytes"
          },
          {
            "name": "sha256_1kb_prove_peak_mem",
            "value": 2171452867,
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
          "id": "f0fe747f3f6d6954916968cb31dbb105e144aa84",
          "message": "refactor(vm): refactor memory model to support heap (#314)\n\n* feat: heap memory\n\n* MAX_ADDRESS constant + slice fix\n\n* minor fixes\n\n* suggestions\n\n* more suggestions\n\n* move projection check to get_qm31_no_trace\n\n* remove insert_slice and related tests\n\n* suggestion",
          "timestamp": "2025-09-11T11:33:19+02:00",
          "tree_id": "91db4e929d1cdb6952caa6d23d7395c641205516",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/f0fe747f3f6d6954916968cb31dbb105e144aa84"
        },
        "date": 1757584357806,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2409846632,
            "unit": "bytes"
          },
          {
            "name": "sha256_1kb_prove_peak_mem",
            "value": 2171452867,
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
          "id": "9b970cfc0ad8631f995e7f0e5a20bbcaee1ebb67",
          "message": "opti: add codegen-units and lto settings (#316)",
          "timestamp": "2025-09-12T09:29:17+03:00",
          "tree_id": "5058f81b318b64c2ca86189c2200ded68ee4a402",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/9b970cfc0ad8631f995e7f0e5a20bbcaee1ebb67"
        },
        "date": 1757659525780,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2409847144,
            "unit": "bytes"
          },
          {
            "name": "sha256_1kb_prove_peak_mem",
            "value": 2171452867,
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
          "id": "f3d0f0acbfe7aed40b62850785b016891eabd179",
          "message": "feat: U32Div -> U32DivRem (#292)\n\n* feat: U32Div -> U32DivRem\n\n* feat: update prover for divrem opcodes\n\n* Update crates/prover/src/components/opcodes/u32_store_div_fp_fp.rs\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>\n\n* Update crates/prover/src/components/opcodes/u32_store_div_fp_fp.rs\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>\n\n* address review\n\n---------\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>",
          "timestamp": "2025-09-15T16:42:35+02:00",
          "tree_id": "b8822c14ad7a05955b7eb3eaf28a319ea5ac5859",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/f3d0f0acbfe7aed40b62850785b016891eabd179"
        },
        "date": 1757948263343,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "fibonacci_prove_peak_mem",
            "value": 2414075440,
            "unit": "bytes"
          },
          {
            "name": "sha256_1kb_prove_peak_mem",
            "value": 2180655819,
            "unit": "bytes"
          }
        ]
      }
    ]
  }
}