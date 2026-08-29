# Load testing the read endpoints

The API serves all reads from an in-memory snapshot, so response times
should be fast and consistent. `scripts/load_test.sh` establishes a
repeatable baseline against the read endpoints using
[`hey`](https://github.com/rakyll/hey):

```
cargo run --release &         # start the API
./scripts/load_test.sh        # defaults: http://localhost:8080, asset id "1"
```

Override the target or asset id with positional args, and the request
volume with env vars:

```
REQUESTS=5000 CONCURRENCY=100 ./scripts/load_test.sh http://localhost:8080 42
```

## Endpoints covered

- `GET /health`
- `GET /stats`
- `GET /assets`
- `GET /assets/:id`
- `GET /assets/:id/holders`
- `GET /assets/:id/compliance`
- `GET /assets/:id/dividends`

## Reading the output

`hey` reports, per endpoint: total time, requests/sec, and latency
distribution (p50/p90/p95/p99). Because everything is served from memory,
watch for:

- **p99 latency** climbing far above p50 — indicates lock contention or
  GC-like pauses (allocation pressure) under concurrent reads.
- **Requests/sec** dropping as concurrency increases — indicates the
  handler or snapshot access is not scaling with load.
- **Non-2xx status codes** in the summary — indicates errors under load
  that don't show up in single-request testing.

## Baseline

No numbers are checked into this file yet — run the script against a
representative snapshot size and record the p50/p95/p99 and RPS per
endpoint here (or in your tracking issue) as the reference baseline.
Re-run after significant handler or snapshot changes and compare against
that baseline to catch regressions.
