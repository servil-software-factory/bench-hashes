# pinned-run: fast-feedback pilot

Public source for Sam Shackleton's existing remote runner, used by the
[development-cycle tracker](https://github.com/servil-software-factory/bench-hashes/issues/146)
and [first pilot](https://github.com/servil-software-factory/bench-hashes/issues/147).
The runner's module documentation describes its manifest and commands.

Run the suite and its sixteen fault controls with one command:

```sh
python3 tools/pinned-run/check.py --output /tmp/pinned-run-check
```

It uses Python's standard library, Git and OpenSSH. Tests use fake transports,
temporary Git repositories and mock jobs. Controls edit temporary copies with an
asserted single exact match. They omit a phase, report a lock prematurely,
rewrite a terminal record, release the lock before verification, bypass a failed
preflight, accept mismatched inputs, ignore a study's scope, skip its review,
hide a Slower judgment, import unrelated refs, refresh an offline known commit,
accept the wrong HEAD, disable host-key checking, accept a changed host key and
ignore a transport failure and permit implicit authentication identities.
Each must fail its named test. The output directory retains the logs.

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

## Host authentication

Obtain the server's ED25519 public host key through its trusted provisioning
channel, verify its fingerprint, then enroll it before the first request:

```sh
pinned-run trust root@HOST host-key.pub
```

The bounded bootstrap saves `/root/fast-lane-host-key.pub` and its fingerprint.
The client records the public key in `~/.ssh/known_hosts`. Re-enrollment is
idempotent; a changed key stops for operator review. Requests require a matching
known ED25519 key with strict checking. Connection sharing uses a new `pinned-`
namespace, separate from earlier unverified SSH sessions. Transport and host-key
failures stop optional version/status requests too. `IdentitiesOnly=yes` keeps
requests on the named everyday key. The interactive shell key retains its role.

## Fresh checkouts from exact commits

Preparation uses the existing hash-named mirror cache. A commit already in the
cache needs no remote refresh. An absent commit is fetched by its full hash.
Every run initializes a fresh private repository, fetches just the named commit
and its full history from that cache, and verifies HEAD before running the job.
Other cached branches and their unrelated results stay in the mirror. Targets,
lockfiles and working trees remain private to the run.

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

## Study-scoped collection

Add `"study": "my-change"` to a job manifest, then:

```sh
pinned-run next --study my-change --fetch results
pinned-run pending --study my-change
```

`next` selects only that study's registered runs and contacts only its hosts.
It returns the next finished, unreported result, including a result that finished
before the command started. It polls pending jobs once a second. An optional
positional number bounds the wait in seconds; exit 3 means all results have
already been reported, exit 4 means the wait expired. `--help` describes the
options. Jobs launched without a study retain the existing unscoped behavior.

With `--fetch`, the same command fetches artifacts and writes `REVIEW.md` plus
`review.json` in the run's result directory. Failed results use the existing
`FAILED-` prefix and retain their failure reasons. A result is marked reported
only after fetching and summary generation complete, so interrupted collection
can be retried. Collection requires a study name.

The summary names the sources, script and runner, derives workflow intervals
from version-1 monotonic phase records on one boot, and copies counts and every
attention row from the instrument's `judgments*.tsv` files. It keeps Slower and
unjudged rows beside the runner's VERIFIED status. The existing instrument
makes every statistical judgment; the summary reads its TSV and links its logs.
It stops on malformed phase or judgment records.

## Evidence and deployment

The [latency audit and pinned ARM/Zen 3 results](https://github.com/samshackleton-tech/bench-hashes/blob/7a248f94aed0f3183e87d5773659b0daa291fa71/process/fast-feedback/STARTED.md)
are separate from this focused implementation branch. The runner bytes here
are those already checked through pinned-run and installed at `~/bin/pinned-run`.
Its existing request transport deploys those same bytes to a bench host on
first use. The fast-lane bootstrap uses a bounded setup and is followed by
`bench-lockdown`'s two-key verification before any measurement job.
