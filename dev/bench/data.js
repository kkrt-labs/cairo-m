window.BENCHMARK_DATA = {
  "lastUpdate": 1754067882100,
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
      }
    ]
  }
}