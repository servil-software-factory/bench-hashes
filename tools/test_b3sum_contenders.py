"""Fixed command/artifact contracts for the shared relinker and existing CLI."""
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("b3sum_contenders", Path(__file__).with_name("b3sum-contenders.py"))
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)  # Importing a helper performs no builds.
HOST = "x86_64-unknown-linux-gnu"


class RelinkTests(unittest.TestCase):
    def test_outboard_forms_keep_hash_only_baselines_and_thread_budgets(self):
        self.assertEqual(module.servil_forms(["b3sum=/srv/b3sum"], True, True), [
            "b3sum=/srv/b3sum",
            "b3sum-1-thread=/srv/b3sum --num-threads 1",
            "b3sum-outboard=/srv/b3sum --outboard {OUT}",
            "b3sum-outboard-1-thread=/srv/b3sum --num-threads 1 --outboard {OUT}",
        ])
        self.assertEqual(module.servil_forms(["b3sum-abcdef0=/srv/b3sum"], False, True), [
            "b3sum-abcdef0=/srv/b3sum",
            "b3sum-abcdef0-outboard=/srv/b3sum --outboard {OUT}",
        ])
        self.assertEqual(module.servil_forms(["b3sum=/srv/b3sum"], True, False), [
            "b3sum=/srv/b3sum",
            "b3sum-1-thread=/srv/b3sum --num-threads 1",
        ])

    def test_default_and_other_binary_keep_the_same_layout_protocol(self):
        for binary in ["b3sum", "bao-reference"]:
            with self.subTest(binary=binary), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                target = root / "target"
                (target / "lld").mkdir(parents=True)
                (target / "lld/ld.lld").touch()
                built = root / "fixture"
                built.write_bytes(b"original")
                calls = []

                def build(*args, env):
                    calls.append((args, env))
                    self.assertEqual(args[args.index("--bin") + 1], binary)
                    self.assertEqual(args[args.index("--target") + 1], HOST)
                    self.assertEqual(env["RUSTFLAGS"], "-C target-feature=+crt-static")
                    self.assertIn("--locked", args)
                    path = Path(env["CARGO_TARGET_DIR"]) / HOST / "release" / binary
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(b"linked " + binary.encode())
                    path.chmod(0o755)

                symbols = "\n".join(f"1 T symbol_{i}" for i in range(8)) + "\n2 r data_only\n"
                options = dict(target=str(target), host=HOST, layouts=2)
                if binary != "b3sum":
                    options["binary"] = binary
                with patch.object(module, "out", return_value=symbols), patch.object(module, "run", side_effect=build):
                    result = module.relink("contender", "Cargo.toml", str(built), "-C target-feature=+crt-static", **options)
                    self.assertEqual(result, [f"contender={built}-L0", f"contender={built}-L1"])
                    self.assertEqual(len(calls), 2)
                    orders = [(target / f"order-fixture-{i}").read_text().splitlines() for i in range(2)]
                    for order in orders:
                        self.assertEqual(sorted(order), [f"symbol_{i}" for i in range(8)])
                    self.assertNotEqual(orders[0], orders[1])
                    for i in range(2):
                        self.assertEqual(Path(f"{built}-L{i}").read_bytes(), b"linked " + binary.encode())
                    self.assertEqual(module.relink("contender", "Cargo.toml", str(built), "-C target-feature=+crt-static", **options), result)
                    self.assertEqual(len(calls), 2, "retained layouts are reused as before")

    def test_existing_cli_retains_its_names_flags_and_artifacts(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            dest = root / "bins"
            cargo_home = root / "cargo"
            official_src = cargo_home / "registry/src/test/b3sum-1.8.7"
            official_src.mkdir(parents=True)
            target = root / "builds/b3sum-contenders"

            def out(*args, **kwargs):
                if args[0] == "rustc":
                    return "rustc test\nhost: " + HOST
                if args[-1] == "--show-toplevel":
                    return str(root / "fork")
                self.assertEqual(args[-2:], ("--short=7", "HEAD^{commit}"))
                return "abcdef0"

            def run(*args, env=None):
                if args[:2] == ("cargo", "install"):
                    path = Path(args[args.index("--root") + 1]) / "bin/b3sum"
                elif args[:2] == ("cargo", "build"):
                    self.assertEqual(env["RUSTFLAGS"], "-C target-feature=+crt-static")
                    path = Path(env["CARGO_TARGET_DIR"]) / HOST / "release/b3sum"
                else:
                    self.assertEqual(args[0], "git")
                    Path(args[-2]).mkdir(parents=True)
                    return
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"program")
                path.chmod(0o755)

            with patch.dict(os.environ, {"LAYOUTS": "1", "CARGO_HOME": str(cargo_home), "CARGO_TARGET_DIR": str(root / "builds")}), patch.object(module.sys, "argv", ["b3sum-contenders.py", str(root / "fork"), str(dest)]), patch.object(module, "out", side_effect=out), patch.object(module, "run", side_effect=run), patch.object(module.shutil, "which", return_value=None):
                stdout = io.StringIO()
                with contextlib.redirect_stdout(stdout):
                    module.main()
            self.assertEqual(stdout.getvalue().splitlines(), [
                f"official={dest}/b3sum-official-1.8.7",
                f"official-1-thread={dest}/b3sum-official-1.8.7 --num-threads 1",
                f"b3sum={dest}/b3sum-abcdef0",
                f"b3sum-1-thread={dest}/b3sum-abcdef0 --num-threads 1",
            ])
            self.assertTrue((target / "worktree-abcdef0").is_dir())


if __name__ == "__main__":
    unittest.main()
