#!/usr/bin/env python3
"""Check the runner and require each planted fault to fail its focused test."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
FAULTS = [
    ("missing-lock-event", 'record_phase(run, "lock_acquired")',
     'pass  # planted missing event',
     "test_phases_cover_success_with_monotonic_and_wall_clock_identities"),
    ("premature-lock-event",
     '        fcntl.flock(lock, fcntl.LOCK_EX)\n        open(f"{run}/LOCKED", "w").close()\n        record_phase(run, "lock_acquired")',
     '        record_phase(run, "lock_acquired")\n        fcntl.flock(lock, fcntl.LOCK_EX)\n        open(f"{run}/LOCKED", "w").close()',
     "test_lock_acquired_event_is_emitted_under_the_lock"),
    ("rewrite-terminal-record",
     '    if os.path.exists(f"{run}/STATUS"):\n        return  # a terminal run and its phase record stay immutable',
     '    # planted terminal record rewrite',
     "test_terminal_records_survive_an_executor_restart"),
    ("unlocked-verification", '        verification = verify(run, m)',
     '        if lock:\n            lock.close()\n        verification = verify(run, m)',
     "test_preparation_and_job_both_hold_the_measurement_lock"),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    Path.home().joinpath("work").mkdir(exist_ok=True)
    replace = shutil.which("replace")
    assert replace, "the exact-replacement tool 'replace' is required for fault controls"
    env = dict(os.environ, PINNED_RUN_TOOL=str(HERE / "pinned-run"))
    suite = subprocess.run([sys.executable, str(HERE / "test_pinned_run.py"), "-v"], env=env,
                           text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    args.output.joinpath("tests.log").write_text(suite.stdout)
    assert suite.returncode == 0, suite.stdout
    print("runner suite passed", flush=True)
    for name, old, new, test in FAULTS:
        with tempfile.TemporaryDirectory(prefix="pinned-run-control-") as temp:
            tool = Path(temp) / "pinned-run"
            shutil.copy2(HERE / "pinned-run", tool)
            change = subprocess.run([replace, str(tool), old, new], text=True,
                                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            assert change.returncode == 0, change.stdout
            result = subprocess.run([sys.executable, str(HERE / "test_pinned_run.py"), "Server." + test],
                                    env=dict(env, PINNED_RUN_TOOL=str(tool)), text=True,
                                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            args.output.joinpath(name + ".log").write_text(change.stdout + result.stdout)
            assert result.returncode == 1 and "FAIL: " + test in result.stdout, result.stdout
            print("control rejected:", name, flush=True)
    print("pinned-run phase checks passed", flush=True)


if __name__ == "__main__":
    main()
