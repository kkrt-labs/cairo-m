window.BENCHMARK_DATA = {
  "lastUpdate": 1757947736956,
  "repoUrl": "https://github.com/kkrt-labs/cairo-m",
  "entries": {
    "Cairo-M VM Benchmarks": [
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
          "id": "96e089cd9fc3da8b1b6321dce52274200be8e2fa",
          "message": "chore(ci): add runner benches to CI (#69)\n\n* chore(ci): add runner benches to CI\n\n* init submodules in benchmark ci\n\n* split in 2 jobs",
          "timestamp": "2025-06-18T17:32:17+02:00",
          "tree_id": "81eb3a5eb764c696f626ba32d4a03cc03f233fc2",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/96e089cd9fc3da8b1b6321dce52274200be8e2fa"
        },
        "date": 1750261230984,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 955638069,
            "range": "± 27281913",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 135190120,
            "range": "± 652584",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 466565034,
            "range": "± 43117844",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 15210652,
            "range": "± 155859",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 89347932,
            "range": "± 615743",
            "unit": "ns/iter"
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
          "id": "3501da7f978c76500f93400269fcf195bf1b3911",
          "message": "feat(compiler): optimize MIR passes (#75)\n\n* feat(compiler): optimize MIR passes\n\n* fix test",
          "timestamp": "2025-06-18T17:35:35+02:00",
          "tree_id": "7dc617fa830200edacb85be11317565c0bb78d0a",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/3501da7f978c76500f93400269fcf195bf1b3911"
        },
        "date": 1750261409066,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 947496343,
            "range": "± 23185615",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 134145159,
            "range": "± 715089",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 459529838,
            "range": "± 34877205",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 16413295,
            "range": "± 123155",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 86754523,
            "range": "± 491187",
            "unit": "ns/iter"
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
          "id": "1bedd56a9db33f2395f169cbdc7acec96affda78",
          "message": "feat(compiler): Implement boolean operators (#76)\n\n* boolean operators\n\n* simpler or\n\n* cleaner label gen\n\n* fix trunk\n\n* changed random.cm test\n\n* integration tests\n\n* fix: removed useless temp allocations\n\n* fixed comments",
          "timestamp": "2025-06-19T11:31:35+02:00",
          "tree_id": "7a9605723e91344eab1b7b92a7930dd2f5de7926",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/1bedd56a9db33f2395f169cbdc7acec96affda78"
        },
        "date": 1750325936884,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 959033452,
            "range": "± 25411497",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 133841201,
            "range": "± 704894",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 466694702,
            "range": "± 25557446",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 14836019,
            "range": "± 183500",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 88578569,
            "range": "± 879178",
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
          "id": "ebc59a923eb40f53d55c0a434589b1162785cff1",
          "message": "feat(vm): lib (#80)",
          "timestamp": "2025-06-19T14:28:51+02:00",
          "tree_id": "0999157d9753021923761e10c9679420e3cc8170",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/ebc59a923eb40f53d55c0a434589b1162785cff1"
        },
        "date": 1750336584193,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 972131594,
            "range": "± 24081804",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 133766071,
            "range": "± 703627",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 468362532,
            "range": "± 37997825",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 15012805,
            "range": "± 88103",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 92120188,
            "range": "± 692203",
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
          "id": "eecaab0cf25810aa0fdf7af604dbad06538b9ba0",
          "message": "feat(vm): add program length to memory trace (#79)",
          "timestamp": "2025-06-19T14:53:28+02:00",
          "tree_id": "b156252d29856630ccc4abb6ef6ed16f3b0dc330",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/eecaab0cf25810aa0fdf7af604dbad06538b9ba0"
        },
        "date": 1750338040700,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 958025071,
            "range": "± 24714500",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 131676955,
            "range": "± 375713",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 466892335,
            "range": "± 58595121",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 14572231,
            "range": "± 88734",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 90838009,
            "range": "± 224500",
            "unit": "ns/iter"
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
          "id": "0f8593d4fc74993223ace4ec083e715c4111bf5b",
          "message": "feat(compiler): MIR lowering for loop/while loops (#82)\n\n* feat(compiler): MIR lowering for loop/while loops\n\n* add fib_loop in diff tests",
          "timestamp": "2025-06-20T10:38:30+02:00",
          "tree_id": "76c055d14d0ee9c6c41085eeb3b0714c321f954f",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/0f8593d4fc74993223ace4ec083e715c4111bf5b"
        },
        "date": 1750409146708,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 958103960,
            "range": "± 24536901",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 131878821,
            "range": "± 965973",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 466906291,
            "range": "± 66332237",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 14786602,
            "range": "± 101357",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 93768151,
            "range": "± 1398193",
            "unit": "ns/iter"
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
          "id": "779853a14082dc3756646e9da21a1a78a5e90774",
          "message": "refactor(vm-compiler): unify instruction & opcode types (#83)\n\n* refactor: unify instruction & opcode types\n\n* build instructions from opcode enum\n\n* Update crates/common/Cargo.toml\n\nCo-authored-by: Oba <obatirou@gmail.com>\n\n* Update crates/compiler/codegen/src/generator.rs\n\n* Update crates/common/src/instruction.rs\n\nCo-authored-by: Oba <obatirou@gmail.com>\n\n* Update crates/common/src/instruction.rs\n\nCo-authored-by: Oba <obatirou@gmail.com>\n\n* Update crates/common/src/instruction.rs\n\nCo-authored-by: Oba <obatirou@gmail.com>\n\n* Update crates/common/src/instruction.rs\n\nCo-authored-by: Oba <obatirou@gmail.com>\n\n* address review comms\n\n* remove unused import\n\n* rename to InstructionOperands\n\n---------\n\nCo-authored-by: Oba <obatirou@gmail.com>",
          "timestamp": "2025-06-20T16:34:17+02:00",
          "tree_id": "92a0ae20c6df1fb22cb7aad04c2cba4a46bab613",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/779853a14082dc3756646e9da21a1a78a5e90774"
        },
        "date": 1750430484260,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 996367317,
            "range": "± 10788621",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 165676872,
            "range": "± 236363",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 484618994,
            "range": "± 27602254",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 14422132,
            "range": "± 23720",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 85755903,
            "range": "± 165288",
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
          "id": "3cd70d41dc5c5107e472d6924c23af0aad10b113",
          "message": "Use TryFrom trait instead of custom func (#86)",
          "timestamp": "2025-06-23T10:05:44+02:00",
          "tree_id": "8a0adadcc95350c4a003b62c0b660ce5fe51f1a6",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/3cd70d41dc5c5107e472d6924c23af0aad10b113"
        },
        "date": 1750666395165,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 1002431574,
            "range": "± 19235894",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 170448086,
            "range": "± 1769296",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 476332254,
            "range": "± 30438481",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 14798875,
            "range": "± 74066",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 86929838,
            "range": "± 833343",
            "unit": "ns/iter"
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
          "id": "d35d7f54dd3f37f87e3e7ba5c6944ddf065f4864",
          "message": "refactor(vm): test organization (#87)\n\n* refactor: test content with helpers\n\n* refactor: move test files",
          "timestamp": "2025-06-23T11:23:13+02:00",
          "tree_id": "6930e11e9c7d1207a1aea13281a8d91ee1235a68",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/d35d7f54dd3f37f87e3e7ba5c6944ddf065f4864"
        },
        "date": 1750671030526,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 1003018644,
            "range": "± 16268270",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 169665206,
            "range": "± 325285",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 479993380,
            "range": "± 25338474",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 14694376,
            "range": "± 315074",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 85821471,
            "range": "± 1157376",
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
          "id": "36667db98d6b45a89f432ec8173e4b7b31468ff1",
          "message": "bench(vm): use compiled file (#88)\n\n* bench(vm): use compiled file\n\n* Add doc FP_OFFSET",
          "timestamp": "2025-06-23T12:38:50+02:00",
          "tree_id": "f094dbf8a719edd070eb6de84cdd6b9a45d80ef4",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/36667db98d6b45a89f432ec8173e4b7b31468ff1"
        },
        "date": 1750675791636,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 2148411357,
            "range": "± 14263989",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 323996816,
            "range": "± 3129657",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 1471926857,
            "range": "± 41705481",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 25924939,
            "range": "± 399473",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 162766218,
            "range": "± 2066917",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mathieu",
            "username": "enitrat",
            "email": "60658558+enitrat@users.noreply.github.com"
          },
          "committer": {
            "name": "enitrat",
            "username": "enitrat",
            "email": "msaug@protonmail.com"
          },
          "id": "1f86d21891deb4c497d57fc4b9cffbc2b5eaf879",
          "message": "feat(compiler): optimize binary operations w/ intermediate vars (#90)\n\n* feat(compiler): optimize binary operations w/ intermediate vars",
          "timestamp": "2025-06-23T14:44:57Z",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/1f86d21891deb4c497d57fc4b9cffbc2b5eaf879"
        },
        "date": 1750691490993,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 1650107273,
            "range": "± 13457492",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 262369468,
            "range": "± 1082085",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 1027438786,
            "range": "± 29693905",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 21056849,
            "range": "± 72402",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 131340937,
            "range": "± 433787",
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
          "id": "1ff8945ae6f8a93ec9d0cb5beb4f0a835d1228cd",
          "message": "feat(vm): use anyhow for CLI (#94)",
          "timestamp": "2025-06-24T11:37:22+02:00",
          "tree_id": "a8eac42d8c172acfe816d113751bc125b49fd50b",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/1ff8945ae6f8a93ec9d0cb5beb4f0a835d1228cd"
        },
        "date": 1750758458970,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 1650520734,
            "range": "± 22315004",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 266038879,
            "range": "± 819004",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 1023151867,
            "range": "± 47507479",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 21064500,
            "range": "± 63533",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 132042104,
            "range": "± 1024837",
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
          "id": "b71874b3aa86516b6a8603d9205eddbabdc286f1",
          "message": "feat(prover): instantiate ProverInput from RunnerOutput (#98)\n\n* feat(prover): import_from_runner_output\n\n* refactor: move State from runner to common\n\n* refactor: simplify input type\n\n* rename import_from_vm_output and simplify test\n\n* review",
          "timestamp": "2025-06-24T17:03:17+02:00",
          "tree_id": "1077b438f56d34ea86054e7fea2abbff41629b52",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/b71874b3aa86516b6a8603d9205eddbabdc286f1"
        },
        "date": 1750777984753,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 1650098577,
            "range": "± 8716629",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 263580554,
            "range": "± 1404763",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 1024943017,
            "range": "± 33815174",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 20940133,
            "range": "± 90651",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 130629782,
            "range": "± 974860",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Mathieu",
            "username": "enitrat",
            "email": "60658558+enitrat@users.noreply.github.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e81f2ed6d0fc1b235ea0dbffb2152284f39bb658",
          "message": "feat(compiler): optimize calls with args already in order (#102)",
          "timestamp": "2025-06-24T16:20:13Z",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/e81f2ed6d0fc1b235ea0dbffb2152284f39bb658"
        },
        "date": 1750782611847,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 1652112771,
            "range": "± 17003240",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 258196910,
            "range": "± 2195278",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 1033314140,
            "range": "± 34212353",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 23199652,
            "range": "± 251018",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 128645093,
            "range": "± 1048221",
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
          "id": "13017d7365cad48c9290870ff0c6be6e9306b45f",
          "message": "refactor(vm-prover): unify trace entry types (#101)\n\n* refactor(vm-prover): unify trace entry types\n\n* refactor: use copy instead of clone",
          "timestamp": "2025-06-25T15:00:30+02:00",
          "tree_id": "49380c9517e229c6b33aa2ae9d7c44eb1e5cd020",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/13017d7365cad48c9290870ff0c6be6e9306b45f"
        },
        "date": 1750857019097,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 1650368132,
            "range": "± 10738972",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 258124166,
            "range": "± 1630795",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 1029706692,
            "range": "± 29160337",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 20659624,
            "range": "± 264545",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 130688294,
            "range": "± 772716",
            "unit": "ns/iter"
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
          "id": "85035398e35d276d8018ab8aea558a819622ff21",
          "message": "feat(vm): support calling entrypoints with args (#106)\n\n* feat(vm): support calling entrypoints with args\n\n* address comments",
          "timestamp": "2025-06-25T15:54:42+02:00",
          "tree_id": "dd06d407208b2a3d8307374841b8c6c32229d2b9",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/85035398e35d276d8018ab8aea558a819622ff21"
        },
        "date": 1750860312010,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 1744397326,
            "range": "± 53459738",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 273654606,
            "range": "± 9445108",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 1114934862,
            "range": "± 50968201",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 25510544,
            "range": "± 2863571",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 155922597,
            "range": "± 14577312",
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
          "id": "4248f26b2a3aa29e0d9bbf99a71a018feb874d12",
          "message": "feat(vm-prover): provide final state to the prover (#108)",
          "timestamp": "2025-06-25T16:42:22+02:00",
          "tree_id": "8594a99eda1567763c063135cf3f5421ad867750",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/4248f26b2a3aa29e0d9bbf99a71a018feb874d12"
        },
        "date": 1750863137259,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/e2e",
            "value": 1650513997,
            "range": "± 30812707",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/execution_only",
            "value": 262310446,
            "range": "± 1427751",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/io_only",
            "value": 1025910246,
            "range": "± 31882436",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_vm_trace",
            "value": 21080714,
            "range": "± 186722",
            "unit": "ns/iter"
          },
          {
            "name": "fibonacci_1m/serialize_memory_trace",
            "value": 132392770,
            "range": "± 1127264",
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
          "id": "6dc6e72c5b4d363c00f6b0c18e5decc2d6f8c444",
          "message": "bench: remove io (#110)",
          "timestamp": "2025-06-25T17:35:46+02:00",
          "tree_id": "02c91e7b322dc16cebc22408340e2d688c7a66be",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/6dc6e72c5b4d363c00f6b0c18e5decc2d6f8c444"
        },
        "date": 1750865919722,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 261625424,
            "range": "± 595344",
            "unit": "ns/iter"
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
          "id": "9e629dbc267067e240433a4b3c0690681339b010",
          "message": "fix(vm): useless conversion in return value collection (#114)",
          "timestamp": "2025-06-25T18:48:53+02:00",
          "tree_id": "adde432da78018a5c85a0ef6ec7cb46a87615f1c",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/9e629dbc267067e240433a4b3c0690681339b010"
        },
        "date": 1750870292030,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 261650154,
            "range": "± 1086210",
            "unit": "ns/iter"
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
          "id": "bc30304c40023f8f54ad06b8d1ae6a43efaad0aa",
          "message": "fix(vm): dont log initial memory in trace entries (#115)\n\n* fix(vm): dont log initial memory in trace entries\n\n* remove ignore",
          "timestamp": "2025-06-26T10:59:57+02:00",
          "tree_id": "5ac8fba9fb5427751ecaad63a55e9a3909a5756c",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/bc30304c40023f8f54ad06b8d1ae6a43efaad0aa"
        },
        "date": 1750928598706,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 258038115,
            "range": "± 1440579",
            "unit": "ns/iter"
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
          "id": "7cc842c1b192964ce97d5a16fa6998cfee105992",
          "message": "ci: add benchmarks on prover crate (#121)\n\n* dev: add benchmarks on prover\n\n* fix ghpage issue\n\n* fixes\n\n* updat n_iter to 100k\n\n* count vm adapter in benchmark",
          "timestamp": "2025-06-27T11:51:29+02:00",
          "tree_id": "9c35e2405045debf9490749a5bf126b4e3c39888",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/7cc842c1b192964ce97d5a16fa6998cfee105992"
        },
        "date": 1751018059095,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 259217700,
            "range": "± 766646",
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
        "date": 1751050557243,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 263026690,
            "range": "± 1638995",
            "unit": "ns/iter"
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
          "id": "02ffdf4397c3eab0c3e6b3577bc301aff223f8d8",
          "message": "feat(compiler): support tuple destructuration (#123)\n\n* parser\n\n* semantic\n\n* mir\n\n* fix bad code\n\n* split lowering function in smaller sub-fn",
          "timestamp": "2025-06-29T18:15:55+02:00",
          "tree_id": "734f76794e3b4c96eed591286f46f8f9b1ae5aff",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/02ffdf4397c3eab0c3e6b3577bc301aff223f8d8"
        },
        "date": 1751213915202,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 261242639,
            "range": "± 753168",
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
        "date": 1752060074997,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 259502278,
            "range": "± 717878",
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
        "date": 1752066715744,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 260491534,
            "range": "± 1873990",
            "unit": "ns/iter"
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
          "id": "a6a4e983d3d5b27d774d5075acdcf57f5f8841b6",
          "message": "fix(compiler): fix return type validation (#154)\n\n* fix(compiler): fix return type validation\n\n* refactor all void functions to return unit type by default\n\n* fix return data",
          "timestamp": "2025-07-14T13:58:39+02:00",
          "tree_id": "b08e89a1bbc7dbbcecabc12e5a43a965414edd58",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/a6a4e983d3d5b27d774d5075acdcf57f5f8841b6"
        },
        "date": 1752494486490,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 259717969,
            "range": "± 1205109",
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
          "id": "adde39448877736fdcc2a958b118efff02f6dd66",
          "message": "chore: fmt (#160)\n\nCo-authored-by: enitrat <msaug@protonmail.com>",
          "timestamp": "2025-07-16T14:01:05+02:00",
          "tree_id": "d79d668ed4acb0e2042c2b21b9cefc4d17fbf62e",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/adde39448877736fdcc2a958b118efff02f6dd66"
        },
        "date": 1752667490509,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 262381633,
            "range": "± 1463856",
            "unit": "ns/iter"
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
        "date": 1752670026061,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 291701761,
            "range": "± 989200",
            "unit": "ns/iter"
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
          "id": "5c0721df1a7ad50ed6acca2467d9c176c8510a2a",
          "message": "fix(compiler): bool type validation (#165)\n\n* fix: bool type validation\n\n* fix review issues\n\n* fix snapshot",
          "timestamp": "2025-07-16T16:33:53+02:00",
          "tree_id": "496502af81089f7c6baaa54aab1f3b1838f36d34",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/5c0721df1a7ad50ed6acca2467d9c176c8510a2a"
        },
        "date": 1752676630888,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 290742909,
            "range": "± 1383054",
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
          "id": "52df73080408295fdf5628f1fff79e63cb5db784",
          "message": "feat(prover): Remove StoreDerefFp opcode and replace with StoreAddFpImm (#174)\n\n* Fix typos\n\n* Vibecode it",
          "timestamp": "2025-07-18T17:06:34+02:00",
          "tree_id": "bc4a33b59ac85c1336128bd44c47a66cd1cbdffe",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/52df73080408295fdf5628f1fff79e63cb5db784"
        },
        "date": 1752851383237,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 241213293,
            "range": "± 1985504",
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
          "id": "0f49972b20474c255c1877c66d601543636abc9a",
          "message": "feat(runner): split execution for continuation (#164)\n\n* feat(runner): split execution for continuation\n\n* review fixes\n\n* use vec with capacity for memory serialization\n\n* added doc for ExecutionStatus\n\n* review modifs\n\n* updated the vm benchmark\n\n* remove cloning for single segment executions\n\n* replace n_steps by max_steps",
          "timestamp": "2025-07-22T11:52:46+02:00",
          "tree_id": "4a63737008d11d2861d0d6cab0c3ea3bbff40ec8",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/0f49972b20474c255c1877c66d601543636abc9a"
        },
        "date": 1753178131984,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 238056180,
            "range": "± 1718119",
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
          "id": "eee15dbe986c752264990425790bd3af8ba3e96d",
          "message": "feat(prover): add public addresses (#185)\n\n* add public addresses\n\n* fixed typo for public addresses\n\n* added comment",
          "timestamp": "2025-07-23T17:23:39+02:00",
          "tree_id": "59434d35afa0605359ef9f30681aa9f159a72ab0",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/eee15dbe986c752264990425790bd3af8ba3e96d"
        },
        "date": 1753284411091,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 237156243,
            "range": "± 2050344",
            "unit": "ns/iter"
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
          "id": "1ea82ef524b13eeb5d1e99b2e565e017be772b30",
          "message": "feat(compiler): C-Style for loops (#201)\n\n* feat: C-Style for loops\n\n* small doc fix",
          "timestamp": "2025-07-28T13:39:45+02:00",
          "tree_id": "85d56288d14e7ce5504e9f3ae0d6c8a21e8b9c8a",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/1ea82ef524b13eeb5d1e99b2e565e017be772b30"
        },
        "date": 1753702973432,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 236820644,
            "range": "± 991272",
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
          "id": "1d34c87b6e87b3472299b1349963fa3ebca94d80",
          "message": "feat(prover): factor components with equal lookup operations (#200)\n\n* Update macro to support list of opcodes\n\n* Merge jmp imm opcodes\n\n* merge store_fp_fp\n\n* Fix tests\n\n* merge store_fp_imm\n\n* Use saturatin_sub",
          "timestamp": "2025-07-31T11:06:40+02:00",
          "tree_id": "8e1efe189c59ba8094b0f865e90e2fa5315de6d8",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/1d34c87b6e87b3472299b1349963fa3ebca94d80"
        },
        "date": 1753953041691,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 235721863,
            "range": "± 4000836",
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
          "id": "84d59b1395dcccb3b6ed42f87408a13336746c78",
          "message": "epic: support variable instruction size (#195)\n\n* feat(common): variable instructions size (#178)\n\n* feat(common): variable instructions size\n\n* remove StoreDerefFp\n\n* adds comments for define_instruction\n\n* feat(compiler): Adapt Compiler Codegen for Variable-Sized Instructions (#179)\n\n* feat(compiler): Adapt Compiler Codegen for Variable-Sized Instructions\n\n* refactor: remove hardcoded opcode ids\n\n* ci: run tests on feature branches\n\n* refactor\n\n* add snapshot variable instructions\n\n* feat(runner): Adapt Runner for Variable-Sized Instructions (#182)\n\n* feat(prover): Adapt Prover Adapter and Bundles for U32 Support (#196)\n\n* feat(prover): Adapt Prover Adapter and Bundles for U32 Support\n\n* refactor\n\n* update store_imm documentation\n\n* PR comments\n\n* panic on failure to get operand type\n\n---------\n\nCo-authored-by: enitrat <msaug@protonmail.com>\n\n* remove unused variable\n\n* fix rebase\n\n---------\n\nCo-authored-by: enitrat <msaug@protonmail.com>\nCo-authored-by: Antoine FONDEUR <antoine.fondeur@gmail.com>",
          "timestamp": "2025-08-01T14:33:16+02:00",
          "tree_id": "21bcc1fdfffdfff537d7be210e38e9cd86456a2c",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/84d59b1395dcccb3b6ed42f87408a13336746c78"
        },
        "date": 1754051807848,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 330514240,
            "range": "± 1604156",
            "unit": "ns/iter"
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
          "id": "0188eb3b57bfc85189ebb9c3c1f3e86e4e295c62",
          "message": "feat(common): define u32 opcodes (#213)\n\n* feat: define u32 opcodes in cairo-m-common\n\n- Add u32 arithmetic opcodes for FP-FP operations (opcodes 15-18)\n- Add u32 arithmetic opcodes for FP-IMM operations (opcodes 19-22)\n- Update tests to reflect new opcode values\n- Maintain consistent ordering with FP-FP before FP-IMM variants\n\n🤖 Generated with Claude Code\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix: update cairo-m-runner for new u32 opcodes\n\n- Update LAST_VALID_OPCODE_ID from 15 to 22\n- Fix test_u32_store_add_fp_imm_from_smallvec to use correct opcode 19\n- Fix test_get_instruction_multi_qm31 to use correct opcode 19\n- Add TODO stubs for new u32 instruction implementations\n- Map all u32 opcodes to their handler functions\n\n🤖 Generated with Claude Code\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* add missing tests\n\n* use MAX_OPCODE const\n\n* refactor: remove redundant tests from cairo-m-runner\n\nRemove tests that duplicate functionality already covered in\ncairo-m-common tests:\n- test_store_add_fp_imm_from_smallvec\n- test_ret_from_smallvec\n- test_u32_store_add_fp_imm_from_smallvec\n\nThese tests were testing SmallVec to Instruction conversion which\nis comprehensively tested in crates/common/tests/instruction_tests.rs\n\nKeep only runner-specific tests for opcode_to_instruction_fn mapping.\n\n🤖 Generated with Claude Code\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-08-01T15:44:22+02:00",
          "tree_id": "e5f3ea85cd25ac5c2a83fd91aa2bdc3a2d46dabe",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/0188eb3b57bfc85189ebb9c3c1f3e86e4e295c62"
        },
        "date": 1754056066218,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 337334903,
            "range": "± 4719247",
            "unit": "ns/iter"
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
        "date": 1754067881161,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 336277049,
            "range": "± 2154136",
            "unit": "ns/iter"
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
          "id": "d7c2f4d4ec8702cc74197da0b8fde8bf9ccec8ec",
          "message": "feat(runner): implement all U32 arithmetic operations in VM (#219)\n\n* feat(runner): implement all U32 operations in VM\n\nImplements all U32 arithmetic operations for Cairo-M VM runner with\nRISC-V-compliant behavior:\n\n- FP-FP operations: U32StoreAddFpFp, U32StoreSubFpFp, U32StoreMulFpFp, U32StoreDivFpFp\n- FP-IMM operations: U32StoreSubFpImm, U32StoreMulFpImm, U32StoreDivFpImm\n- All operations use wrapping arithmetic (no overflow traps)\n- Division by zero returns 0xFFFFFFFF following RISC-V specification\n- U32 values stored as two 16-bit M31 limbs in consecutive memory cells\n- Comprehensive tests for all operations including edge cases\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* refactor(runner): consolidate U32 operations and add property-based tests\n\n- Move U32_LIMB_BITS and U32_LIMB_MASK constants to memory module\n- Consolidate repetitive U32 binary operation implementations using generic helper functions\n- Add proptest dependency for property-based testing of U32 operations\n- Reduce code duplication by ~400 lines while maintaining functionality\n- Add .repo_ignore to gitignore for local development files\n\n* fmt\n\n* unused import\n\n* address comments\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-08-04T13:59:14+02:00",
          "tree_id": "2daaa9bb3ccc88c8cc4aa9352bff2604c127b74a",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/d7c2f4d4ec8702cc74197da0b8fde8bf9ccec8ec"
        },
        "date": 1754308958083,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 337531450,
            "range": "± 4526624",
            "unit": "ns/iter"
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
          "id": "ffaecc8f1a02c18621c0e8d6c78ce874f7dcaa2f",
          "message": "feat(runner-compiler): add u32_store_imm instruction (#220)\n\n* feat: add u32_store_imm instruction\n\n* optimize",
          "timestamp": "2025-08-04T16:24:50+02:00",
          "tree_id": "252f13d66cbafe75d02e9b1684920497f0d4b68d",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/ffaecc8f1a02c18621c0e8d6c78ce874f7dcaa2f"
        },
        "date": 1754317701676,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 334957999,
            "range": "± 3196617",
            "unit": "ns/iter"
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
          "id": "d3518dc94111b6061d6eae7377464b559fe3833b",
          "message": "feat(compiler-codegen): implement U32 operations and fix return value slot calculation (#217)\n\n* feat(codegen): implement direct argument placement optimization\n\nThis refactoring establishes a clean architectural boundary between MIR and\ncodegen layers while implementing an optimization that eliminates unnecessary\ncopy instructions for function arguments.\n\nKey changes:\n- Add CalleeSignature to Call/VoidCall instructions for self-contained MIR\n- Implement Direct Argument Placement optimization in CodeGenerator\n- Look ahead to place values directly at argument positions when possible\n- Support optimization for Store, Assign, UnaryOp, and BinaryOp instructions\n- Add per-argument copy skipping in pass_arguments when already in place\n\nThe optimization reduces instruction count by eliminating copies like:\n  Before: Store value at temp location, then copy to argument position\n  After: Store value directly at argument position\n\nThis maintains the robust pre-allocated layout architecture while achieving\nefficient code generation through smart orchestration in the CodeGenerator.\n\n* feat(codegen): implement U32 operations and fix return value slot calculation\n\n- Add U32 arithmetic operations (add, sub, mul, div) code generation\n- Fix return value offset calculation to use slots instead of values\n- Update layout to track num_return_slots for multi-slot types\n- Handle multi-slot value copying in return statements\n- Add comprehensive tests for U32 operations\n\nBreaking changes:\n- Return values for multi-slot types (like u32) now correctly placed at fp-K-2\n  where K is the total number of return slots (not values)\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix: address PR review comments\n\n- Replace unwrap() with proper error handling using ? operator in layout.rs\n- Remove fib_u32_loop test that uses unimplemented U32 comparison operations\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* minor changes\n\n* feat(compiler): add info on args/ret data size in ABI\n\n* feat(mir): add AssignU32 instruction for type-aware u32 literal handling\n\n- Add AssignU32 variant to InstructionKind for explicit u32 assignments\n- Update MIR generation to use AssignU32 for u32 literal returns based on function signatures\n- Implement codegen support using U32_STORE_IMM opcode for efficient u32 storage\n- Add proper type inference for literal values in return statements\n- Maintain return value optimization for u32 literals\n- Add test for u32 literal return to verify correct handling\n\nThis change makes the MIR more explicit about u32 operations, avoiding the need\nfor type queries during codegen and ensuring proper handling of multi-slot types.\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-08-05T11:00:52+02:00",
          "tree_id": "2d96001a4c7cd3744fc874c458697f3e33b3d6e8",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/d3518dc94111b6061d6eae7377464b559fe3833b"
        },
        "date": 1754384651010,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 337102649,
            "range": "± 991067",
            "unit": "ns/iter"
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
          "id": "8f1f777a371b08e5d86387a5b03040c0e3f4b27a",
          "message": "feat(compiler): add struct support with mem2reg optimization pass (#224)\n\n- Implement complete struct support in MIR and codegen layers\n- Add stackalloc/getelementptr/load/store instructions for memory operations\n- Create InstructionEmitter trait for type-specific instruction generation\n- Add mem2reg optimization pass for eliminating redundant memory operations\n  - Implements escape analysis to identify non-escaping allocations\n  - Store-to-load forwarding within basic blocks\n  - Full allocation promotion to registers for single-block allocations\n  - Multi-block promotion with proper mutation handling\n- Fix layout calculation for function call return values\n- Add comprehensive documentation for flattened pointer model\n- Restore debug comments in MIR lowering for better debugging experience\n\nThe mem2reg pass successfully optimizes simple struct operations, reducing\nredundant memory traffic. For example, creating a struct and immediately\naccessing its fields is optimized to direct value computation.",
          "timestamp": "2025-08-06T15:43:22+02:00",
          "tree_id": "e9d0557882d6e01ddbc3bfbf0e8d0ee6a0874a7e",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/8f1f777a371b08e5d86387a5b03040c0e3f4b27a"
        },
        "date": 1754487993345,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 338171921,
            "range": "± 1579843",
            "unit": "ns/iter"
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
          "id": "0a950aee8b209f3592a1e558509751916b720041",
          "message": "feat(compiler-runner): implement U32 comparison opcodes and fix return value slot calculation (#228)\n\n* feat(compiler): implement U32 comparison opcodes and fix return value slot calculation\n\n- Add 6 new U32 comparison opcodes (Eq, Neq, Gt, Ge, Lt, Le) that return felt values\n- Implement VM operations for U32 comparisons in store.rs\n- Fix U32 literal returns to use U32_STORE_IMM instead of STORE_IMM\n- Fix caller's calculation of return value slots to account for U32 taking 2 slots\n- Update call and call_multiple methods to properly calculate frame offsets for multi-slot types\n\nThis enables full U32 comparison support and fixes issues where U32 return values\nwere incorrectly read from wrong frame pointer offsets after function calls.\n\n* feat: add U32ComparisonFpImm variants\n\n* review nit",
          "timestamp": "2025-08-07T18:14:02+02:00",
          "tree_id": "1504ca26048c15b096b1976ec4a020e625078a91",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/0a950aee8b209f3592a1e558509751916b720041"
        },
        "date": 1754583485136,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 386595472,
            "range": "± 1548665",
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
          "id": "1a50c20aaaa42f0648c590185b1b99d67b00b455",
          "message": "feat(prover): updated public memory (#189)\n\n* update public memory and refacto merkle tree\n\n* fix rebase\n\n* fix: fix rebase\n\n---------\n\nCo-authored-by: malatrax <71888134+zmalatrax@users.noreply.github.com>",
          "timestamp": "2025-08-20T14:05:42+02:00",
          "tree_id": "7a1424080579b64fa9e2e3a42e9b110f610f0b05",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/1a50c20aaaa42f0648c590185b1b99d67b00b455"
        },
        "date": 1755691806466,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 382860682,
            "range": "± 2740476",
            "unit": "ns/iter"
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
          "id": "84745806085e33f985f451da90a9a8f1616f3b2e",
          "message": "refactor(mir): comprehensive MIR optimization pipeline overhaul (#233)\n\n* refactor(compiler): reimplement mem2reg with SSA construction and dominance analysis\n\nReplace old mem2reg pass with proper SSA-based implementation using:\n- Dominance tree and frontier computation\n- Phi node insertion at join points\n- SSA variable renaming via dominator tree traversal\n- SSA destruction pass to eliminate phi nodes\n\nThis provides a more robust foundation for optimizations and correctly handles:\n- Complex control flow with multiple predecessors\n- Struct and tuple field promotions\n- Proper phi node placement and coalescing\n\nAdd comprehensive test suite for dominance analysis algorithms.\n\n* refactor(mir): improve SSA form with enhanced dominance algorithms and CFG utilities\n\n- Replace dominance algorithms with standard implementations:\n  - Implement Cooper-Harvey-Kennedy algorithm for efficient dominator tree computation\n  - Use standard dominance frontiers algorithm for correct phi placement\n  - Fix incorrect phi placement issues in SSA construction\n\n- Centralize CFG utilities in new cfg module:\n  - Extract critical edge detection and splitting logic\n  - Add helper functions for predecessor/successor queries\n  - Remove code duplication across passes\n\n- Remove in_place_target from BinaryOp to maintain pure SSA:\n  - Binary and unary operations are now side-effect free\n  - Remove deprecated InPlaceOptimizationPass entirely\n  - Backend optimizations continue independently at codegen level\n\n- Strengthen validation pass:\n  - Add comprehensive type checking for load/store operations\n  - Validate CFG structure and detect critical edges\n  - Ensure single-definition property for all values\n  - Add validation tests for all new checks\n\nAll MIR and codegen tests pass. The compiler now maintains proper SSA invariants\nwith correct dominance computation and cleaner separation between MIR and backend\noptimization concerns.\n\n* feat(mir): add SROA pass and first-class aggregate operations\n\nImplement comprehensive support for aggregate types in MIR with:\n\n- New DataLayout module centralizing all memory layout calculations (sizes, offsets)\n- First-class aggregate operations: BuildStruct, BuildTuple, ExtractValue, InsertValue\n- Type-safe GetElementPtrTyped using field paths instead of raw offsets\n- SROA (Scalar Replacement of Aggregates) pass that splits aggregate allocations\n  into per-field scalar allocations, enabling mem2reg to promote each field\n\nThe SROA pass transforms aggregate allocations accessed through constant paths\ninto individual scalar allocations, and eliminates Build*/Extract* patterns on\nSSA aggregates. This fixes struct/tuple handling in the compiler by ensuring\naggregates can be properly promoted to SSA form.\n\nAll existing tests pass and comprehensive SROA tests verify the transformations.\n\n* fix(mir): replace hardcoded entry block assumptions with entry_block field\n\n- Fixed critical correctness issues in SROA and mem2reg passes that assumed entry block is always at index 0\n- Removed duplicated CFG helper functions in dominance.rs and mem2reg_ssa.rs, now using canonical implementations from cfg module\n- Fixed iteration patterns in dominance frontier computation to use iter_enumerated() instead of raw indices\n\nThese changes make the compiler more robust to potential future block reordering and eliminate ~30 lines of duplicate code.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(mir): correct empty tuple representation and remove duplicate helpers\n\n- Fixed empty tuple to return proper Value::unit() instead of Value::integer(0)\n- Removed duplicate resolve_function() that created circular dependencies with resolve_callee_expression()\n- Removed unused get_expression_type() in favor of get_expr_type() which has caching\n- Cleaned up ~75 lines of duplicate code\n\nThese changes fix a type confusion bug and improve code maintainability.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(mir): disable unsound dead store elimination and add documentation\n\n- CRITICAL: Disabled dead store elimination pass due to unsoundness with GEP aliasing\n  The pass incorrectly assumes stores through pointers with zero uses can be eliminated,\n  but the same memory location may be accessed through different GEP-derived pointers\n- Added comprehensive pass pipeline documentation in PASSES.md\n- Documented the purpose of the locals field in MirFunction\n\nThis prevents potential miscompilation where stores could be incorrectly eliminated\nwhen accessed through aliased pointers.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(codegen): handle multi-slot stores correctly in CasmBuilder\n\nWhen storing multi-slot values (like structs with u32 fields), the store\nand store_at methods now properly copy all slots instead of just the first\none. This partially fixes aggregate store issues.\n\nAlso fixed map_value to preserve multi-slot information when mapping values\nto new offsets.\n\nNote: Full struct assignment still not working due to MIR generation issue.\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(codegen): comprehensive U32 handling and codegen correctness fixes\n\nThis commit addresses multiple critical bugs and improvements in the codegen module:\n\nCritical Correctness Fixes:\n- Fix U32 immediate splitting to properly mask both 16-bit halves (was sign-extending)\n- Fix store_u32_at to correctly touch 2 slots instead of 1 for liveness tracking\n- Fix BranchCmp for U32 to use comparison ops that return felt instead of subtraction\n- Fix return-slot preplacement to use total slots, not just value count\n- Fix duplicate-offset rewriter to use separate temp vars for felt (1-slot) and U32 (2-slot)\n\nSafety Improvements:\n- Implement U32 constant folding for all arithmetic and comparison operations\n- Add division-by-zero checks for both felt and U32 constant folding\n- Fix asymmetry in assign_u32_with_target to match assign_with_target behavior\n- Propagate errors from duplicate-offset resolution instead of silently ignoring\n\nCode Quality:\n- Add split_u32_value() and split_u32_i32() helper functions to reduce duplication\n- Reserve temp variables on-demand in duplicate-offset resolver\n- Improve comments to clarify boolean result values (1 for true, 0 for false)\n- Fix U32_STORE_ADD_FP_IMM instruction encoding with correct operands\n\nAll tests pass with updated snapshots reflecting the corrected codegen output.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(mir): re-enable dead store elimination in pre-optimization pass\n\nRe-enabled the previously disabled eliminate_dead_stores function with\nconservative analysis that only removes stores where the address operand\nitself is unused. This approach avoids GEP aliasing issues while still\nproviding optimization benefits for simple cases.\n\nThe optimization pipeline now runs all three passes in the intended order:\n1. Dead instruction elimination\n2. Dead store elimination (now active)\n3. Dead allocation elimination\n\nThis restores full optimization capability while maintaining safety\nthrough conservative analysis. Future work can enhance this with more\nsophisticated alias analysis for GEP-derived pointers.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(mir): implement parallel copy semantics in SSA destruction\n\nFixes a critical correctness issue where SSA destruction could produce\nwrong results when phi nodes have overlapping sources and destinations.\nThe previous implementation used sequential assignments which could\noverwrite values before they were read.\n\nImplemented a proper parallel copy algorithm that:\n- Detects copy cycles using DFS-based cycle detection\n- Breaks cycles by introducing temporary variables\n- Uses topological sort to ensure correct assignment ordering\n- Groups assignments by insertion block for batch processing\n\nThis ensures phi elimination maintains the parallel copy semantics\nrequired by SSA form and prevents silent correctness bugs in programs\nwith complex control flow patterns.\n\nAdded comprehensive tests for copy cycles and dependency chains to\nverify the implementation handles all edge cases correctly.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(mir): remove silent type fallback in struct literal lowering\n\nFixes a critical type safety issue where the compiler would silently\nfall back to 'felt' type when a struct field type was not found,\npotentially hiding bugs and generating incorrect code.\n\nReplaced the silent fallback with proper error handling that reports\nan internal compiler error with a clear message indicating the field\nname and struct type. This ensures:\n- Type safety violations are caught immediately\n- Field name mismatches are properly reported\n- Type propagation issues are detected early\n- Consistency with existing error handling patterns\n\nThe fix aligns with the existing error handling for field offset\ncalculation, providing consistent and robust type checking throughout\nthe struct literal lowering process.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* feat(mir): add basic block naming support for better debugging\n\nFixes misleading API where add_basic_block_with_name accepted but\nignored the name parameter. Now basic blocks properly store and\ndisplay their names in MIR output.\n\nChanges:\n- Added optional name field to BasicBlock struct\n- Implemented with_name constructor for named blocks\n- Updated add_basic_block_with_name to use the name parameter\n- Enhanced pretty printing to show block names in output\n- Added descriptive names for edge blocks in critical edge splitting\n\nThis significantly improves the debugging experience by preserving\nthe semantic intent of control flow structures (then, else, merge,\nloop_header, etc.) throughout the compilation pipeline.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(mir): eliminate false positive validation warnings after SSA destruction\n\n- Add context-aware validation with check_ssa_invariants flag\n- Create new_post_ssa() constructor for post-SSA validation\n- Update standard pipeline to validate at appropriate stages\n- Skip SSA single-definition checks after SSA destruction\n- Add comprehensive tests for SSA and post-SSA validation\n\nThis fixes false warnings about multiple value definitions which are\nexpected and correct after phi node elimination.\n\n* feat(mir): enable U32 promotion in mem2reg pass\n\n- Relax DataLayout is_promotable to allow U32 and small aggregates\n- Add special handling for U32 in mem2reg with GEP protection\n- Mark U32 allocations with GEP access as escaping for correctness\n- Add comprehensive tests for U32 promotion scenarios\n- Update layout tests to reflect new promotability rules\n\nThis enables optimization of U32 arithmetic operations while maintaining\ncorrectness by preventing promotion when partial access (GEP) is used.\nFull multi-slot phi insertion remains TODO for complete SROA support.\n\n* feat(mir): implement backend-pluggable architecture for code generation\n\n- Add Backend trait system with validation and code generation interfaces\n- Implement CompilationPipeline for managing MIR optimizations and backend passes\n- Create CasmBackend adapter that implements Backend trait for CASM generation\n- Add comprehensive test coverage for backend pluggability\n- Maintain backward compatibility while enabling future extensibility\n\nThis architectural change enables:\n- Support for alternative backends (LLVM, Cranelift, WebAssembly)\n- Clean separation between MIR optimizations and backend code generation\n- Backend-specific optimization passes and configuration\n- Better testability with mock backends\n\n* refactor(mir): unify semantic type lookups with get_expr_type helper\n\n- Replace 14 duplicated semantic type lookup patterns with get_expr_type()\n- Leverages existing caching for improved performance\n- Eliminates ~28 lines of duplicated code\n- Maintains special cases for definition types and TypeData examination\n- All tests pass with identical behavior\n\n* refactor(mir): extract memory access patterns into builder helpers\n\n- Add load_field, store_field, load_tuple_element, store_tuple_element helpers\n- Add get_element_address helper for lvalue address calculations\n- Replace 8+ duplicated patterns across expr.rs and stmt.rs\n- Eliminate ~100+ lines of duplicated code\n- Centralize memory access logic for better maintainability\n\n* refactor(mir): standardize builder API naming conventions\n\n- Rename methods to follow consistent pattern: base/to/with\n- binary_op_with_dest -> binary_op_to, load -> load_to, etc.\n- Make automatic destination the default (load_value -> load)\n- Update all call sites to use new naming\n- Establish clear, predictable API patterns\n\n* refactor(mir): remove unused SSA aggregate instructions and improve code quality\n\nThis commit completes two major improvements to the MIR compiler infrastructure:\n\n## Removed Unused IR Operations (Task 011)\n\nEliminated 5 unused instruction kinds that were never generated by the lowering phase:\n- BuildStruct, BuildTuple: SSA aggregate construction\n- ExtractValue, InsertValue: SSA aggregate field manipulation\n- GetElementPtrTyped: Type-safe pointer arithmetic\n\nThese instructions added complexity without providing value since the compiler\nexclusively uses a memory-based approach for aggregates.\n\nChanges:\n- Removed instruction variants from InstructionKind enum\n- Simplified SROA pass to focus only on memory-based aggregates\n- Updated codegen to remove handlers for unused instructions\n- Rewrote tests to work without removed instructions\n\n## Quality of Life Improvements (Task 012)\n\n1. **Robust zero comparison**: FuseCmpBranch now handles both Integer(0) and Boolean(false)\n2. **Optimized pre-opt pass**: Reduced redundant use_counts computation from 3x to ~1.5x\n3. **Enhanced documentation**: Added detailed future considerations to DataLayout methods\n4. **Proper logging**: Replaced all eprintln! with log crate (error/warn/debug levels)\n\nThe compiler now has a cleaner, more maintainable codebase with reduced complexity\nand better engineering practices throughout.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(mir): address critical issues from MIR audit report\n\n- Disable broken SROA pass to prevent IR corruption\n- Fix dead-store elimination to properly track memory reads\n- Make Mem2Reg conservative: only promote scalar types without GEP uses\n- Fix validation to allow SSA uses from dominating blocks\n- Remove dominance frontier double-insertion\n- Replace recursive DFS with iterative version to prevent stack overflow\n- Remove inappropriate const fn on mutating methods\n- Add helper methods to reduce code duplication\n- Optimize function signature lookups with O(1) HashMap cache\n\nThese changes prioritize correctness over optimization effectiveness.\nSROA needs complete rewrite to properly handle constant GEPs.\n\n🤖 Generated with Claude Code\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* feat(mir): add first-class aggregate instructions for tuples and structs\n\nImplement four new InstructionKind variants to handle aggregates as first-class SSA values:\n- MakeTuple: Creates tuples from a list of values\n- ExtractTupleElement: Extracts elements from tuples by index\n- MakeStruct: Creates structs from field-value pairs\n- ExtractStructField: Extracts fields from structs by name\n\nThis foundational change enables cleaner MIR generation and eliminates the need for\nheavy memory-centric optimization passes like SROA and Mem2Reg for simple aggregate\noperations. The implementation is purely additive with no changes to existing behavior.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* feat(mir): implement value-based aggregate lowering for tuples and structs\n\nRefactor MIR lowering to generate value-based aggregate instructions instead of memory operations:\n\n- Tuple literals now generate MakeTuple instead of frame_alloc + stores\n- Struct literals now generate MakeStruct instead of memory operations\n- Tuple indexing uses ExtractTupleElement instead of get_element_ptr + load\n- Field access uses ExtractStructField instead of get_element_ptr + load\n\nThis change significantly simplifies the generated MIR and eliminates the need for\nheavy optimization passes like SROA and Mem2Reg for simple aggregate operations.\nThe implementation maintains backward compatibility for arrays and other constructs\nthat still require memory-based handling.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* feat(mir): implement InsertField and InsertTuple instructions for SSA assignment\n\n- Add InsertField and InsertTuple instruction variants to InstructionKind\n- Implement constructor functions and helper methods (destinations, used_values, validate)\n- Add pretty-print formatting for both new instructions\n- Create builder methods for type-safe instruction creation\n- Add comprehensive test suite validating all functionality\n- Update Task 004 documentation with implementation summary\n\nThese instructions enable SSA-based field/element updates without memory\noperations, supporting the transition from memory-centric to value-based\naggregate handling in the MIR.\n\n🤖 Generated with Claude Code (https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* feat(mir): implement conditional optimization pass execution\n\n- Add function_uses_memory() to detect memory operations in functions\n- Implement ConditionalPass wrapper for selective pass execution\n- Add add_conditional_pass() method to PassManager for conditional passes\n- Update standard_pipeline() to conditionally run Mem2RegSsaPass\n- Create comprehensive test suite for conditional pass behavior\n- Update Task 005 documentation with implementation summary\n\nThis optimization improves compilation performance by skipping expensive\nmemory-oriented passes for functions that only use value-based aggregates,\nwhile maintaining correctness for functions that require memory operations.\n\n🤖 Generated with Claude Code (https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* test(mir): fix aggregate instruction pretty print test assertions\n\nUpdate test assertions to match the new pretty print format for aggregate\ninstructions which no longer use underscores (e.g., 'maketuple' instead\nof 'make_tuple'). Also fix aggregate folding tests to use ConstFoldPass\nwhere aggregate folding now resides, and mark deprecated doc tests as\nignore to prevent compilation failures.\n\n- Update InsertField/InsertTuple test assertions for new format\n- Fix pretty print aggregate instruction test assertions\n- Update value-based lowering test assertions\n- Switch aggregate folding tests to use ConstFoldPass\n- Mark deprecated builder method doc tests as ignore\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(mir): critical correctness fixes for tuple offsets and aggregate lowering\n\nFixed critical bugs in MIR layer that were causing incorrect code generation:\n\n1. Tuple Element Offset Calculations:\n   - Fixed tuple element access to use DataLayout::tuple_offset() instead of raw indices\n   - Correctly handles wide types like u32 (2 slots) in tuple layouts\n   - Updated in lowering/expr.rs and lowering/stmt.rs for all tuple operations\n\n2. LowerAggregatesPass ValueId Allocation:\n   - Fixed hardcoded ValueIds that could collide with existing values\n   - Now uses function.new_typed_value_id() for proper unique allocation\n   - Derives types from destination values instead of guessing from literals\n\n3. InsertField/InsertTuple Data Preservation:\n   - Fixed data loss bug where unchanged fields were not copied\n   - Now properly copies all unchanged fields when creating updated aggregates\n   - Prevents corruption of struct/tuple data during field updates\n\n4. Const Method Corrections:\n   - Removed incorrect const qualifiers from cfg() and instr() methods\n   - These methods return builders that capture mutable references\n   - Fixes undefined behavior from const methods with mutable operations\n\n5. Added Comprehensive Tests:\n   - aggregate_offset_tests.rs: Tests for proper tuple offset calculations\n   - lower_aggregates_tests.rs: Tests for ValueId allocation and data preservation\n\nThese fixes ensure correct memory layout and data integrity for aggregate types\nthroughout the compilation pipeline.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* refactor(mir): complete value-based aggregate migration and fix optimization pipeline\n\nMajor changes to complete the aggregate-first MIR design:\n- Remove non-functional VarSsaPass from standard pipeline (implementation incomplete)\n- Fix lowering to use value-based operations for tuples (MakeTuple instead of memory ops)\n- Remove 'address as value' workarounds from mem2reg_ssa pass (no longer needed)\n- Remove redundant *_auto methods from MirBuilder API\n- Integrate PipelineConfig properly for environment-based optimization control\n\nThe MIR now consistently uses value-based operations for aggregates (tuples/structs),\nwith memory operations reserved for arrays and explicit address operations. This\nsimplifies optimization passes and produces cleaner, more optimizable IR.\n\nTests updated to handle optimization levels via CAIRO_M_OPT_LEVEL environment variable.\n\n* fix(mir): remove broken mem2reg passes and simplify pipeline\n\nThis commit removes the non-functional mem2reg_ssa pass implementation that was\ncausing issues with aggregate lowering. The pass had fundamental problems:\n- Never populated GEP tracking maps, preventing field-level promotion\n- Used overly conservative allocation tracking that blocked optimizations\n- Created incorrect assumptions about aggregate handling\n\nKey changes:\n- Remove mem2reg_ssa.rs and its tests completely\n- Simplify pipeline to use LowerAggregates directly in standard pipeline\n- Fix pipeline ordering to lower aggregates BEFORE any memory optimizations\n- Remove unused BackendTarget enum (was never properly integrated)\n- Update CASM backend validation to provide detailed error messages\n\nThe simplified pipeline now correctly:\n1. Lowers value-based aggregates to memory operations early\n2. Runs remaining optimizations on the lowered memory operations\n3. Validates that no aggregate operations reach CASM codegen\n\nThis fixes the struct test failures and ensures proper aggregate handling\nthroughout the compilation pipeline. Tests have been updated to use the\nCAIRO_M_BACKEND environment variable to control aggregate preservation\nfor value-based tests.\n\nResolves compilation failures for:\n- nested_structs\n- struct_as_function_parameter\n- struct_field_access\n- struct_field_access_2\n\n* delete old tasks\n\n* refactor(mir): remove broken optimization passes and simplify MIR pipeline\n\n      - Remove broken mem2reg passes: const_fold, ssa_destruction, var_ssa, lower_aggregates\n      - Simplify passes.rs by removing ~3000 lines of unused optimization code\n      - Update test infrastructure and snapshots to match simplified pipeline\n      - Clean up aggregate instruction tests and lowering logic\n      - Remove associated test files for deleted passes\n\n      This continues the MIR refactoring to focus on stable, working functionality\n      while removing complex optimization passes that were causing correctness issues.\n\n* refactor(mir): eliminate VoidCall instruction and unify call handling\n\nRemove the VoidCall instruction variant and replace it with a unified\nCall instruction approach. This eliminates duplicate logic across\noptimization passes and improves maintainability.\n\nKey changes:\n- Remove VoidCall from InstructionKind enum and related constructors\n- Update emit_call_and_discard_result to use Call with empty destinations\n- Simplify all analysis passes to handle only Call instructions\n- Update codegen to detect empty destinations for void calls\n- Remove tuple-index-on-call optimization shortcut for canonical IR\n- Add new optimization passes (dead_code_elimination, fuse_cmp)\n- Clean up documentation and migration files\n\nThis follows the canonical approach used by mature compiler IRs where\nall function calls use the same instruction format, improving correctness\nand reducing the risk of missing cases in optimization passes.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* minor api improvements\n\n* refactor(mir): consolidate CFG terminator setting and fix SSA construction\n\n* Consolidate inconsistent terminator setting logic in CfgBuilder with single\n  private helper function to ensure proper edge cleanup in both terminate()\n  and set_block_terminator() methods\n* Remove redundant successors field from BasicBlock - derive from terminator\n* Fix fragile phi operand logic using explicit block-value associations\n* Fix incomplete SSA construction for for loops with proper seal_block calls\n* Update test snapshots to reflect improved MIR generation\n\n🤖 Generated with Claude Code\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* feat(mir): implement optimization passes including SROA with nested struct fix\n\nThis commit introduces several optimization passes to the MIR pipeline:\n\n- **Scalar Replacement of Aggregates (SROA)**: Decomposes tuples and structs into\n  per-field SSA values, eliminating unnecessary aggregate construction. Includes\n  critical fix for nested struct handling where extracted aggregate fields properly\n  propagate their scalarized state.\n\n- **Copy Propagation**: Removes redundant assignments by replacing uses of copied\n  values with their original sources in SSA form.\n\n- **Local Common Subexpression Elimination (CSE)**: Eliminates redundant computations\n  within basic blocks by reusing previously computed values.\n\n- **Arithmetic Simplification**: Simplifies arithmetic operations including identity\n  operations, constant folding, and algebraic simplifications.\n\n- **Constant Folding**: Evaluates operations on constant operands at compile time.\n\n- **Branch Simplification**: Simplifies conditional branches with constant conditions\n  and fuses comparison-branch patterns.\n\nThe SROA implementation correctly handles:\n- Simple tuple/struct scalarization\n- Materialization at ABI boundaries (calls, stores)\n- Partial aggregate updates (InsertTuple/InsertField)\n- Aggregate copy forwarding\n- Nested struct extraction with state propagation\n\nAll passes preserve SSA form and the PHI-first basic block invariant. Comprehensive\ntest coverage included for each optimization pass.\n\n* feat(mir): implement phi elimination pass for SSA to stack machine conversion\n\nAdds comprehensive phi node elimination algorithm that converts SSA-form MIR\ninto stack machine compatible code by inserting copy instructions at block\nboundaries. The implementation handles complex control flow patterns including\nloops, nested conditions, and critical edges.\n\nKey changes:\n- Implement PhiElimination pass with dominance-based copy placement\n- Add critical edge splitting for correct phi semantics\n- Update optimization pipeline to include phi elimination before codegen\n- Add comprehensive tests and demo example\n- Update MIR snapshots to reflect new pass output\n\nThe pass correctly handles:\n- Simple convergence patterns (if-then-else)\n- Loop carried dependencies\n- Nested control flow structures\n- Multiple phi nodes per block\n- Self-referential phi nodes in loops\n\n* fix(mir): ensure we dont aggresively remove scalars that are used in other blocks\n\n* feat(mir): implement aggregate lowering for structs and tuples\n\nImplemented register-based aggregate lowering from MIR to CASM:\n- Added support for MakeStruct, ExtractStructField, InsertField operations\n- Added support for MakeTuple, ExtractTupleElement, InsertTuple operations\n- Refactored InstructionKind::Assign to handle all types including aggregates\n- Aggregates are laid out as contiguous register sequences\n- Zero-copy field extraction by mapping directly to source locations\n- In-place updates for InsertField/InsertTuple operations\n\nThe implementation follows the aggregate_lowering_plan.md with optimizations\nto avoid unnecessary copies. Multi-slot values (u32, nested aggregates) are\nproperly handled with slot-by-slot copying.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(semantic): tuple field mutation validation\n\n* fix(mir): correct SSA construction for loops with mutable variables\n\nFixed SSA construction issues in the MIR compiler that caused incorrect values\nto be returned from loops. The problems were:\n\n1. In seal_block(), when phi nodes were completed and found to be trivial\n   (all operands the same), the variable map wasn't being updated with the\n   replacement value, causing stale phi values to be used.\n\n2. The implementation now correctly handles the case where a trivial phi is\n   replaced, ensuring the variable map reflects the actual value to use.\n\nThis fixes:\n- Loops returning incorrect values after break statements\n- Nested loops creating undefined variable references\n\nThe fix ensures the SSA construction follows the Braun et al. algorithm\ncorrectly, with proper handling of trivial phi elimination.\n\n* feat(parser,semantic,mir): add support for nested tuple patterns in let statements\n\nImplement recursive pattern matching for nested tuples in let statements,\nallowing destructuring of complex nested tuple structures like `let (a, b, (c, d)) = tuple`.\n\nChanges:\n- Parser: Update Pattern enum to support recursive tuple patterns\n- Semantic: Update type resolution to navigate nested tuple types using paths\n- MIR: Update lowering to recursively handle nested pattern destructuring\n\nThis enables more expressive pattern matching and cleaner code when working\nwith functions that return nested data structures.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* refactor(mir): improve SROA pass with better scalarization analysis\n\nEnhance the Scalar Replacement of Aggregates (SROA) optimization pass with\ncomprehensive documentation and improved recursive forward-looking analysis\nfor determining which aggregates can be safely scalarized.\n\nChanges:\n- Add detailed algorithm documentation explaining scalarization rules\n- Improve recursive dependency checking for nested aggregates\n- Clean up code formatting and remove unused imports\n- Update snapshots for affected test cases\n\nThe improved SROA pass better handles complex aggregate patterns and provides\nclearer reasoning about when aggregates can be decomposed into scalars.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix clippy lints\n\n* refactor(mir): improve constant folding and M31 field arithmetic\n\n- Simplify M31 arithmetic operations by using raw field values directly\n- Change immediate value types from i32 to u32 for consistency\n- Make m31_to_i32 conversion function const\n- Update constant folding to handle field arithmetic more efficiently\n- Fix type mismatches in codegen for immediate value handling\n- Update test snapshots to reflect improved field arithmetic\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* refactor(parser): remove constant folding from parser\n\nRemove compile-time constant evaluation from the parser phase to improve\narchitectural separation of concerns. Constant folding should be done in\nMIR optimization passes where proper type information is available.\n\nChanges:\n- Remove try_evaluate_binary_op function and calls\n- Delete constant_folding test module\n- Update snapshots to preserve AST structure\n- Add comprehensive task documentation for MIR improvements\n\nThis fixes the architectural issue where optimization logic was mixed\nwith syntax parsing, and sets up proper constant folding implementation\nin the MIR pipeline with correct M31 field arithmetic semantics.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* refactor(mir): centralize constant evaluation and remove all logging\n\n- Create centralized ConstEvaluator module for consistent constant evaluation semantics\n- Refactor all optimization passes to use the centralized evaluator\n- Remove all environment variable checks and logging statements from MIR passes\n- Simplify validation functions to run silently without diagnostics\n- Update tests to work without environment variable configuration\n- Add comprehensive tests for edge cases in M31 field and U32 arithmetic\n- Clean up unused imports and variables\n\nThis simplifies the MIR optimization pipeline by removing all logging infrastructure\nwhile ensuring consistent constant evaluation semantics across all passes.\n\n* refactor(mir): remove dead code\n\n* fix(mir): fix critical correctness and safety issues in MIR\n\n- Replace debug_assert with assert in Instruction::call to prevent SSA corruption in release builds\n- Add comprehensive return type validation to catch type mismatches between returns and function signatures\n- Fix pretty printer to use Display trait for BinaryOp instead of Debug, enabling proper MIR round-tripping\n- Replace silent early returns with explicit panics on invalid block IDs to prevent CFG corruption\n- Collect and return lowering errors instead of silently logging them to prevent invalid module generation\n- Update test infrastructure to properly set return_values field for validation compatibility\n\nThese fixes address data corruption risks, type system soundness holes, and silent failure modes that could produce incorrect compiler output.\n\n* test(mir): fix tests after stricter validation\n\nUpdate test expectations and helper functions to comply with the stricter\nvalidation rules introduced in the critical bug fixes:\n\n- Update test_missing_imported_function_error to expect proper error\n  propagation instead of silent error recovery\n- Fix phi elimination test helpers to set return_values field correctly\n- Ensure all test functions that create Returns also set return_values\n\nAll 149 MIR tests now pass with the improved correctness checks.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* add readme\n\n* fix(mir): properly remove unreachable blocks in DeadCodeElimination pass\n\nInstead of just clearing unreachable blocks, the pass now:\n- Completely removes unreachable blocks from the CFG\n- Compacts remaining blocks into a dense IndexVec\n- Remaps all block references in terminators to new IDs\n\nThis ensures clean MIR output and prevents codegen errors when\nprocessing blocks that become unreachable after optimization passes\nlike SimplifyBranches.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix(compiler): resolve tuple vs parentheses parsing confusion\n\n- Fix parser precedence so (var) is parsed as parenthesized expr not tuple\n- Handle empty tuple literals in MIR return statement generation\n- Resolves issues where (tempvar) + 2 failed and return() caused errors\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>\n\n* fix remaining tests\n\n---------\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-08-22T15:50:06+02:00",
          "tree_id": "b3428a464c4e5a1619272a890d51b148c6c28997",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/84745806085e33f985f451da90a9a8f1616f3b2e"
        },
        "date": 1755870841439,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 338746896,
            "range": "± 3822534",
            "unit": "ns/iter"
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
          "id": "697794551266c5411c43662b9fa612b8c0475f35",
          "message": "feat(runner): implement U32 bitwise AND, OR, XOR opcodes (#234)\n\nAdd support for bitwise operations on U32 types in the runner:\n- U32StoreAndFpFp, U32StoreOrFpFp, U32StoreXorFpFp for register-register ops\n- U32StoreAndFpImm, U32StoreOrFpImm, U32StoreXorFpImm for register-immediate ops\n\nFollowing the same implementation pattern as U32 comparison opcodes.",
          "timestamp": "2025-08-22T16:33:14+02:00",
          "tree_id": "c440f256b08d40531234ba62af265ac484dc4169",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/697794551266c5411c43662b9fa612b8c0475f35"
        },
        "date": 1755873389155,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 340661347,
            "range": "± 2364152",
            "unit": "ns/iter"
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
        "date": 1756132022165,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 335568571,
            "range": "± 1466573",
            "unit": "ns/iter"
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
          "id": "3b2ca46d2beb10c4017d0887af01757106a611ab",
          "message": "feat(codegen): implement fixed-size array compilation to CASM (#246)\n\nAdd comprehensive code generation for fixed-size arrays:\n\nCore Array Operations:\n- Implement `make_fixed_array` to materialize arrays in contiguous memory\n- Add `extract_array_element` for static index access with offset calculation\n- Implement `dynamic_array_index` with runtime bounds checking and pointer arithmetic\n- Add `insert_array_element` for functional array updates with element copying\n\nMemory & Layout Management:\n- Extend function call ABI to pass arrays as pointers (single slot)\n- Update memory layout calculation to handle array types\n- Add proper handling of zero-sized arrays and unit types\n- Implement efficient element copying with size-aware optimizations\n\nCode Generation Features:\n- Arrays materialize to contiguous stack memory when needed\n- Static indexing compiles to direct memory access\n- Dynamic indexing uses pointer arithmetic with runtime offset calculation\n- Arrays passed to functions as pointers (fp + offset)\n- Proper support for nested types and multi-slot elements (e.g., U32)\n\nTesting & Integration:\n- Add comprehensive test coverage with mdtest snapshots\n- Update compiler dependencies and cargo configuration\n- Extend builder interface for array operations\n- Integrate with existing type system and memory management\n\nThis implementation provides efficient compilation of array operations\nwhile maintaining compatibility with the existing CASM instruction set\nand memory model.\n\n🤖 Generated with [Claude Code](https://claude.ai/code)\n\nCo-authored-by: Claude <noreply@anthropic.com>",
          "timestamp": "2025-08-25T16:52:52+02:00",
          "tree_id": "9c4180d01963066af1305ca09fced9e854c48f5b",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/3b2ca46d2beb10c4017d0887af01757106a611ab"
        },
        "date": 1756133993150,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 338729356,
            "range": "± 2883700",
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
          "id": "d00094085499aa2308387200b8c100c767cf23b6",
          "message": "feat(runner): add PrintM31 and PrintU32 debug opcodes (#250)\n\n* feat(runner): add PrintM31 and PrintU32 debug opcodes\n\nAdd non-tracing print instructions for debugging Cairo-M programs:\n- PrintM31: prints M31 field element at [fp + offset]\n- PrintU32: prints 32-bit unsigned integer at [fp + offset]\n- Output format: [PrintM31/U32] [address] = value\n\nThese are true no-op instructions that don't modify execution trace.\n\n* test: add missing test cases for PrintM31 and PrintU32 opcodes",
          "timestamp": "2025-08-25T19:59:29+02:00",
          "tree_id": "c402498b03edd99bd594e8430823452b711d531f",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/d00094085499aa2308387200b8c100c767cf23b6"
        },
        "date": 1756144998527,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 337717102,
            "range": "± 1227943",
            "unit": "ns/iter"
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
          "id": "75ef031d2760d3ab05b0cebdc607103cf5078df0",
          "message": "feat(common): add codec support for fixed-size arrays (#248)\n\n* Adapt AbiCodec for fixed-size arrays\n\n* fix: SROA pass too aggressive on arrays inside structs & tuples\n\n* refactor(runner): improve code readability and naming conventions",
          "timestamp": "2025-08-26T15:22:39+02:00",
          "tree_id": "a22b82651f8baa67dcc2d659e80c6420467ad474",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/75ef031d2760d3ab05b0cebdc607103cf5078df0"
        },
        "date": 1756214807879,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 336767341,
            "range": "± 1484332",
            "unit": "ns/iter"
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
          "id": "2d5597181159816eec0c315e49c34445929029a7",
          "message": "feat(compiler): make `while` statements parenthesis optional (#256)\n\n* Support while statement without parentheses in parser and formatter\n\nCo-authored-by: msaug <msaug@protonmail.com>\n\n* remove parenthesis in all examples\n\n---------\n\nCo-authored-by: Cursor Agent <cursoragent@cursor.com>",
          "timestamp": "2025-08-27T12:00:51+02:00",
          "tree_id": "ef0f843a7829cf5be1a09c2be567dc1bb0161df0",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/2d5597181159816eec0c315e49c34445929029a7"
        },
        "date": 1756289067552,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 338233137,
            "range": "± 1238977",
            "unit": "ns/iter"
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
          "id": "df1f906f859f1d1887d2f8ff6f0b4db174290cb3",
          "message": "feat(runner): add LowerThan, AssertEqFpFp and AssertEqFpImm instructions (#254)\n\nfeat(runner): add AssertEqFpFp and AssertEqFpImm instructions\n\n- Add AssertionFailed error variant for assertion instructions\n- Implement AssertEqFpFp for comparing two FP-relative values\n- Implement AssertEqFpImm for comparing FP-relative value with immediate\n- Add comprehensive tests for assertion instructions\n- Register new opcodes 49 and 50 for assert instructions\n\nCloses CORE-1147\n\nfeat(runner): add StoreLowerThanFpImm instruction\n\n- Implement StoreLowerThanFpImm for less-than comparisons\n- Store result as 0 (false) or 1 (true) in M31 field\n- Register opcode 48 for the new instruction\n- Add tests for comparison instruction\n\nCloses CORE-1145",
          "timestamp": "2025-08-27T13:19:37+01:00",
          "tree_id": "3f4604f4f8e37cf42b26fe057d456e22a01880b2",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/df1f906f859f1d1887d2f8ff6f0b4db174290cb3"
        },
        "date": 1756297399681,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 330181500,
            "range": "± 1112963",
            "unit": "ns/iter"
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
          "id": "11d9ea49be47c3ed020d1df6aa068792addf2a80",
          "message": "feat(compiler): add type cast support from u32 to felt (#255)\n\n* feat(compiler): add type cast support from u32 to felt\n\n- Add Cast expression variant to parser with 'as' keyword\n- Implement precedence rules: cast binds tighter than logical, looser than arithmetic\n- Add semantic validation for cast expressions (only u32 to felt allowed)\n- Implement MIR lowering for cast operations\n- Generate efficient CASM code using StoreLowerThanFpImm for range checking\n- Add comprehensive test coverage with snapshots at all compiler phases\n- Update mdtest documentation with type casting examples\n\nThe implementation converts u32 to felt by:\n1. Validating high limb < 2^15 using StoreLowerThanFpImm\n2. Asserting the validation passes\n3. Computing result as lo + (hi * 2^16)\n\n* fix snaps\n\n* edit snaps\n\n* fix parser infinite recursion on tuple parenthesis\n\n* update semantic snap\n\n* fix bound checks\n\n* fix doc\n\n* fix snapshots\n\n* fix mdtest runner",
          "timestamp": "2025-08-28T10:50:49+02:00",
          "tree_id": "22468bf1182bffe549d6c03c1ac563724dacecab",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/11d9ea49be47c3ed020d1df6aa068792addf2a80"
        },
        "date": 1756371320590,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 335708465,
            "range": "± 2619664",
            "unit": "ns/iter"
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
          "id": "78ad3a55191f40f2d50318e0c9f880a8fce6244c",
          "message": "feat(runner): add crate dependency support for mdtest runner (#268)\n\n* feat(runner): add crate dependency support for mdtest runner\n\nEnable mdtest runner to use external crates like stwo_prover in Rust test snippets\nby implementing rustc compilation with --extern flags. This allows field arithmetic\ntests to use M31 types directly from stwo_prover.\n\nChanges:\n- Add serde_json as dev dependency for parsing cargo build JSON output\n- Implement build_and_discover_crates() to find rlib paths via cargo --message-format=json\n- Enhance run_rust_code() to pass --extern and -L flags to rustc\n- Add special handling for M31 return types in test wrapper code\n- Update arithmetic mdtest examples to demonstrate M31 usage\n\nCORE-1168\n\n* update snapshots",
          "timestamp": "2025-08-28T16:48:31+02:00",
          "tree_id": "6981eb40108784647d0ccfba55ea6045c78ff35b",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/78ad3a55191f40f2d50318e0c9f880a8fce6244c"
        },
        "date": 1756392845330,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 332592234,
            "range": "± 1917042",
            "unit": "ns/iter"
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
          "id": "f3029fd86483114f24f2a8ac5a6fab22c6347152",
          "message": "refactor(compiler): remove dead code and restrict visibility (#276)\n\n- Remove unused Store, Load, GEP instruction variants from MIR\n- Delete obsolete control_flow.rs and conditional_pass_test.rs files\n- Change public functions to pub(crate) where not externally used\n- Enable Rust dead code detection by restricting visibility\n- Clean up unused imports and functions across compiler crates\n- Simplify instruction handling by removing pointer-based operations\n\nThis cleanup reduces the codebase by ~3000 lines and improves maintainability\nby leveraging Rust's dead code analysis. Part of the aggregate-first MIR\nmigration that eliminated pointer-based memory operations.",
          "timestamp": "2025-08-28T21:32:13+02:00",
          "tree_id": "ad9add36ab41abd7a6fa0ffcdce9676f9169c252",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/f3029fd86483114f24f2a8ac5a6fab22c6347152"
        },
        "date": 1756409806668,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 335244541,
            "range": "± 2040349",
            "unit": "ns/iter"
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
        "date": 1756485024145,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 337796972,
            "range": "± 2518336",
            "unit": "ns/iter"
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
          "id": "30dc8bd129d95b2b85b2224b2783dfc298ad5155",
          "message": "refactor(common,runner): improve instruction macro system for type safety (#279)\n\nRefactor the instruction definition macro in the common crate to:\r\n- Add structured field-based instruction definitions\r\n- Include operand type metadata for better validation\r\n- move extract_as! macro for safe field extraction in common\r\n\r\nThis makes the instruction system more maintainable and less error-prone by:\r\n- Centralizing instruction definitions with their types\r\n- Providing compile-time field validation\r\n- Eliminating manual pattern matching boilerplate\r\n- calculating size automatically.\r\n\r\nThis is not the ideal nor final design, but it's a small improvement until a bigger re-think of this system.\r\n\r\nPart of CORE-1142",
          "timestamp": "2025-09-01T09:58:51+01:00",
          "tree_id": "5c7b2053563886f54af51d30816209049dc90d35",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/30dc8bd129d95b2b85b2224b2783dfc298ad5155"
        },
        "date": 1756717409783,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 336336139,
            "range": "± 1830759",
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
          "id": "79e1fe09a74350ac7c39a79965a8ac34e2166b9d",
          "message": "fix(runner): handle division by zero in u32_store_div_fp_imm (#283)\n\nReplace macro-generated function with custom implementation that checks for\ndivision by zero and returns InvalidOperand error instead of panicking.\n\nThis approach was chosen over modifying exec_u32_bin_op_fp_imm because:\n- No performance overhead for other arithmetic operations\n- Minimal code change with clear error handling\n- Avoids breaking changes to the existing macro system\n\nThe ~20 lines of code duplication is acceptable given the performance\nbenefits and simplicity of the solution.",
          "timestamp": "2025-09-01T11:20:08+02:00",
          "tree_id": "29a93dec17a77c3441537ecc45d7c5fc2d3b06d5",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/79e1fe09a74350ac7c39a79965a8ac34e2166b9d"
        },
        "date": 1756718676643,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 335007704,
            "range": "± 7180675",
            "unit": "ns/iter"
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
        "date": 1756724019676,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 338413636,
            "range": "± 1040489",
            "unit": "ns/iter"
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
        "date": 1756808671953,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 337676296,
            "range": "± 1484134",
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
          "id": "6e445f6200983bcab753970e14254fac854574b7",
          "message": "feat(prover): add double_derefs and store_frame_pointer (#294)\n\n* feat: added files and boiler plate with AIR in comments (code not matching)\n\n* feat: added double_derefs (not fully tested)\n\n* feat: add store_frame_pointer and fix VM reading order\n\n* typo: fixed copy paste typo\n\n* merge double-deref-fp-fp\n\n* merge double_deref_fp_imm\n\n* merged opcodes\n\n* update tests\n\n* remove debugs\n\n* Fix component docstring\n\n---------\n\nCo-authored-by: Clément Walter <clement0walter@gmail.com>",
          "timestamp": "2025-09-03T19:38:15+03:00",
          "tree_id": "1bac71094ceb20c1f5513582e9585e35a1da305a",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/6e445f6200983bcab753970e14254fac854574b7"
        },
        "date": 1756917836417,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 342360575,
            "range": "± 1061021",
            "unit": "ns/iter"
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
        "date": 1756984009584,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 343988283,
            "range": "± 1767848",
            "unit": "ns/iter"
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
          "id": "e5ff1be3d8e7d04ff4622369496c600f25188563",
          "message": "feat(mir): implement constant propagation optimization pass (#303)\n\n* feat(mir): implement constant propagation optimization pass\n\nAdds a new constant propagation pass to the MIR optimization pipeline that:\n- Evaluates arithmetic operations on constant values at compile time\n- Propagates known constant values through the IR\n- Folds operations like 1 + 2 + 4 into 7\n\nThis significantly reduces the number of runtime operations needed for\nconstant expressions, improving both code size and execution performance.\n\nCloses https://linear.app/kkrt-labs/issue/CORE-1165\n* prevent deadcode pass from optimizing out array inserts",
          "timestamp": "2025-09-05T12:18:01+02:00",
          "tree_id": "6b76cf321c964c7a3936e9ee6b208aae6201a10f",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/e5ff1be3d8e7d04ff4622369496c600f25188563"
        },
        "date": 1757067815700,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 344205317,
            "range": "± 1171634",
            "unit": "ns/iter"
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
        "date": 1757323737123,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 339140431,
            "range": "± 4010890",
            "unit": "ns/iter"
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
        "date": 1757583552081,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 347726735,
            "range": "± 2484305",
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
          "id": "9b970cfc0ad8631f995e7f0e5a20bbcaee1ebb67",
          "message": "opti: add codegen-units and lto settings (#316)",
          "timestamp": "2025-09-12T09:29:17+03:00",
          "tree_id": "5058f81b318b64c2ca86189c2200ded68ee4a402",
          "url": "https://github.com/kkrt-labs/cairo-m/commit/9b970cfc0ad8631f995e7f0e5a20bbcaee1ebb67"
        },
        "date": 1757658999154,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 346133943,
            "range": "± 2700104",
            "unit": "ns/iter"
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
        "date": 1757947736472,
        "tool": "cargo",
        "benches": [
          {
            "name": "fibonacci_1m/execution_only",
            "value": 291541720,
            "range": "± 3436934",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}