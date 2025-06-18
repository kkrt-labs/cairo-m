window.BENCHMARK_DATA = {
  "lastUpdate": 1750261231364,
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
      }
    ]
  }
}