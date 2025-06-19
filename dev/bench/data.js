window.BENCHMARK_DATA = {
  "lastUpdate": 1750336585148,
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
      }
    ]
  }
}