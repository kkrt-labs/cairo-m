window.BENCHMARK_DATA = {
  "lastUpdate": 1752067302723,
  "repoUrl": "https://github.com/kkrt-labs/cairo-m",
  "entries": {
    "Cairo-M Prover Speed Benchmarks": [
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
        "date": 1751018409450,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 3949291432,
            "range": "± 25565258",
            "unit": "ns/iter"
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
        "date": 1751018735868,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 5535972874,
            "range": "± 38684996",
            "unit": "ns/iter"
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
        "date": 1751039793000,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 5574499130,
            "range": "± 35279663",
            "unit": "ns/iter"
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
        "date": 1751043244775,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 5692926134,
            "range": "± 40811804",
            "unit": "ns/iter"
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
        "date": 1751051149153,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 5622495421,
            "range": "± 30895028",
            "unit": "ns/iter"
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
        "date": 1751899798157,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 5649715750,
            "range": "± 44323671",
            "unit": "ns/iter"
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
        "date": 1751902053179,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 5650078065,
            "range": "± 29247178",
            "unit": "ns/iter"
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
        "date": 1751902060723,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 5659134728,
            "range": "± 26870140",
            "unit": "ns/iter"
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
        "date": 1751977992479,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 5659452459,
            "range": "± 31675470",
            "unit": "ns/iter"
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
        "date": 1751991302609,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 5640828290,
            "range": "± 42162187",
            "unit": "ns/iter"
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
        "date": 1752060646857,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 5678419054,
            "range": "± 29754072",
            "unit": "ns/iter"
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
        "date": 1752067301696,
        "tool": "cargo",
        "benches": [
          {
            "name": "prover_fibonacci/prove",
            "value": 5622880833,
            "range": "± 35887480",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}