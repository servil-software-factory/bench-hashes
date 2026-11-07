# pinned-run: fast-feedback pilot

Public source for Sam Shackleton's existing remote runner, used by the
[development-cycle tracker](https://github.com/servil-software-factory/bench-hashes/issues/146)
and [first pilot](https://github.com/servil-software-factory/bench-hashes/issues/147).
The runner's module documentation describes its manifest and commands.

Run the suite and its six fault controls with one command:

```sh
python3 tools/pinned-run/check.py --output /tmp/pinned-run-check
```

It uses Python's standard library and the project's `replace` exact-replacement
tool. Tests use fake transports, temporary run directories and mock jobs.
Fault controls omit a phase, report a lock prematurely, rewrite a terminal
record, release the lock before verification, bypass a failed preflight, and
accept mismatched preflight inputs. Each must fail its named test. The output
directory retains the suite and control logs.

## First slice: automatic phase records

Every new submission writes `phases.jsonl`. The existing fetch operation
includes it automatically. Each JSON line has `version: 1`, `phase`, integer
`utc_ns` and `monotonic_ns`, and the Linux host's `boot_id`.

| Event | Meaning / additional fields |
|---|---|
| `accepted` | The server recorded the manifest, before starting its executor. |
| `executor_started` | This executor owns the run's exclusive STARTED marker. |
| `lock_acquired` | The executor holds the manifest's lock. Present for locked jobs. |
| `prepared` | Source/script preparation completed; `ok` states whether it passed. |
| `job_finished` | The script returned; `exit_code` is its exit status. |
| `verification_finished` | Output checks returned; `ok` states whether they passed. |
| `finished` | Normal executor completion; `status` is VERIFIED or FAILED. |

Subtract monotonic timestamps within the same `boot_id` to separate dispatch,
lock waiting, preparation, script execution and verification. UTC correlates
records with other systems. A failed preparation has no job/verification event.
A cancelled, crashed or restarted executor can leave a prefix; STATUS/REASONS
retain the outcome. A terminal run's record is left unchanged on re-execution.

These are workflow events surrounding the script. Workload timing remains in
clocks. The benchmark lock still covers preparation, the script and verification.
Build/measurement subphases belong to the instrument and follow in the pilot.

## Preflight before fan-out

```sh
pinned-run fanout root@FAST_HOST manifest-a.json manifest-b.json
```

The manifests name one instrument: identical script bytes, repository commits
and paths, environment and forbid rules. Hosts, names and output expectations
may differ. The runner captures every manifest and the script bytes first,
checks that identity, then submits one preflight on FAST_HOST under the first
manifest's lock. It sets `PINNED_RUN_PREFLIGHT=1` and requires
`out/preflight.txt` to contain `pinned-run preflight complete`.

The script's preflight mode compiles the exact probe against its named sources
and runs its small reference/work checks. It writes the marker after they pass.
A VERIFIED preflight permits fan-out of the captured inputs. FAILED, CANCELLED
or lost preflight stops the command before any target launch. A later local edit
cannot change what the preflight authorized. The normal jobs retain all of their
original output expectations. Different instruments use separate preflights.

`fanout-example.sh` is a compiler/reference fixture: it builds a small Rust
probe against the named library, checks the official `abc` digest, and supports
a deliberate compile fault. Its pinned runs demonstrate both successful fan-out
and failure before fan-out. It requires repositories named `library` and `tools`.
The generic gate checks instrument identity and the VERIFIED completion; the
instrument owns its preflight's useful compile/reference/work assertions.

## Evidence and deployment

The [latency audit and pinned ARM/Zen 3 results](https://github.com/samshackleton-tech/bench-hashes/blob/7a248f94aed0f3183e87d5773659b0daa291fa71/process/fast-feedback/STARTED.md)
are separate from this focused implementation branch. The runner bytes here
are those already checked through pinned-run and installed at `~/bin/pinned-run`.
Its existing request transport deploys those same bytes to a bench host on
first use. The fast-lane bootstrap uses a bounded setup and is followed by
`bench-lockdown`'s two-key verification before any measurement job.
