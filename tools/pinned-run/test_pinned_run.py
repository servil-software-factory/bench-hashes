"""Tests of pinned-run's requests and its time savers (superseding, host choice, next), the server run
in-process against a temporary runs directory, no SSH and no systemd."""
import base64, fcntl, glob, importlib.machinery, importlib.util, io, json, os, shutil, sys, tarfile, tempfile, unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
TOOL = os.environ.get("PINNED_RUN_TOOL", os.path.join(HERE, "pinned-run"))


def load(root):
    os.environ["PINNED_RUN_ROOT"] = root
    os.environ["PINNED_RUN_INSTALLED"] = os.path.join(root, "installed-pinned-run")
    loader = importlib.machinery.SourceFileLoader("pinned_run_under_test", TOOL)
    spec = importlib.util.spec_from_loader(loader.name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    mod.SYSTEMCTL = ["true"]
    return mod


class Server(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp(prefix="pinned-run-test-")
        self.pr = load(self.root)
        self.started = []

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def manifest(self, name, commit="a" * 40, lock=None):
        script = b"echo hi\n"
        m = {"name": name, "host": "root@h", "repos": [{"url": "https://x/r", "commit": commit, "path": "src"}],
             "script": "job.sh", "script_sha256": self.pr.sha256(script), "forbid": []}
        if lock:
            m["lock"] = lock
        return {"manifest": m, "script": base64.b64encode(script).decode()}

    def submit(self, name, n, **kw):
        run_id = f"{name}-20261105T00000{n}-0000000{n}"
        reply = self.pr.serve_submit(run_id, self.manifest(name, **kw), start=lambda r, t: self.started.append(r))
        return run_id, reply

    def serve(self, verb, *args, stdin=b""):
        out = io.BytesIO()
        self.pr.serve(verb, list(args), stdin=io.BytesIO(stdin), stdout=out)
        return out.getvalue()

    def phases(self, run_id):
        with open(os.path.join(self.root, run_id, "phases.jsonl")) as f:
            return [json.loads(line) for line in f]

    def execute_fake(self, run_id, *, job_code=0, verification=()):
        import subprocess
        def git(args, **kw):
            return subprocess.CompletedProcess(args, 0, stdout="a" * 40 if "rev-parse" in args else "")
        with mock.patch.object(self.pr, "sh", git), \
             mock.patch.object(self.pr, "run_job", return_value=job_code), \
             mock.patch.object(self.pr, "verify", return_value=list(verification)):
            self.pr.remote_execute(os.path.join(self.root, run_id))

    def test_acceptance_is_recorded_before_starting_the_executor(self):
        run_id = "accepted-20261105T000001-00000001"
        observed = []
        self.pr.serve_submit(run_id, self.manifest("accepted"),
                             start=lambda r, _: observed.extend(self.phases(r)))
        self.assertEqual([x["phase"] for x in observed], ["accepted"])

    def test_phases_cover_success_with_monotonic_and_wall_clock_identities(self):
        lock = os.path.join(self.root, "bench.lock")
        with mock.patch.object(self.pr.time, "monotonic_ns", side_effect=range(100, 107)), \
             mock.patch.object(self.pr.time, "time_ns", side_effect=range(200, 193, -1)):
            run_id, _ = self.submit("phases", 1, lock=lock)
            self.execute_fake(run_id)
        rows = self.phases(run_id)
        self.assertEqual([x["phase"] for x in rows], ["accepted", "executor_started", "lock_acquired",
                         "prepared", "job_finished", "verification_finished", "finished"])
        self.assertEqual([x["monotonic_ns"] for x in rows], list(range(100, 107)))
        self.assertEqual([x["utc_ns"] for x in rows], list(range(200, 193, -1)))
        self.assertTrue(all(x["version"] == 1 and x["boot_id"] == rows[0]["boot_id"] for x in rows))
        self.assertTrue(rows[0]["boot_id"])
        self.assertTrue(rows[3]["ok"])
        self.assertEqual(rows[4]["exit_code"], 0)
        self.assertTrue(rows[5]["ok"])
        self.assertEqual(rows[-1]["status"], "VERIFIED")

    def test_lock_acquired_event_is_emitted_under_the_lock(self):
        lock = os.path.join(self.root, "bench.lock")
        run_id, _ = self.submit("phase-lock", 1, lock=lock)
        original = self.pr.record_phase
        observed = []
        def record(run, phase, **details):
            if phase == "lock_acquired":
                with open(lock, "a") as other:
                    with self.assertRaises(BlockingIOError):
                        fcntl.flock(other, fcntl.LOCK_EX | fcntl.LOCK_NB)
                observed.append(phase)
            original(run, phase, **details)
        with mock.patch.object(self.pr, "record_phase", record):
            self.execute_fake(run_id)
        self.assertEqual(observed, ["lock_acquired"])

    def test_no_lock_has_no_lock_acquired_event(self):
        run_id, _ = self.submit("no-lock", 1)
        self.execute_fake(run_id)
        self.assertEqual([x["phase"] for x in self.phases(run_id)], ["accepted", "executor_started",
                         "prepared", "job_finished", "verification_finished", "finished"])

    def test_failed_job_and_failed_verification_are_distinct(self):
        for n, (code, failures) in enumerate([(7, []), (0, ["missing identity"])], 1):
            with self.subTest(code=code, failures=failures):
                run_id, _ = self.submit("failed-stage", n)
                self.execute_fake(run_id, job_code=code, verification=failures)
                rows = {r["phase"]: r for r in self.phases(run_id)}
                self.assertEqual(rows["job_finished"]["exit_code"], code)
                self.assertEqual(rows["verification_finished"]["ok"], not failures)
                self.assertEqual(rows["finished"]["status"], "FAILED")

    def test_terminal_records_survive_an_executor_restart(self):
        for n, code in enumerate([0, 7], 1):
            with self.subTest(code=code):
                run_id, _ = self.submit("terminal", n)
                self.execute_fake(run_id, job_code=code)
                run = os.path.join(self.root, run_id)
                paths = [f"{run}/{name}" for name in ["STATUS", "REASONS", "phases.jsonl"]]
                before = {}
                for path in paths:
                    with open(path, "rb") as f:
                        before[path] = f.read()
                with mock.patch.object(self.pr, "record_phase") as record, mock.patch.object(self.pr, "sh") as prepare:
                    self.pr.remote_execute(run)
                    record.assert_not_called()
                    prepare.assert_not_called()
                for path in paths:
                    with open(path, "rb") as f:
                        self.assertEqual(f.read(), before[path], path)

    def test_a_run_cancelled_before_it_starts_executes_nothing(self):
        run_id, _ = self.submit("early-cancel", 1)
        run = os.path.join(self.root, run_id)
        self.serve("cancel", run_id)
        before = self.phases(run_id)
        with mock.patch.object(self.pr, "sh") as prepare, mock.patch.object(self.pr, "run_job") as job:
            self.pr.remote_execute(run)
            prepare.assert_not_called()
            job.assert_not_called()
        self.assertEqual(self.phases(run_id), before)
        self.assertFalse(os.path.exists(f"{run}/STARTED"))

    def test_interrupted_executor_fails_without_fabricating_completed_phases(self):
        run_id, _ = self.submit("restarted", 1)
        run = os.path.join(self.root, run_id)
        with open(f"{run}/STARTED", "w"):
            pass
        before = self.phases(run_id)
        self.pr.remote_execute(run)
        self.assertEqual(self.phases(run_id), before)
        with open(f"{run}/STATUS") as f:
            self.assertEqual(f.read().strip(), "FAILED")

    def test_a_run_launched_again_supersedes_its_queued_namesake(self):
        lock = os.path.join(self.root, "bench.lock")
        a, _ = self.submit("check-epyc", 1, lock=lock)
        b, reply = self.submit("check-epyc", 2, commit="b" * 40, lock=lock)
        self.assertEqual(self.serve("status", a).decode().splitlines(), ["CANCELLED", f"cancelled: superseded by {b}"])
        self.assertEqual(self.serve("status", b).decode().strip(), "queued")
        self.assertTrue(any("superseded" in n for n in reply["notes"]), reply)
        self.assertEqual(self.started, [a, b])

    def test_a_running_namesake_is_left_alone_and_other_names_too(self):
        lock = os.path.join(self.root, "bench.lock")
        a, _ = self.submit("check-epyc", 1, lock=lock)
        open(os.path.join(self.root, a, "LOCKED"), "w").close()  # it holds the lock: running
        c, _ = self.submit("probe-epyc", 3, lock=lock)
        b, reply = self.submit("check-epyc", 2, lock=lock)
        self.assertEqual(self.serve("status", a).decode().strip(), "running")
        self.assertEqual(self.serve("status", c).decode().strip(), "queued", "another name is never superseded")
        self.assertIn(f"2 run(s) ahead on {lock}: {a}, {c}", reply["notes"])

    def test_a_run_whose_unit_is_gone_is_lost_not_queued(self):
        lock = os.path.join(self.root, "bench.lock")
        a, _ = self.submit("x", 1, lock=lock)
        self.pr.SYSTEMCTL = ["false"]  # no unit active
        self.assertEqual(self.serve("status", a).decode().strip(), "lost")
        self.assertEqual(json.loads(self.serve("lock", lock))["queued"], 0, "a lost run waits for nothing")
        b, reply = self.submit("y", 2, lock=lock)
        self.assertFalse(any("ahead" in n for n in reply["notes"]), reply)

    def test_list_and_cancel(self):
        a, _ = self.submit("x", 1)
        rows = [json.loads(l) for l in self.serve("list").decode().splitlines()]
        self.assertEqual([(r["run"], r["state"]) for r in rows], [(a, "queued")])
        self.assertEqual(self.serve("cancel", a).decode().strip(), "CANCELLED")
        self.assertEqual(self.serve("status", a).decode().splitlines()[0], "CANCELLED")

    def test_the_lock_request_tells_free_from_held(self):
        lock = os.path.join(self.root, "bench.lock")
        self.submit("x", 1, lock=lock)
        self.assertEqual(json.loads(self.serve("lock", lock)), {"free": True, "queued": 1})
        with open(lock, "a") as held:
            fcntl.flock(held, fcntl.LOCK_EX)
            # Another open file description of the same file: held.
            pid = os.fork()
            if pid == 0:
                os._exit(0 if not json.loads(self.serve("lock", lock))["free"] else 1)
            self.assertEqual(os.waitpid(pid, 0)[1], 0, "held while another process holds it")

    def test_preparation_and_job_both_hold_the_measurement_lock(self):
        lock = os.path.join(self.root, "bench.lock")
        run_id, _ = self.submit("prepare-lock", 1, lock=lock)
        run = os.path.join(self.root, run_id)
        events = []
        def assert_locked(phase):
            self.assertTrue(os.path.exists(f"{run}/LOCKED"), phase)
            with open(lock, "a") as other:
                with self.assertRaises(BlockingIOError, msg=phase):
                    fcntl.flock(other, fcntl.LOCK_EX | fcntl.LOCK_NB)
            events.append(phase)
        def git(args, **kw):
            assert_locked("prepare")
            return __import__("subprocess").CompletedProcess(args, 0, stdout="a"*40 if "rev-parse" in args else "")
        def job(*args, **kw):
            assert_locked("job")
            return 0
        def verify(*args):
            assert_locked("verify")
            return []
        with mock.patch.object(self.pr, "sh", git), mock.patch.object(self.pr, "run_job", job), mock.patch.object(self.pr, "verify", verify):
            self.pr.remote_execute(run)
        self.assertEqual(events[-2:], ["job", "verify"])
        self.assertIn("prepare", events)
        self.assertEqual(open(f"{run}/STATUS").read().strip(), "VERIFIED")
        with open(lock, "a") as other:
            fcntl.flock(other, fcntl.LOCK_EX | fcntl.LOCK_NB)  # released afterwards

    def test_failed_preparation_releases_the_measurement_lock(self):
        lock = os.path.join(self.root, "bench.lock")
        run_id, _ = self.submit("prepare-fail", 1, lock=lock)
        run = os.path.join(self.root, run_id)
        def fail(args, **kw):
            self.assertTrue(os.path.exists(f"{run}/LOCKED"))
            return __import__("subprocess").CompletedProcess(args, 1, stdout="")
        with mock.patch.object(self.pr, "sh", fail), mock.patch.object(self.pr.subprocess, "run") as job:
            self.pr.remote_execute(run)
            job.assert_not_called()
        self.assertEqual(open(f"{run}/STATUS").read().strip(), "FAILED")
        rows = self.phases(run_id)
        self.assertEqual([x["phase"] for x in rows], ["accepted", "executor_started", "lock_acquired", "prepared", "finished"])
        self.assertFalse(rows[-2]["ok"])
        self.assertEqual(rows[-1]["status"], "FAILED")
        with open(lock, "a") as other:
            fcntl.flock(other, fcntl.LOCK_EX | fcntl.LOCK_NB)

    def git(self, repo, *args, check=True):
        import subprocess
        return subprocess.run(["git", "-C", repo, "-c", "core.hooksPath=/dev/null", "-c", "user.name=Runner Test",
                               "-c", "user.email=runner@example.invalid", "-c", "commit.gpgsign=false", *args],
                              capture_output=True, text=True, check=check)

    def real_repo(self):
        repo = os.path.join(self.root, "upstream")
        os.mkdir(repo)
        self.git(repo, "init", "-q", "--initial-branch=main")
        first = self.real_commit(repo, "first")
        return repo, first

    def real_commit(self, repo, value):
        with open(os.path.join(repo, "value"), "w") as f:
            f.write(value)
        self.git(repo, "add", "value")
        self.git(repo, "commit", "-qm", value)
        return self.git(repo, "rev-parse", "HEAD").stdout.strip()

    def prepare_real(self, repo, commit, n):
        payload = self.manifest("real", commit=commit, lock=os.path.join(self.root, "bench.lock"))
        payload["manifest"]["repos"][0]["url"] = repo
        run_id = f"real-20261105T00000{n}-0000000{n}"
        self.pr.serve_submit(run_id, payload, start=lambda *_: None)
        run = os.path.join(self.root, run_id)
        self.pr.remote_execute(run)
        return run

    def test_exact_commit_preparation_keeps_history_without_importing_other_refs(self):
        repo, first = self.real_repo()
        wanted = self.real_commit(repo, "wanted")
        self.real_commit(repo, "later")
        run = self.prepare_real(repo, wanted, 1)
        with open(run + "/STATUS") as f:
            self.assertEqual(f.read().strip(), "VERIFIED")
        self.assertEqual(self.git(run + "/src", "rev-parse", "HEAD").stdout.strip(), wanted)
        self.assertEqual(self.git(run + "/src", "rev-list", "--count", "HEAD").stdout.strip(), "2")
        self.git(run + "/src", "cat-file", "-e", first + "^{commit}")
        self.assertEqual(self.git(run + "/src", "for-each-ref", "--format=%(refname)", "refs/cache").stdout, "",
                         "only the requested history is fetched, not the mirror's unrelated refs")

    def test_known_commit_preparation_works_offline_and_uses_a_fresh_checkout(self):
        repo, wanted = self.real_repo()
        first = self.prepare_real(repo, wanted, 1)
        with open(first + "/src/stale-untracked", "w") as f:
            f.write("must stay in the first checkout")
        os.rename(repo, repo + "-offline")
        second = self.prepare_real(repo, wanted, 2)
        with open(second + "/STATUS") as f:
            self.assertEqual(f.read().strip(), "VERIFIED", "a known immutable commit needs no remote refresh")
        self.assertFalse(os.path.exists(second + "/src/stale-untracked"))
        self.assertNotEqual(os.stat(first + "/src/value").st_ino, os.stat(second + "/src/value").st_ino)
        self.assertEqual(self.git(second + "/src", "rev-parse", "HEAD").stdout.strip(), wanted)

    def test_unknown_commit_is_fetched_by_hash_or_fails_before_the_job(self):
        repo, first = self.real_repo()
        self.prepare_real(repo, first, 1)
        wanted = self.real_commit(repo, "new upstream commit")
        second = self.prepare_real(repo, wanted, 2)
        with open(second + "/STATUS") as f:
            self.assertEqual(f.read().strip(), "VERIFIED")
        self.assertEqual(self.git(second + "/src", "rev-parse", "HEAD").stdout.strip(), wanted)
        failed = self.prepare_real(repo, "f" * 40, 3)
        with open(failed + "/STATUS") as f:
            self.assertEqual(f.read().strip(), "FAILED")
        with open(failed + "/phases.jsonl") as f:
            self.assertNotIn("job_finished", [json.loads(l)["phase"] for l in f])

    def test_successful_git_commands_with_wrong_head_fail_before_the_job(self):
        import subprocess
        run_id, _ = self.submit("wrong-head", 1)
        def git(args, **kw):
            return subprocess.CompletedProcess(args, 0, stdout="b" * 40 if "rev-parse" in args else "")
        with mock.patch.object(self.pr, "sh", git), \
             mock.patch.object(self.pr.subprocess, "run", return_value=subprocess.CompletedProcess([], 0)) as job:
            self.pr.remote_execute(os.path.join(self.root, run_id))
            job.assert_not_called()
        with open(os.path.join(self.root, run_id, "STATUS")) as f:
            self.assertEqual(f.read().strip(), "FAILED")

    def test_fetch_streams_the_records_and_outputs_alone(self):
        a, _ = self.submit("x", 1)
        run = os.path.join(self.root, a)
        os.makedirs(f"{run}/out/sub")
        for p, t in ((f"{run}/STATUS", "VERIFIED\n"), (f"{run}/out/sub/r.txt", "result\n"), (f"{run}/src-secret", "no\n")):
            open(p, "w").write(t)
        with tarfile.open(fileobj=io.BytesIO(self.serve("fetch", a))) as t:
            names = sorted(t.getnames())
            with open(f"{run}/phases.jsonl", "rb") as original:
                self.assertEqual(t.extractfile("phases.jsonl").read(), original.read())
        self.assertIn("phases.jsonl", names)
        self.assertIn("out/sub/r.txt", names)
        self.assertIn("STATUS", names)
        self.assertNotIn("src-secret", names)

    def test_install_refuses_a_file_that_does_not_parse(self):
        with self.assertRaises(SyntaxError):
            self.serve("install", stdin=b"def broken(:\n")
        self.assertFalse(os.path.exists(self.pr.INSTALLED))
        good = b"print('ok')\n"
        self.assertEqual(self.serve("install", stdin=good).decode().strip(), self.pr.sha256(good))

    def test_a_bad_run_id_and_unknown_requests_are_refused(self):
        for verb, args in (("status", ["../../etc"]), ("no-such", []), ("status", [])):
            with self.assertRaises(SystemExit):
                self.serve(verb, *args)


class Forced(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp(prefix="pinned-run-test-")
        self.pr = load(self.root)

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def run_forced(self, command):
        env = dict(os.environ, SSH_ORIGINAL_COMMAND=command, PINNED_RUN_ROOT=self.root)
        import subprocess
        return subprocess.run([sys.executable, TOOL, "serve-forced"], env=env, capture_output=True, text=True)

    def test_the_forced_command_runs_requests_and_nothing_else(self):
        for command in ("bash -i", "cargo build", "bench-hashes check --fork .", "pinned-run remote-execute /root/x",
                        "pinned-run serve", "", "pinned-run serve version; id"):
            r = self.run_forced(command)
            self.assertNotEqual(r.returncode, 0, command)
            self.assertNotIn(self.pr.sha256(open(TOOL, "rb").read()), r.stdout, command)
        r = self.run_forced("pinned-run serve version")
        self.assertEqual((r.returncode, r.stdout.strip()), (0, self.pr.sha256(open(TOOL, "rb").read())))
        r = self.run_forced("rm -rf /")
        self.assertIn("this key runs pinned-run requests only", r.stderr)

    def test_remote_execute_is_no_request(self):
        r = self.run_forced("pinned-run serve remote-execute /root/pinned-runs/x-20261105T000001-00000001")
        self.assertNotEqual(r.returncode, 0)


class Client(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp(prefix="pinned-run-test-")
        self.pr = load(self.root)
        self.pr.REGISTRY = os.path.join(self.root, "launched.jsonl")
        self.pr.REPORTED = os.path.join(self.root, "reported.txt")
        self.pr.ensure_installed = lambda host: None  # the fake transports answer requests, not installs

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def fanout_manifests(self, *, different=None):
        script = os.path.join(self.root, "instrument.sh")
        with open(script, "w") as f:
            f.write("echo exact instrument\n")
        paths = []
        for i in range(2):
            m = {"name": f"cell-{i}", "host": f"root@{i}", "lock": "/root/bench.lock",
                 "script": script, "repos": [{"url": "https://x/r", "commit": "a" * 40, "path": "src"}],
                 "env": {"MODE": "same"}, "expect": [{"file": "out/result", "contains": ["result"]}],
                 "forbid": ["wrong input"]}
            if i and different:
                m.update(different)
            path = os.path.join(self.root, f"manifest-{i}.json")
            with open(path, "w") as f:
                json.dump(m, f)
            paths.append(path)
        return paths

    def test_fanout_waits_for_verified_preflight_and_freezes_its_inputs(self):
        paths = self.fanout_manifests()
        submitted = []
        states = iter(["running", "VERIFIED"])
        def submit(m, script):
            submitted.append((m, script))
            if len(submitted) == 1:
                self.assertEqual(m["host"], "root@fast")
                self.assertEqual(m["env"]["PINNED_RUN_PREFLIGHT"], "1")
                self.assertEqual(m["expect"], [{"file": "out/preflight.txt", "contains": ["pinned-run preflight complete"]}])
                # A local edit after preflight must not change the later submitted inputs.
                with open(m["script"], "w") as f:
                    f.write("echo changed after capture\n")
                with open(paths[1], "w") as f:
                    json.dump({"broken": True}, f)
            return {"run": "run-" + str(len(submitted)), "host": m["host"]}
        def status(host, run):
            self.assertEqual(len(submitted), 1, "no job launches before preflight verification")
            return next(states)
        with mock.patch.object(self.pr, "submit_manifest", submit), mock.patch.object(self.pr, "status", status):
            result = self.pr.fanout("root@fast", paths, poll=0)
        self.assertEqual([x["host"] for x in result], ["root@0", "root@1"])
        self.assertEqual(len(submitted), 3)
        self.assertTrue(all(script == b"echo exact instrument\n" for _, script in submitted))
        self.assertTrue(all("PINNED_RUN_PREFLIGHT" not in m["env"] for m, _ in submitted[1:]))
        self.assertTrue(all(m["expect"] == [{"file": "out/result", "contains": ["result"]}] for m, _ in submitted[1:]))

    def test_failed_preflight_launches_no_fanout_jobs(self):
        paths = self.fanout_manifests()
        for state in ["FAILED", "CANCELLED", "lost"]:
            with self.subTest(state=state), mock.patch.object(self.pr, "submit_manifest", return_value={"host": "root@fast", "run": "preflight"}) as submit, \
                 mock.patch.object(self.pr, "status", return_value=state), self.assertRaises(SystemExit):
                self.pr.fanout("root@fast", paths, poll=0)
            self.assertEqual(submit.call_count, 1, "only preflight was submitted")

    def test_fanout_refuses_different_instrument_inputs_before_any_submit(self):
        changes = [
            {"repos": [{"url": "https://x/r", "commit": "b" * 40, "path": "src"}]},
            {"env": {"MODE": "different"}},
            {"forbid": ["different rules"]},
        ]
        for change in changes:
            with self.subTest(change=change):
                paths = self.fanout_manifests(different=change)
                with mock.patch.object(self.pr, "submit_manifest", return_value={"host": "root@fast", "run": "preflight"}) as submit, \
                     mock.patch.object(self.pr, "status", return_value="VERIFIED"), self.assertRaises(SystemExit):
                    self.pr.fanout("root@fast", paths, poll=0)
                submit.assert_not_called()

    def test_fanout_refuses_different_script_bytes(self):
        other = os.path.join(self.root, "other.sh")
        with open(other, "w") as f:
            f.write("echo other\n")
        paths = self.fanout_manifests(different={"script": other})
        with mock.patch.object(self.pr, "submit_manifest") as submit, self.assertRaises(SystemExit):
            self.pr.fanout("root@fast", paths, poll=0)
        submit.assert_not_called()

    def test_fanout_requires_a_lock_and_reserves_its_mode_variable(self):
        for change in [{"lock": None}, {"env": {"PINNED_RUN_PREFLIGHT": "0"}}]:
            paths = self.fanout_manifests(different=change)
            with mock.patch.object(self.pr, "submit_manifest") as submit, self.assertRaises(SystemExit):
                self.pr.fanout("root@fast", paths, poll=0)
            submit.assert_not_called()

    def test_preflight_missing_marker_fails_output_verification(self):
        run = os.path.join(self.root, "marker")
        os.mkdir(run)
        m = {"expect": [{"file": "out/preflight.txt", "contains": ["pinned-run preflight complete"]}], "forbid": []}
        self.assertEqual(self.pr.verify(run, m), ["expect: no file out/preflight.txt"])

    def test_submit_manifest_keeps_the_capture_unchanged(self):
        path = self.fanout_manifests()[0]
        with open(path) as f:
            m = json.load(f)
        before = json.loads(json.dumps(m))
        with mock.patch.object(self.pr, "call", return_value=b'{"notes": []}'):
            result = self.pr.submit_manifest(m, b"echo exact instrument\n")
        self.assertEqual(m, before)
        self.assertEqual(result["host"], "root@0")
        self.assertTrue(result["run"].startswith("cell-0-"))

    def test_transport_checks_the_pinned_host_and_uses_a_fresh_connection_namespace(self):
        import subprocess
        self.pr.KNOWN_HOSTS = self.root + "/known_hosts"
        with mock.patch.object(self.pr.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, stdout=b"ok")) as run:
            self.assertEqual(self.pr.call("root@ours", "version"), b"ok")
        args = run.call_args.args[0]
        self.assertIn("StrictHostKeyChecking=yes", args)
        self.assertIn("IdentitiesOnly=yes", args)
        self.assertIn("HostKeyAlgorithms=ssh-ed25519", args)
        self.assertIn("UserKnownHostsFile=" + self.pr.KNOWN_HOSTS, args)
        self.assertIn("ControlPath=" + os.path.expanduser("~/.ssh/pinned-%C"), args)
        self.assertNotIn("StrictHostKeyChecking=no", args)
        self.assertNotIn("UserKnownHostsFile=/dev/null", args)

    def test_transport_failure_stops_even_an_optional_request(self):
        import subprocess
        failure = subprocess.CompletedProcess([], 255, stdout=b"", stderr=b"Host key verification failed")
        for check in [True, False]:
            with self.subTest(check=check), mock.patch.object(self.pr.subprocess, "run", return_value=failure), \
                 self.assertRaises(SystemExit):
                self.pr.call("root@ours", "version", check=check)

    def test_host_enrollment_is_idempotent_and_refuses_a_changed_key(self):
        import subprocess
        self.pr.KNOWN_HOSTS = self.root + "/ssh/known_hosts"
        keys = []
        for i in range(2):
            key = self.root + f"/key-{i}"
            subprocess.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", key], check=True)
            keys.append(key + ".pub")
        self.pr.trust("root@192.0.2.1", keys[0])
        with open(self.pr.KNOWN_HOSTS) as f:
            original = f.read()
        self.pr.trust("root@192.0.2.1", keys[0])
        with self.assertRaises(SystemExit):
            self.pr.trust("root@192.0.2.1", keys[1])
        with open(self.pr.KNOWN_HOSTS) as f:
            self.assertEqual(f.read(), original)
        self.pr.trust("root@192.0.2.2", keys[1])
        with open(self.pr.KNOWN_HOSTS) as f:
            self.assertEqual(len(f.read().splitlines()), 2)

    def test_study_is_validated_and_preserved_in_the_registry(self):
        path = self.fanout_manifests()[0]
        with open(path) as f:
            m = json.load(f)
        m["study"] = "fast-feedback-147"
        with mock.patch.object(self.pr, "call", return_value=b'{"notes": []}'):
            self.pr.submit_manifest(m, b"echo exact instrument\n")
        self.assertEqual(self.pr.registered()[0]["study"], "fast-feedback-147")
        m["study"] = "two words"
        with self.assertRaises(SystemExit):
            self.pr.validate(m)

    def test_scoped_next_ignores_other_studies_and_contacts_only_its_hosts(self):
        runs = [{"run": "unrelated", "host": "root@other", "study": "other"},
                {"run": "ours", "host": "root@ours", "study": "pilot"},
                {"run": "legacy", "host": "root@old"}]
        with open(self.pr.REGISTRY, "w") as f:
            f.write("".join(json.dumps(r) + "\n" for r in runs))
        def call(host, verb, *args, **kw):
            self.assertEqual(host, "root@ours", "scope filtering happens before any remote request")
            return b'{"run":"ours","state":"VERIFIED"}\n' if verb == "list" else b"VERIFIED\n"
        with mock.patch.object(self.pr, "call", call), mock.patch("sys.stdout", new_callable=io.StringIO):
            self.assertEqual(self.pr.next_done(study="pilot", poll=0), "ours")
            with self.assertRaises(SystemExit) as error:
                self.pr.next_done(study="pilot", poll=0)
            self.assertEqual(error.exception.code, 3)
            self.assertEqual(self.pr.pending(study="pilot"), [])

    def review_fixture(self):
        target = os.path.join(self.root, "review-fixture")
        os.makedirs(target + "/out", exist_ok=True)
        m = {"host": "root@ours", "study": "pilot", "lock": "/root/bench.lock",
             "repos": [{"url": "https://x/r", "commit": "a" * 40, "path": "src"}],
             "script_sha256": "b" * 64, "tool_sha256": "c" * 64}
        with open(target + "/manifest.json", "w") as f:
            json.dump(m, f)
        names = ["accepted", "executor_started", "lock_acquired", "prepared", "job_finished", "verification_finished", "finished"]
        with open(target + "/phases.jsonl", "w") as f:
            for i, name in enumerate(names):
                f.write(json.dumps({"version": 1, "phase": name, "monotonic_ns": i * 10,
                                    "utc_ns": 1000 - i * 20, "boot_id": "same-boot"}) + "\n")
        with open(target + "/out/judgments.tsv", "w") as f:
            f.write("shape\tmetric\tratio_permille\tverdict\n")
            f.write("tiny\tinstructions\t1040\tSlower\n")
            f.write("other\twall_ns\t980\tLevel\n")
            f.write("busy\tcycles\t1000\tNo verdict\n")
        return target

    def test_review_preserves_slower_and_no_verdict_beside_runner_verified(self):
        target = self.review_fixture()
        summary = self.pr.review_card(target, "VERIFIED")
        self.assertEqual(summary["runner_result"], "VERIFIED")
        self.assertEqual(summary["durations_ns"], {"dispatch": 10, "lock_wait": 10, "preparation": 10,
                                                 "job": 10, "verification": 10, "total": 60})
        judged = summary["judgments"]["out/judgments.tsv"]
        self.assertEqual(judged["counts"], {"Level": 1, "No verdict": 1, "Slower": 1})
        self.assertEqual(judged["attention"][0]["ratio_permille"], "1040")
        self.assertEqual([r["verdict"] for r in judged["attention"]], ["Slower", "No verdict"])
        with open(target + "/REVIEW.md") as f:
            text = f.read()
        self.assertIn("1 Slower", text)
        self.assertIn('"ratio_permille": "1040"', text)
        with open(target + "/review.json") as f:
            self.assertEqual(json.load(f), summary)

    def test_review_requires_current_single_boot_monotonic_phase_records(self):
        for field, value in [("version", 2), ("boot_id", "another-boot"), ("monotonic_ns", -1), ("phase", "accepted")]:
            target = self.review_fixture()
            path = target + "/phases.jsonl"
            with open(path) as f:
                rows = [json.loads(l) for l in f]
            rows[1][field] = value
            with open(path, "w") as f:
                f.write("".join(json.dumps(r) + "\n" for r in rows))
            with self.subTest(field=field), self.assertRaises(AssertionError):
                self.pr.review_card(target, "VERIFIED")

    def test_review_keeps_a_failure_without_judgments(self):
        target = self.review_fixture()
        os.remove(target + "/out/judgments.tsv")
        summary = self.pr.review_card(target, "FAILED\njob: exit 101")
        self.assertEqual(summary["runner_result"], "FAILED\njob: exit 101")
        self.assertEqual(summary["judgments"], {})
        with open(target + "/REVIEW.md") as f:
            self.assertIn("job: exit 101", f.read())

    def test_collection_finishes_fetch_and_review_before_reporting_once(self):
        with open(self.pr.REGISTRY, "w") as f:
            f.write(json.dumps({"run": "ours", "host": "root@ours", "study": "pilot"}) + "\n")
        events = []
        def fetch(*args, **kwargs):
            self.assertFalse(os.path.exists(self.pr.REPORTED))
            self.assertEqual(kwargs, {"failed": True}, "failed artifacts stay collectable")
            events.append("fetch")
            return self.root + "/FAILED-ours"
        def review(target, result):
            self.assertFalse(os.path.exists(self.pr.REPORTED))
            self.assertEqual(result, "FAILED\njob: exit 101")
            self.assertEqual(target, self.root + "/FAILED-ours")
            events.append("review")
        def call(host, verb, *args, **kw):
            return b'{"run":"ours","state":"FAILED"}\n' if verb == "list" else b"FAILED\njob: exit 101"
        with mock.patch.object(self.pr, "call", call), mock.patch.object(self.pr, "fetch", fetch), \
             mock.patch.object(self.pr, "review_card", review):
            self.assertEqual(self.pr.next_done(study="pilot", dest=self.root, poll=0), "ours")
        self.assertEqual(events, ["fetch", "review"])
        self.assertEqual(self.pr.reported(), {"ours"})

    def test_failed_collection_is_retryable(self):
        for stage in ["fetch", "review_card"]:
            with mock.patch.object(self.pr, "status", return_value="VERIFIED"), \
                 mock.patch.object(self.pr, "fetch", return_value=self.root) as fetch, \
                 mock.patch.object(self.pr, "review_card") as review:
                (fetch if stage == "fetch" else review).side_effect = OSError("collection interrupted")
                with self.assertRaises(OSError):
                    self.pr.report("root@ours", "ours", dest=self.root)
                self.assertFalse(os.path.exists(self.pr.REPORTED), "an incomplete collection remains unreported")

    def test_malformed_judgments_stop_review(self):
        target = self.review_fixture()
        with open(target + "/out/judgments.tsv", "w") as f:
            f.write("shape\tverdict\ncell\tLevel\textra\n")
        with self.assertRaisesRegex(AssertionError, "malformed judgments"):
            self.pr.review_card(target, "VERIFIED")

    def test_next_help_and_collection_scope_contract(self):
        import subprocess
        for args, code in [(["next", "--help"], 0), (["next", "--fetch", self.root], 2)]:
            result = subprocess.run([sys.executable, TOOL, *args], capture_output=True, text=True)
            self.assertEqual(result.returncode, code)
            self.assertIn("--study", result.stdout + result.stderr)

    def test_wait_polls_once_a_second_and_preserves_terminal_result(self):
        for pending in ([], ["queued", "running"]):
            for result in ("VERIFIED", "FAILED\njob: exit 101", "CANCELLED", "lost"):
                with self.subTest(pending=pending, result=result), \
                     mock.patch.object(self.pr, "status", side_effect=pending + [result]) as status, \
                     mock.patch.object(self.pr.time, "sleep") as sleep, \
                     mock.patch.object(sys, "argv", [TOOL, "wait", "root@ours", "run"]), \
                     mock.patch("sys.stdout", new_callable=io.StringIO) as out:
                    self.pr.main()
                    self.assertEqual(sleep.call_args_list, [mock.call(1)] * len(pending))
                    self.assertEqual(status.call_args_list, [mock.call("root@ours", "run")] * (len(pending) + 1))
                    self.assertEqual(out.getvalue(), result + "\n")

    def test_a_host_list_takes_a_free_machine_else_the_shortest_queue(self):
        states = {"root@a": {"free": False, "queued": 2}, "root@b": {"free": True, "queued": 0}, "root@c": {"free": False, "queued": 0}}
        with mock.patch.object(self.pr, "call", lambda h, verb, *a, **k: json.dumps(states[h]).encode()):
            self.assertEqual(self.pr.choose_host({"host": ["root@a", "root@b", "root@c"], "lock": "/l"}), "root@b")
            states["root@b"] = {"free": False, "queued": 1}
            self.assertEqual(self.pr.choose_host({"host": ["root@a", "root@b", "root@c"], "lock": "/l"}), "root@c")
            self.assertEqual(self.pr.choose_host({"host": "root@a", "lock": "/l"}), "root@a")

    def test_next_returns_the_first_pending_run_to_finish(self):
        with open(self.pr.REGISTRY, "w") as f:
            for r in ("one-20261105T000001-00000001", "two-20261105T000002-00000002", "old-20261105T000003-00000003"):
                f.write(json.dumps({"run": r, "host": "root@a"}) + "\n")
        polls = {"n": 0}
        def fake(host, verb, *args, **kw):
            if verb == "list":
                return b"".join(json.dumps({"run": r, "state": s}).encode() + b"\n" for r, s in (
                    ("one-20261105T000001-00000001", "running"), ("two-20261105T000002-00000002", "queued"), ("old-20261105T000003-00000003", "VERIFIED")))
            polls["n"] += 1
            done = args[0].startswith("two") and polls["n"] > 4
            return b"VERIFIED\n" if done else b"running\n"
        with open(self.pr.REPORTED, "w") as f:
            f.write("old-20261105T000003-00000003\n")  # reported already
        with mock.patch.object(self.pr, "call", fake), mock.patch("sys.stdout", new_callable=io.StringIO) as out:
            self.assertEqual(self.pr.next_done(poll=0), "two-20261105T000002-00000002")
        self.assertIn("two-20261105T000002-00000002 root@a\nVERIFIED", out.getvalue())
        os.remove(self.pr.REPORTED)
        # The run that finished before this call was never reported: it comes first, once.
        with mock.patch.object(self.pr, "call", fake), mock.patch("sys.stdout", new_callable=io.StringIO):
            self.assertEqual(self.pr.next_done(poll=0), "old-20261105T000003-00000003")
        with mock.patch.object(self.pr, "call", lambda h, verb, *a, **k: b""):
            with self.assertRaises(SystemExit) as e:
                self.pr.next_done(poll=0)
            self.assertEqual(e.exception.code, 3, "none pending or unreported")



def sentinel():
    """A sleeping process in a group of its own, standing for some other program's: a test that
    signals nothing of its own leaves it alive. The caller reaps it (reap)."""
    import subprocess
    return subprocess.Popen(["sleep", "60"], start_new_session=True)


def reap(p):
    import signal
    try:
        os.killpg(p.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    p.wait()


class Ownership(unittest.TestCase):
    """A process-supervised run's group is signalled only while it is that run's live executor
    (Sam's review of #152: a finished record, a group ID reused after a crash)."""
    setUp_server, tearDown, manifest, submit, serve = Server.setUp, Server.tearDown, Server.manifest, Server.submit, Server.serve

    def setUp(self):
        self.setUp_server()
        self.pr.SUPERVISOR = "process"
        self.other = sentinel()
        self.addCleanup(reap, self.other)
        self.run_id, _ = self.submit("owned", 1)
        self.run = os.path.join(self.root, self.run_id)
        with open(f"{self.run}/PGID", "w") as f:
            f.write(f"{self.other.pid}\n")
        open(f"{self.run}/ALIVE", "w").close()

    def hold_alive(self):
        """Hold the run's ALIVE lock, as its live executor would."""
        fd = os.open(f"{self.run}/ALIVE", os.O_RDWR)
        fcntl.flock(fd, fcntl.LOCK_EX)
        self.addCleanup(os.close, fd)

    def assertAlive(self, p):
        """`p` still runs half a second on: a SIGKILL sent to it lands well within that."""
        import subprocess
        with self.assertRaises(subprocess.TimeoutExpired, msg="a group that is not the run's was signalled"):
            p.wait(timeout=0.5)

    def test_cancelling_a_finished_run_signals_nothing(self):
        self.hold_alive()  # even with the lock held by someone, a finished run is left alone
        self.assertTrue(self.pr.finish(self.run, "VERIFIED", []))
        self.serve("cancel", self.run_id)
        self.assertAlive(self.other)
        self.assertEqual(self.serve("status", self.run_id).decode(), "VERIFIED\n")

    def test_a_stale_group_id_is_not_the_runs_executor(self):
        self.assertEqual(self.serve("status", self.run_id).decode().strip(), "lost")
        self.serve("cancel", self.run_id)
        self.assertAlive(self.other)
        self.assertEqual(self.serve("status", self.run_id).decode().splitlines()[0], "CANCELLED")

    def test_a_live_executor_is_its_group(self):
        self.hold_alive()
        self.assertEqual(self.serve("status", self.run_id).decode().strip(), "queued")
        self.serve("cancel", self.run_id)
        self.assertEqual(self.other.wait(timeout=10), -9)

    def test_the_first_terminal_state_stands(self):
        self.assertTrue(self.pr.finish(self.run, "CANCELLED", ["cancelled: by request"]))
        self.assertFalse(self.pr.finish(self.run, "VERIFIED", []))
        self.assertEqual(self.serve("status", self.run_id).decode(), "CANCELLED\ncancelled: by request\n")


class Local(unittest.TestCase):
    """The local host end to end, no mocks: a run on this machine through the command line,
    its executor a process group of its own (a sandbox: no root, no systemd, no SSH). Self-
    contained: an ordinary temporary directory is its HOME, and every executor it started is
    stopped before the directory goes, whatever the test's outcome."""

    def setUp(self):
        import subprocess
        self.sp = subprocess
        self.home = tempfile.mkdtemp(prefix="pinned-run-local-")
        self.addCleanup(shutil.rmtree, self.home, True)
        self.addCleanup(self.stop_executors)
        self.env = dict(os.environ, HOME=self.home)
        self.env.pop("PINNED_RUN_ROOT", None)
        self.env.pop("PINNED_RUN_INSTALLED", None)
        repo = os.path.join(self.home, "repo")
        git = lambda *a: subprocess.run(["git", "-C", repo, *a], check=True, capture_output=True, env=dict(
            self.env, GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@t.invalid", GIT_COMMITTER_NAME="t",
            GIT_COMMITTER_EMAIL="t@t.invalid"))
        os.makedirs(repo)
        git("init", "-q")
        with open(os.path.join(repo, "f"), "w") as f:
            f.write("x")
        git("add", "f")
        git("commit", "-q", "-m", "one")
        self.repo, self.commit = repo, git("rev-parse", "HEAD").stdout.decode().strip()

    def stop_executors(self):
        """Kill each run's executor group that still holds its ALIVE lock, and wait for the lock."""
        import signal, time
        for alive in glob.glob(os.path.join(self.home, "pinned-runs", "*", "ALIVE")):
            run = os.path.dirname(alive)
            fd = os.open(alive, os.O_RDONLY)
            try:
                try:
                    fcntl.flock(fd, fcntl.LOCK_SH | fcntl.LOCK_NB)
                    continue
                except BlockingIOError:
                    pass
                for _ in range(200):  # the executor publishes its group as it starts
                    if os.path.exists(os.path.join(run, "PGID")):
                        break
                    time.sleep(0.05)
                try:
                    os.killpg(int(open(os.path.join(run, "PGID")).read()), signal.SIGKILL)
                except (OSError, ValueError):
                    pass
                fcntl.flock(fd, fcntl.LOCK_SH)  # released as the executor dies
            finally:
                os.close(fd)

    def reap_group_of(self, pid):
        """Kill the group of `pid`, a process this test's job started seconds ago, should a
        fault have let it escape its run's group (and outlive the cancel under test)."""
        import signal
        try:
            group = os.getpgid(pid)
            if group != os.getpgrp():
                os.killpg(group, signal.SIGKILL)
        except ProcessLookupError:
            pass

    def tool(self, *args, timeout=60):
        return self.sp.run([sys.executable, TOOL, *args], env=self.env, capture_output=True, text=True,
                           timeout=timeout, cwd=self.home)

    def launch(self, name, script):
        with open(os.path.join(self.home, "job.sh"), "w") as f:
            f.write(script)
        with open(os.path.join(self.home, "m.json"), "w") as f:
            json.dump({"name": name, "host": "local", "script": "job.sh",
                       "repos": [{"url": "file://" + self.repo, "commit": self.commit, "path": "src"}],
                       "expect": [{"file": "out/x.txt", "contains": ["ok"]}]}, f)
        r = self.tool("launch", "m.json")
        self.assertEqual(r.returncode, 0, r.stderr)
        return r.stdout.strip()

    def test_a_local_run_is_verified_from_its_pinned_inputs(self):
        run_id = self.launch("local-ok", 'cat "$1/src/f" > /dev/null && echo ok > "$1/out/x.txt" && ps -o pgid= -p $$ > "$1/out/pgid"\n')
        r = self.tool("wait", "local", run_id)
        self.assertIn("VERIFIED", r.stdout + r.stderr)
        run = os.path.join(self.home, "pinned-runs", run_id)
        phases = [json.loads(l)["phase"] for l in open(os.path.join(run, "phases.jsonl"))]
        self.assertEqual(phases[0], "accepted")
        self.assertEqual(phases[-1], "finished")
        self.assertEqual(int(open(os.path.join(run, "out", "pgid")).read()), int(open(os.path.join(run, "PGID")).read()),
                         "the job runs in its executor's group")

    def test_cancelling_a_local_run_stops_its_whole_group(self):
        import time
        run_id = self.launch("local-cancel", 'sleep 30 & echo $! > "$1/child.pid"; wait\n')
        run = os.path.join(self.home, "pinned-runs", run_id)
        for _ in range(200):
            if os.path.exists(os.path.join(run, "child.pid")):
                break
            time.sleep(0.05)
        child = int(open(os.path.join(run, "child.pid")).read())
        self.addCleanup(self.reap_group_of, child)
        self.assertEqual(self.tool("cancel", "local", run_id).returncode, 0)
        for _ in range(200):
            try:
                os.kill(child, 0)
            except ProcessLookupError:
                break
            time.sleep(0.05)
        else:
            self.fail("the job's child outlived its cancelled run")
        self.assertIn("CANCELLED", self.tool("status", "local", run_id).stdout)


class Bounds(unittest.TestCase):
    """A job's limits: its deadline and the machine's memory floor fail the run and stop its
    whole group, the executor with everything it started. Each job runs under a stand-in
    executor, a process leading its own group, since a limit kills the caller's group."""

    def setUp(self):
        import subprocess
        self.sp = subprocess
        self.root = tempfile.mkdtemp(prefix="pinned-run-bounds-")
        self.addCleanup(shutil.rmtree, self.root, True)

    def job(self, m, memory=None):
        """A job that starts a child sleeping 5 s and waits for it, under a stand-in executor
        (a job its limit leaves alone ends on its own in 5 s, so a control that drops the limit
        fails fast): the executor's exit status and the child's pid."""
        program = f"""
import importlib.machinery, importlib.util, json, os, sys
loader = importlib.machinery.SourceFileLoader("pr", {TOOL!r})
pr = importlib.util.module_from_spec(importlib.util.spec_from_loader("pr", loader)); loader.exec_module(pr)
if {memory!r} is not None:
    pr.mem_available_mib = lambda: {memory!r}
with open("job.log", "w") as out:
    print(pr.run_job(["bash", "-c", "sleep 5 & echo $! > child.pid; wait"], {self.root!r}, dict(os.environ), out,
                     json.loads({json.dumps(m)!r}), poll_s=0.05))
"""
        p = self.sp.run([sys.executable, "-c", program], cwd=self.root, start_new_session=True, timeout=60,
                        capture_output=True, text=True, env=dict(os.environ, PINNED_RUN_ROOT=self.root))
        with open(os.path.join(self.root, "child.pid")) as f:
            child = int(f.read())
        return p, child

    def status(self):
        return open(os.path.join(self.root, "STATUS")).read() + open(os.path.join(self.root, "REASONS")).read()

    def assertGone(self, pid):
        import time
        for _ in range(100):
            try:
                os.kill(pid, 0)
            except ProcessLookupError:
                return
            time.sleep(0.02)
        self.fail(f"process {pid} outlived its stopped job")

    def test_a_job_past_its_deadline_is_stopped_with_everything_it_started(self):
        p, child = self.job({"deadline_s": 1})
        self.assertGone(child)
        self.assertEqual(p.returncode, -9, p.stdout + p.stderr)
        self.assertEqual(self.status(), "FAILED\njob: stopped at its deadline, 1 s\n")

    def test_a_job_under_the_memory_floor_is_stopped_with_everything_it_started(self):
        p, child = self.job({"memory_floor_mib": 1536}, memory=100)
        self.assertGone(child)
        self.assertEqual(p.returncode, -9, p.stdout + p.stderr)
        self.assertIn("memory fell under 1536 MiB", self.status())

    def test_a_job_within_its_limits_ends_on_its_own(self):
        with open(os.path.join(self.root, "job.log"), "w") as out:
            code = load(self.root).run_job(["bash", "-c", "exit 3"], self.root, dict(os.environ), out,
                                           {"deadline_s": 60, "memory_floor_mib": 1}, poll_s=0.05)
        self.assertEqual(code, 3)
        self.assertFalse(os.path.exists(os.path.join(self.root, "STATUS")))

    def test_limits_are_whole_numbers_above_zero(self):
        pr = load(self.root)
        base = {"name": "n", "host": "local", "repos": [], "script": "j.sh"}
        for bad in ({"deadline_s": 0}, {"deadline_s": 1.5}, {"memory_floor_mib": -1}, {"memory_floor_mib": "1"}):
            with self.assertRaises(SystemExit):
                pr.validate(dict(base, **bad))
        pr.validate(dict(base, deadline_s=1, memory_floor_mib=1))


if __name__ == "__main__":
    unittest.main()
