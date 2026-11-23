"""python3 tools/test_dependency_policy.py: which packages without a source the dependency policy takes for
ours (tools/dependency_policy.py, `strangers`), on accounts of the graph written here."""
import os
import sys
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dependency_policy as policy

ROOT = Path("/repo")
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"


def package(name, folder=None, source=None):
    place = ROOT / folder if folder else ROOT / "registry" / name
    return {"id": f"{name}#1.0.0", "name": name, "version": "1.0.0", "source": source,
            "manifest_path": str(place / "Cargo.toml")}


OURS = [package("bench-hashes", "."), package("clocks", "clocks")]
OUR_IDS = [p["id"] for p in OURS]


class Strangers(unittest.TestCase):
    def test_the_two_crates_and_registry_packages_are_no_strangers(self):
        self.assertEqual(policy.strangers({"packages": OURS + [package("libc", source=REGISTRY)], "workspace_members": OUR_IDS}, root=ROOT), [])

    def test_a_vendored_path_crate_is_named_with_its_folder(self):
        wrong = policy.strangers({"packages": OURS + [package("copied", "third-party/copied")], "workspace_members": OUR_IDS}, root=ROOT)
        self.assertEqual(len(wrong), 1)
        self.assertIn("copied 1.0.0 is taken by path, from third-party/copied/", wrong[0])

    def test_an_implicit_third_workspace_member_is_caught(self):
        extra = package("copied", "copied")
        wrong = policy.strangers({"packages": OURS + [extra], "workspace_members": OUR_IDS + [extra["id"]]}, root=ROOT)
        self.assertTrue(any("Cargo workspace_members are" in w for w in wrong), wrong)
        self.assertTrue(any("copied 1.0.0 is taken by path" in w for w in wrong), wrong)

    def test_an_expected_name_in_another_folder_is_a_stranger(self):
        wrong = policy.strangers({"packages": [OURS[0], package("clocks", "copy/clocks")], "workspace_members": OUR_IDS}, root=ROOT)
        self.assertEqual(len(wrong), 2)
        self.assertIn("clocks 1.0.0 is taken by path, from copy/clocks/", wrong[0])
        self.assertIn("expected crate clocks at clocks/Cargo.toml is absent", wrong[1])

    def test_another_name_in_an_expected_folder_is_a_stranger(self):
        other = package("other", "clocks")
        wrong = policy.strangers({"packages": [OURS[0], other], "workspace_members": [OURS[0]["id"], other["id"]]}, root=ROOT)
        self.assertTrue(any("other 1.0.0 is taken by path, from clocks/" in w for w in wrong), wrong)
        self.assertTrue(any("expected crate clocks at clocks/Cargo.toml is absent" in w for w in wrong), wrong)

    def test_a_missing_crate_is_said(self):
        wrong = policy.strangers({"packages": [OURS[0]], "workspace_members": [OURS[0]["id"]]}, root=ROOT)
        self.assertTrue(any("expected crate clocks at clocks/Cargo.toml is absent" in w for w in wrong), wrong)


if __name__ == "__main__":
    unittest.main()
