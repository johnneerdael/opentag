"""Synthetic integrity/containment controls; no Mp3tag or Windows execution."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location("reference_collector", Path(__file__).parents[1] / "reference-capture/collector.py")
collector = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(collector)


class ReferenceCapture(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def artifact(self, name, data):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return {"path": name, "sha256": collector.digest(path), "bytes": len(data)}

    def capture(self):
        # This manifest exercises the validator, not the truth of runtime claims.
        input_hash = hashlib.sha256(b"independent test input").hexdigest()
        self.contract = self.root / "contract.json"
        collector.write_json(self.contract, {"schema": 1, "baseline": {"café.flac": {"input_sha256": input_hash}}})
        cfg = self.root / "runtime/application/mp3tag.cfg"
        cfg.parent.mkdir(parents=True)
        cfg.write_bytes(b"synthetic configuration control")
        artifacts = [self.artifact("evidence/control.json", b'{"observation_type":"synthetic unit control"}')]
        cases = []
        for operation in collector.OPERATIONS:
            stages = {}
            for stage in collector.STAGES:
                relative = f"evidence/{operation}/{stage}/café.flac"
                artifacts.append(self.artifact(relative, b"fLaCsynthetic header control"))
                stages[stage] = relative
            cases.append({"operation": operation, "file": "café.flac", "input_sha256": input_hash, "stages": stages,
                "operator_steps": [{"action": action, "observed_at_utc": "2026-10-18T00:00:00Z", "evidence": "evidence/control.json"}
                                   for action in ("edit", "save", "undo", "redo", "reload")]})
        record = {"schema": collector.SCHEMA, "observation_type": "runtime_save_capture", "status": "complete",
            "recorded_at_utc": "2026-10-18T00:00:00Z", "contract_sha256": collector.digest(self.contract),
            "reference": {"application": "Mp3tag", "version": "3.36.1", "installer_url": collector.INSTALLER_URL,
                "installer_sha256": collector.INSTALLER_SHA256, "executable_sha256": collector.EXECUTABLE_SHA256},
            "runtime": {"os": "Windows", "os_version": "SYNTHETIC UNIT CONTROL; Windows was not run", "session_id": 1,
                "process_id": 123, "process_creation_time_utc": "2026-10-18T00:00:00Z", "collector_source_sha256": "a" * 64},
            "configuration": {"mode": "portable", "observed_path": "runtime/application/mp3tag.cfg", "sha256": collector.digest(cfg), "evidence": "evidence/control.json", "observed_at_utc": "2026-10-18T00:00:00Z"},
            "defaults": {**{key: False for key in collector.DEFAULT_SWITCHES}, "id3v2_format": "v23_utf16", "evidence": "evidence/control.json", "observed_at_utc": "2026-10-18T00:00:00Z"},
            "artifacts": artifacts, "cases": cases}
        self.record = record
        self.save()

    def save(self):
        collector.write_json(self.root / "capture.json", self.record)

    def test_actual_pins_and_generic_artifact_verification(self):
        self.assertEqual(collector.INSTALLER_SHA256, "a977861003a87b1f00f2e0c91263c61bf12d457c6d457b367ce5f69de28fbea0")
        self.assertEqual(collector.EXECUTABLE_SHA256, "484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911")
        path = self.root / "owned.data"
        path.write_bytes(b"independent bytes")
        collector.verify_file(path, collector.digest(path), path.stat().st_size)
        with self.assertRaisesRegex(ValueError, "identity mismatch"):
            collector.verify_file(path, "0" * 64, path.stat().st_size)

    def test_wrong_installer_never_reaches_extraction(self):
        path = self.root / "fake.exe"
        path.write_bytes(b"MZ test control")
        with mock.patch.object(collector.subprocess, "run") as execute:
            with self.assertRaisesRegex(ValueError, "identity mismatch"):
                collector.prepare_runtime(self.root / "fresh", path, "7z")
        execute.assert_not_called()

    def test_redirect_to_a_mirror_is_refused(self):
        with self.assertRaisesRegex(ValueError, "unapproved URL"):
            collector.PinnedRedirect().redirect_request(None, None, 302, "", {}, "http://example.invalid/reference.exe")

    def test_wrong_extracted_executable_leaves_no_qualified_runtime(self):
        package = self.root / "synthetic-package"
        package.write_bytes(b"independent archive control")
        def extract(command, **kwargs):
            target = Path(command[3][2:])
            target.mkdir(parents=True)
            (target / "Mp3tag.exe").write_bytes(b"MZ wrong executable control")
        target = self.root / "fresh-extract"
        with mock.patch.object(collector, "INSTALLER_SHA256", collector.digest(package)), mock.patch.object(collector, "INSTALLER_BYTES", package.stat().st_size), mock.patch.object(collector.subprocess, "run", side_effect=extract):
            with self.assertRaisesRegex(ValueError, "identity mismatch"):
                collector.prepare_runtime(target, package, "synthetic-extractor")
        self.assertFalse((target / "runtime.json").exists())
        self.assertFalse((target / "runtime/application/mp3tag.cfg").exists())

    def test_paths_and_runtime_reuse_fail_closed(self):
        for path in ("../capture.json", "evidence/../../runtime/Mp3tag.exe", "C:/runtime/Mp3tag.exe", "evidence\\file.json", "evidence/stream:payload.json", "evidence/trailing ./file.json"):
            with self.subTest(path=path), self.assertRaises(ValueError):
                collector.owned_path(self.root, path)
        with self.assertRaises(FileExistsError):
            collector.new_temporary_workspace(self.root)
        with mock.patch.object(collector.tempfile, "gettempdir", return_value=str(self.root / "other")), mock.patch.dict(collector.os.environ, {}, clear=True):
            with self.assertRaisesRegex(ValueError, "temporary directory"):
                collector.new_temporary_workspace(self.root / "outside")

    def test_static_probe_and_synthetic_records_never_qualify(self):
        self.capture()
        for observation in ("static_runtime_preparation", "runtime_probe", "synthetic_test_control"):
            self.record["observation_type"] = observation
            self.save()
            with self.assertRaisesRegex(ValueError, "cannot qualify"):
                collector.validate_capture(self.root, self.contract)

    def test_valid_manifest_exports_only_json_identity_without_runtime_bodies(self):
        self.capture()
        collector.validate_capture(self.root, self.contract)
        destination = self.root / "identity.json"
        collector.export_identity(self.root, self.contract, destination)
        identity = json.loads(destination.read_text())
        self.assertEqual(identity["executable_sha256"], collector.EXECUTABLE_SHA256)
        self.assertEqual(identity["input_sha256"], {"café.flac": self.record["cases"][0]["input_sha256"]})
        self.assertNotIn("configuration control", destination.read_text())
        self.assertFalse((self.root / "reference").exists())
        with self.assertRaisesRegex(ValueError, "must not exist"):
            collector.export_identity(self.root, self.contract, destination)

    def test_mutated_artifacts_contract_and_configuration_are_rejected(self):
        self.capture()
        for relative in ("evidence/control.json", "evidence/edit-one/saved/café.flac", "runtime/application/mp3tag.cfg", "contract.json"):
            path = self.root / relative
            original = path.read_bytes()
            path.write_bytes(original + b" ")
            with self.subTest(path=relative), self.assertRaises(ValueError):
                collector.validate_capture(self.root, self.contract)
            path.write_bytes(original)

    def test_missing_duplicate_misidentified_and_unobserved_cases_are_rejected(self):
        self.capture()
        mutations = [lambda r: r["cases"].pop(), lambda r: r["cases"].append(copy.deepcopy(r["cases"][0])),
            lambda r: r["cases"][0]["stages"].pop("undone"), lambda r: r["cases"][0].update(input_sha256="0" * 64),
            lambda r: r["cases"][0].update(operator_steps=[]), lambda r: r["defaults"].pop("write_id3v2"),
            lambda r: r["configuration"].update(mode="unverified"), lambda r: r["reference"].update(executable_sha256="0" * 64),
            lambda r: r["cases"][0]["operator_steps"][1].update(observed_at_utc="unknown"),
            lambda r: r["defaults"].update(observed_at_utc="2027-01-01T00:00:00Z")]
        original = copy.deepcopy(self.record)
        for mutate in mutations:
            self.record = copy.deepcopy(original)
            mutate(self.record)
            self.save()
            with self.assertRaises(ValueError):
                collector.validate_capture(self.root, self.contract)

    def test_vendor_files_and_renamed_executables_cannot_enter_artifact_set(self):
        self.capture()
        for relative in ("runtime/application/Mp3tag.exe", "evidence/Mp3tag.exe", "evidence/renamed.mp3"):
            original = list(self.record["artifacts"])
            self.record["artifacts"].append(self.artifact(relative, b"MZ synthetic executable control"))
            self.save()
            with self.subTest(path=relative), self.assertRaises(ValueError):
                collector.validate_capture(self.root, self.contract)
            self.record["artifacts"] = original

    def test_symlinked_artifacts_cannot_escape(self):
        self.capture()
        outside = self.root / "outside.json"
        outside.write_text("{}")
        link = self.root / "evidence/link.json"
        link.symlink_to(outside)
        self.record["artifacts"].append({"path": "evidence/link.json", "sha256": collector.digest(outside), "bytes": 2})
        self.save()
        with self.assertRaisesRegex(ValueError, "escapes"):
            collector.validate_capture(self.root, self.contract)


if __name__ == "__main__":
    unittest.main()
