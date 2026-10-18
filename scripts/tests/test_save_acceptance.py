"""Exercise the paired consumer with explicitly synthetic captures, never Mp3tag proof."""
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location("save_acceptance", Path(__file__).parents[1] / "save-acceptance.py")
harness = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(harness)


class CarrierIdentity(unittest.TestCase):
    def test_retained_audio_matches_the_frozen_manifest(self):
        directory = harness.REPO / "crates/tag-core/tests/fixtures/save-corpus"
        manifest = json.loads((directory / "manifest.json").read_text())
        self.assertEqual(set(manifest["sha256"]), {"tone.mp3", "tone.flac", "tone.m4a"})
        for name, digest in manifest["sha256"].items():
            self.assertEqual(harness.sha256((directory / name).read_bytes()), digest, name)


class SaveAcceptance(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.executable = harness.backend()

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "paired"
        harness.prepare(self.root, self.executable)
        self.assertTrue(harness.run(self.root, self.executable)["passed"])
        self.contract = json.loads((self.root / "contract.json").read_text())
        self.identity_path = self.root / "synthetic-consumer-control.json"
        harness.write_json(self.identity_path, {
            "application": "Mp3tag", "version": "3.36.1", "executable_sha256": harness.REFERENCE_SHA256,
            "platform": "SYNTHETIC CONTROL; original was not executed", "runtime": "TagGroove copies used to test consumer only",
            "recorded_at_utc": "2026-10-18T00:00:00Z", "defaults": {"synthetic": True},
            "evidence": ["test-only input/output copies; no reference evidence"],
            "input_sha256": {name: data["input_sha256"] for name, data in self.contract["baseline"].items()},
        })

    def synthetic_reference(self):
        for operation in harness.OPERATIONS:
            for stage in ("saved", "undone", "redone"):
                shutil.copytree(self.root / "taggroove" / operation / stage, self.root / "reference" / operation / stage)

    def change_unprojected_bytes(self, path):
        before = harness.normalize(harness.invoke(self.executable, "observe", path))
        data = bytearray(path.read_bytes())
        if path.suffix == ".flac":
            at = 4
            while True:
                kind = data[at] & 0x7f
                length = int.from_bytes(data[at + 1:at + 4], "big")
                if kind == 1 and length:
                    data[at + 4] ^= 1
                    break
                self.assertFalse(data[at] & 0x80, "fixture must retain a padding block")
                at += 4 + length
        else:
            self.assertEqual(path.suffix, ".mp3")
            self.assertEqual(data[3], 3)
            frames = []
            at = 10
            while data[at] != 0:
                length = 10 + int.from_bytes(data[at + 4:at + 8], "big")
                frames.append(bytes(data[at:at + length]))
                at += length
            candidates = [i for i, frame in enumerate(frames) if frame[:1] == b"T" and frame[:4] != b"TXXX"]
            first, second = candidates[:2]
            self.assertNotEqual(frames[first][:4], frames[second][:4])
            frames[first], frames[second] = frames[second], frames[first]
            data[10:at] = b"".join(frames)
        path.write_bytes(data)
        self.assertEqual(before, harness.normalize(harness.invoke(self.executable, "observe", path)),
                         "control must change bytes outside the observer's projection")

    def test_compare_rejects_changed_backend_before_observing_captures(self):
        self.synthetic_reference()
        changed = Path(self.temporary.name) / "changed-backend"
        data = bytearray(self.executable.read_bytes())
        data[-1] ^= 1
        changed.write_bytes(data)
        with mock.patch.object(harness, "invoke", side_effect=AssertionError("changed backend must never execute")):
            with self.assertRaisesRegex(ValueError, "identity.*backend_sha256"):
                harness.compare(self.root, changed, self.identity_path)

    def test_compare_rejects_an_actual_different_repository_identity(self):
        self.synthetic_reference()
        repository = Path(self.temporary.name) / "different-repository"
        repository.mkdir()
        original = harness.REPO
        for relative in harness.identity(self.executable)["source_files_sha256"]:
            destination = repository / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(original / relative, destination)
        subprocess.run(["git", "init", "-q", str(repository)], check=True)
        subprocess.run(["git", "-C", str(repository), "add", "."], check=True)
        subprocess.run(["git", "-C", str(repository), "-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false",
                        "-c", "user.name=Acceptance control", "-c", "user.email=control@example.invalid",
                        "commit", "-qm", "Independent identity negative control"], check=True)
        with mock.patch.object(harness, "REPO", repository):
            with self.assertRaisesRegex(ValueError, "identity.*source_head"):
                harness.compare(self.root, self.executable, self.identity_path)

    def test_operator_corpus_run_rejects_unprojected_undo_and_redo_changes(self):
        for stage, extension in [("undone", "flac"), ("redone", "mp3")]:
            with self.subTest(stage=stage):
                root = Path(self.temporary.name) / f"operator-{stage}"
                harness.prepare(root, self.executable, self.root / "inputs")
                invoke = harness.invoke
                def corrupt(executable, *args):
                    result = invoke(executable, *args)
                    if args[0] == "exercise" and args[2] == "edit-one" and Path(args[1]).suffix == f".{extension}":
                        path = Path(args[3]) / stage / Path(args[1]).name
                        with mock.patch.object(harness, "invoke", invoke):
                            self.change_unprojected_bytes(path)
                    return result
                with mock.patch.object(harness, "invoke", corrupt):
                    self.assertFalse(harness.run(root, self.executable)["passed"])

    def test_compare_rejects_unprojected_retained_stage_changes(self):
        self.synthetic_reference()
        for stage, extension in [("undone", "flac"), ("redone", "mp3")]:
            with self.subTest(stage=stage):
                path = self.root / "taggroove" / "edit-one" / stage / f"café 日本.{extension}"
                original = path.read_bytes()
                self.change_unprojected_bytes(path)
                with self.assertRaisesRegex(ValueError, "retained local.*bytes"):
                    harness.compare(self.root, self.executable, self.identity_path)
                path.write_bytes(original)
        saved = self.root / "taggroove" / "edit-one" / "saved" / "café 日本.flac"
        redone = self.root / "taggroove" / "edit-one" / "redone" / "café 日本.flac"
        self.change_unprojected_bytes(saved)
        shutil.copyfile(saved, redone)
        with self.assertRaisesRegex(ValueError, "retained local capture bytes changed"):
            harness.compare(self.root, self.executable, self.identity_path)

    def test_consumer_rejects_missing_capture_and_mismatched_reference(self):
        with self.assertRaisesRegex(ValueError, "missing original-runtime capture"):
            harness.compare(self.root, self.executable, self.identity_path)
        self.assertFalse((self.root / "paired-results.json").exists())
        identity = json.loads(self.identity_path.read_text())
        identity["executable_sha256"] = "0" * 64
        harness.write_json(self.identity_path, identity)
        with self.assertRaisesRegex(ValueError, "pinned original"):
            harness.compare(self.root, self.executable, self.identity_path)
        contract = copy.deepcopy(self.contract)
        contract["baseline"] = {}
        with self.assertRaisesRegex(ValueError, "nonempty captured input"):
            harness.validate_inputs(self.root, self.executable, contract)

    def test_consumer_reads_real_retained_files_and_detects_audio_damage(self):
        self.synthetic_reference()
        self.assertTrue(harness.compare(self.root, self.executable, self.identity_path)["passed"])
        paired = json.loads((self.root / "paired-results.json").read_text())
        local = json.loads((self.root / "taggroove-results.json").read_text())
        self.assertEqual(paired["producer"], local["producer"])
        self.assertEqual(paired["prepared_by"], self.contract["producer"])
        path = self.root / "reference" / "edit-one" / "saved" / "café 日本.flac"
        data = bytearray(path.read_bytes())
        data[-1] ^= 1
        path.write_bytes(data)
        self.assertFalse(harness.compare(self.root, self.executable, self.identity_path)["passed"])
        receipt = json.loads((self.root / "paired-results.json").read_text())
        self.assertIn("/audio_payload_sha256", receipt["cases"][0]["differences_from_taggroove"])

    def test_baseline_and_local_receipt_cannot_mask_changed_files(self):
        self.synthetic_reference()
        contract = copy.deepcopy(self.contract)
        name = next(iter(contract["baseline"]))
        contract["baseline"][name]["snapshot"]["fields"]["TITLE"] = ["fabricated"]
        harness.write_json(self.root / "contract.json", contract)
        with self.assertRaisesRegex(ValueError, "baseline does not match"):
            harness.compare(self.root, self.executable, self.identity_path)
        harness.write_json(self.root / "contract.json", self.contract)
        path = self.root / "taggroove" / "edit-one" / "saved" / "café 日本.flac"
        data = bytearray(path.read_bytes())
        data[-1] ^= 1
        path.write_bytes(data)
        with self.assertRaisesRegex(ValueError, "retained local"):
            harness.compare(self.root, self.executable, self.identity_path)

    def test_captured_inputs_are_copied_and_originals_remain_unchanged(self):
        source = self.root / "inputs"
        hashes = {p.name: harness.sha256(p.read_bytes()) for p in source.iterdir()}
        imported = Path(self.temporary.name) / "captured"
        harness.prepare(imported, self.executable, source)
        self.assertEqual(hashes, {p.name: harness.sha256(p.read_bytes()) for p in source.iterdir()})
        self.assertTrue(harness.run(imported, self.executable)["passed"])

    def test_comparator_detects_each_preservation_dimension(self):
        baseline = next(iter(self.contract["baseline"].values()))["snapshot"]
        for key in ("fields", "covers", "opaque_native", "opaque_marker_count", "audio_payload_sha256", "tag_kinds"):
            damaged = copy.deepcopy(baseline)
            damaged[key] = None
            self.assertTrue(harness.differences(baseline, damaged), key)


if __name__ == "__main__":
    unittest.main()
