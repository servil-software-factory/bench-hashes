#!/usr/bin/env python3
"""Check the runner and require each planted fault to fail its focused test."""
import argparse
import difflib
import os
from pathlib import Path
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
     "test_a_run_cancelled_before_it_starts_executes_nothing"),
    ("unlocked-verification", '        verification = verify(run, m)',
     '        if lock:\n            lock.close()\n        verification = verify(run, m)',
     "test_preparation_and_job_both_hold_the_measurement_lock"),
    ("slow-wait-poll", '            time.sleep(1)\n        print(s)',
     '            time.sleep(20)\n        print(s)',
     "Client.test_wait_polls_once_a_second_and_preserves_terminal_result"),
    ("failed-preflight-fanout", '    if state != "VERIFIED":',
     '    if False:  # planted failed gate',
     "Client.test_failed_preflight_launches_no_fanout_jobs"),
    ("mismatched-preflight-inputs", '        if jobs and identity != jobs[0][2]:',
     '        if False:  # planted unchecked inputs',
     "Client.test_fanout_refuses_different_instrument_inputs_before_any_submit"),
    ("unscoped-collection", '    runs = [r for r in registered() if study is None or r.get("study") == study]',
     '    runs = registered()  # planted ignored scope',
     "Client.test_scoped_next_ignores_other_studies_and_contacts_only_its_hosts"),
    ("missing-review", '        review_card(target, result)',
     '        pass  # planted omitted review',
     "Client.test_collection_finishes_fetch_and_review_before_reporting_once"),
    ("hidden-slower", 'if r["verdict"] not in ("Faster", "Level")',
     'if r["verdict"] not in ("Faster", "Level", "Slower")',
     "Client.test_review_preserves_slower_and_no_verdict_beside_runner_verified"),
    ("all-ref-preparation", '["git", "-C", d, "fetch", "-q", cache, r["commit"]]',
     '["git", "-C", d, "fetch", "-q", cache, "+refs/*:refs/cache/*"]',
     "Server.test_exact_commit_preparation_keeps_history_without_importing_other_refs"),
    ("unneeded-remote-refresh", '            steps += [["sh", "-c",',
     '            else:\n                steps.append(["git", "-C", cache, "fetch", "-q", "--prune", "origin"])\n            steps += [["sh", "-c",',
     "Server.test_known_commit_preparation_works_offline_and_uses_a_fresh_checkout"),
    ("unchecked-head", '            if head != r["commit"]:',
     '            if False:  # planted wrong HEAD accepted',
     "Server.test_successful_git_commands_with_wrong_head_fail_before_the_job"),
    ("unchecked-host-key", '"StrictHostKeyChecking=yes"',
     '"StrictHostKeyChecking=no"',
     "Client.test_transport_checks_the_pinned_host_and_uses_a_fresh_connection_namespace"),
    ("changed-host-key", '            if key not in pins:',
     '            if False:  # planted changed host accepted',
     "Client.test_host_enrollment_is_idempotent_and_refuses_a_changed_key"),
    ("ignored-transport-failure", '        transport_failed = r.returncode == 255',
     '        transport_failed = False',
     "Client.test_transport_failure_stops_even_an_optional_request"),
    ("implicit-auth-identity", '"IdentitiesOnly=yes"',
     '"IdentitiesOnly=no"',
     "Client.test_transport_checks_the_pinned_host_and_uses_a_fresh_connection_namespace"),
    ("cancel-signals-a-finished-run", '    if not finish(run, "CANCELLED", [f"cancelled: {reason}"]):\n        return',
     '    finish(run, "CANCELLED", [f"cancelled: {reason}"])',
     "Ownership.test_cancelling_a_finished_run_signals_nothing"),
    ("trusts-a-stale-group-id", '            fcntl.flock(fd, fcntl.LOCK_SH | fcntl.LOCK_NB)\n            return False',
     '            return True  # planted: any group at the number',
     "Ownership.test_a_stale_group_id_is_not_the_runs_executor"),
    ("terminal-state-overwritten", '        os.link(tmp, f"{run}/STATUS")',
     '        os.replace(tmp, f"{run}/STATUS"); open(tmp, "w").close()',
     "Ownership.test_the_first_terminal_state_stands"),
    ("job-in-a-session-of-its-own", "p = subprocess.Popen(cmd, cwd=run, env=env, stdout=out, stderr=subprocess.STDOUT)",
     "p = subprocess.Popen(cmd, cwd=run, env=env, stdout=out, stderr=subprocess.STDOUT, start_new_session=True)",
     "Local.test_cancelling_a_local_run_stops_its_whole_group"),
    ("ignored-deadline", "        if deadline is not None and time.monotonic() - started > deadline:",
     "        if False:  # planted ignored deadline",
     "Bounds.test_a_job_past_its_deadline_is_stopped_with_everything_it_started"),
    ("ignored-memory-floor", "        elif floor is not None and mem_available_mib() < floor:",
     "        elif False:  # planted ignored floor",
     "Bounds.test_a_job_under_the_memory_floor_is_stopped_with_everything_it_started"),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, PINNED_RUN_TOOL=str(HERE / "pinned-run"))
    suite = subprocess.run([sys.executable, str(HERE / "test_pinned_run.py"), "-v"], env=env,
                           text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    args.output.joinpath("tests.log").write_text(suite.stdout)
    assert suite.returncode == 0, suite.stdout
    print("runner suite passed", flush=True)
    for name, old, new, test in FAULTS:
        with tempfile.TemporaryDirectory(prefix="pinned-run-control-") as temp:
            tool = Path(temp) / "pinned-run"
            original = HERE.joinpath("pinned-run").read_text()
            assert original.count(old) == 1, f"{name}: exactly one fault site required"
            changed = original.replace(old, new)
            tool.write_text(changed)
            diff = "".join(difflib.unified_diff(original.splitlines(True), changed.splitlines(True),
                                             fromfile="pinned-run", tofile=name))
            selection = test if "." in test else "Server." + test
            result = subprocess.run([sys.executable, str(HERE / "test_pinned_run.py"), selection],
                                    env=dict(env, PINNED_RUN_TOOL=str(tool)), text=True,
                                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            args.output.joinpath(name + ".log").write_text(diff + result.stdout)
            assert result.returncode == 1 and "FAIL: " + selection.split(".")[-1] in result.stdout, result.stdout
            print("control rejected:", name, flush=True)
    print("pinned-run phase checks passed", flush=True)


if __name__ == "__main__":
    main()
