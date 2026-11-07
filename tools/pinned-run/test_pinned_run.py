"""Tests of pinned-run's requests and its time savers (superseding, host choice, next), the server run
in-process against a temporary runs directory, no SSH and no systemd."""
import base64, fcntl, importlib.machinery, importlib.util, io, json, os, shutil, sys, tarfile, tempfile, unittest
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
        self.root = tempfile.mkdtemp(prefix="pinned-run-test-", dir=os.path.expanduser("~/work"))
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
             mock.patch.object(self.pr.subprocess, "run", return_value=subprocess.CompletedProcess([], job_code)), \
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
        def job(args, **kw):
            assert_locked("job")
            return __import__("subprocess").CompletedProcess(args, 0)
        def verify(*args):
            assert_locked("verify")
            return []
        with mock.patch.object(self.pr, "sh", git), mock.patch.object(self.pr.subprocess, "run", job), mock.patch.object(self.pr, "verify", verify):
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
        self.root = tempfile.mkdtemp(prefix="pinned-run-test-", dir=os.path.expanduser("~/work"))
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
        self.root = tempfile.mkdtemp(prefix="pinned-run-test-", dir=os.path.expanduser("~/work"))
        self.pr = load(self.root)
        self.pr.REGISTRY = os.path.join(self.root, "launched.jsonl")
        self.pr.REPORTED = os.path.join(self.root, "reported.txt")
        self.pr.ensure_installed = lambda host: None  # the fake transports answer requests, not installs

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

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


if __name__ == "__main__":
    unittest.main()
