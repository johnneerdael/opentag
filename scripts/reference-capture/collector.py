#!/usr/bin/env python3
"""Stage a pinned disposable reference runtime and validate supplied captures.

Staging never executes Mp3tag. Windows execution is a separate explicit probe.
No vendor file is an admissible evidence artifact or repository dependency.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path, PureWindowsPath
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
from datetime import datetime, timezone

INSTALLER_URL = "https://download.mp3tag.de/mp3tag-v3.36.1-x64-setup.exe"
INSTALLER_SHA256 = "a977861003a87b1f00f2e0c91263c61bf12d457c6d457b367ce5f69de28fbea0"
INSTALLER_BYTES = 6008656
EXECUTABLE_SHA256 = "484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911"
EXECUTABLE_BYTES = 25073528
SCHEMA = "taggroove.mp3tag-reference/v1"
OPERATIONS = ("edit-one", "edit-many")
STAGES = ("saved", "undone", "redone")
DEFAULT_SWITCHES = tuple(f"{action}_{kind}" for action in ("read", "write", "remove")
                         for kind in ("id3v1", "id3v2", "ape"))


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def verify_file(path, expected_hash, expected_bytes):
    if path.is_symlink() or not path.is_file() or path.stat().st_size != expected_bytes or digest(path) != expected_hash:
        raise ValueError(f"pinned artifact identity mismatch: {path.name}")
    return {"sha256": expected_hash, "bytes": expected_bytes}


def owned_path(root, relative):
    windows = PureWindowsPath(relative)
    parts = relative.split("/")
    if (not relative or windows.is_absolute() or windows.drive or "\\" in relative
            or any(not p or p in (".", "..") or p != p.rstrip(" .") or any(c in p for c in '<>:"|?*') for p in parts)):
        raise ValueError("evidence path must be a plain relative path")
    path = root / relative
    if not path.resolve().is_relative_to(root.resolve()) or path.is_symlink():
        raise ValueError("evidence path escapes the owned workspace")
    return path


def new_temporary_workspace(path):
    root = path.resolve()
    allowed = [Path(tempfile.gettempdir()).resolve()]
    if os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("RUNNER_TEMP"):
        allowed.append(Path(os.environ["RUNNER_TEMP"]).resolve())
    if not any(root != base and root.is_relative_to(base) for base in allowed):
        raise ValueError("runtime workspace must be new and inside an OS/runner temporary directory")
    root.mkdir(parents=True, exist_ok=False)
    return root


class PinnedRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, response, code, message, headers, newurl):
        if newurl != INSTALLER_URL:
            raise ValueError("pinned download redirected to an unapproved URL")
        return super().redirect_request(request, response, code, message, headers, newurl)


def download_installer(path):
    # Retain normal TLS verification; never substitute a mirror or latest URL.
    opener = urllib.request.build_opener(PinnedRedirect())
    with opener.open(INSTALLER_URL, timeout=60) as response, path.open("xb") as output:
        data = response.read(INSTALLER_BYTES + 1)
        output.write(data)
    verify_file(path, INSTALLER_SHA256, INSTALLER_BYTES)


def prepare_runtime(destination, installer=None, seven_zip=None):
    root = new_temporary_workspace(destination)
    download = root / "download"
    download.mkdir()
    package = download / "setup.exe"
    if installer is None:
        download_installer(package)
    else:
        verify_file(installer, INSTALLER_SHA256, INSTALLER_BYTES)
        shutil.copyfile(installer, package)
    verify_file(package, INSTALLER_SHA256, INSTALLER_BYTES)
    extractor = seven_zip or shutil.which("7z") or shutil.which("7zz")
    if not extractor:
        raise ValueError("7-Zip is required; no tool was installed and no installer was executed")
    application = root / "runtime" / "application"
    command = [str(extractor), "x", "-y", f"-o{application}", str(package)]
    subprocess.run(command, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    executable = application / "Mp3tag.exe"
    verify_file(executable, EXECUTABLE_SHA256, EXECUTABLE_BYTES)
    # Presence selects portable mode according to the author, but empty-file
    # initialization and the actual configuration path still require a probe.
    (application / "mp3tag.cfg").open("xb").close()
    for name in ("environment/roaming", "environment/local", "environment/temp", "empty", "evidence"):
        (root / name).mkdir(parents=True)
    record = {"schema": SCHEMA, "observation_type": "static_runtime_preparation", "status": "unexecuted",
        "recorded_at_utc": datetime.now(timezone.utc).isoformat(), "staging_platform": platform.platform(),
        "reference": {"application": "Mp3tag", "version": "3.36.1", "installer_url": INSTALLER_URL,
            "installer": verify_file(package, INSTALLER_SHA256, INSTALLER_BYTES),
            "executable": verify_file(executable, EXECUTABLE_SHA256, EXECUTABLE_BYTES)},
        "extraction_argv": command, "executable": "runtime/application/Mp3tag.exe",
        "configuration": {"candidate": "runtime/application/mp3tag.cfg", "seed": "empty portable candidate",
            "mode_observed": False, "path_observed": False},
        "environment": {"APPDATA": "environment/roaming", "LOCALAPPDATA": "environment/local",
            "TEMP": "environment/temp", "TMP": "environment/temp"},
        "vendor_files_may_be_uploaded": False}
    write_json(root / "runtime.json", record)
    return record


def validate_reference(reference):
    expected = {"application": "Mp3tag", "version": "3.36.1", "executable_sha256": EXECUTABLE_SHA256,
                "installer_sha256": INSTALLER_SHA256, "installer_url": INSTALLER_URL}
    if any(reference.get(key) != value for key, value in expected.items()):
        raise ValueError("reference identity does not match the pinned official distribution")


def evidence_files(root, artifacts):
    result = {}
    for item in artifacts:
        relative = item["path"]
        if relative in result:
            raise ValueError("duplicate evidence artifact")
        path = owned_path(root, relative)
        # Keep every downloaded/extracted vendor file outside admissible evidence.
        suffix = path.suffix.lower()
        if not relative.startswith("evidence/") or suffix not in (".json", ".mp3", ".flac", ".m4a"):
            raise ValueError("vendor/runtime files are not admissible evidence artifacts")
        if path.is_symlink() or not path.is_file() or digest(path) != item["sha256"] or path.stat().st_size != item["bytes"]:
            raise ValueError(f"evidence artifact changed: {relative}")
        with path.open("rb") as stream:
            magic = stream.read(12)
        if suffix == ".json":
            json.loads(path.read_text(encoding="utf-8"))
        elif not ((suffix == ".mp3" and (magic.startswith(b"ID3") or len(magic) >= 2 and magic[0] == 255 and magic[1] & 224 == 224))
                  or (suffix == ".flac" and magic.startswith(b"fLaC"))
                  or (suffix == ".m4a" and magic[4:8] == b"ftyp")):
            raise ValueError("evidence media must be an audio carrier, not a renamed runtime binary")
        result[relative] = item
    return result


def timestamp(value):
    try:
        result = datetime.fromisoformat(value.replace("Z", "+00:00"))
        if result.tzinfo is None:
            raise ValueError("missing timezone")
        return result
    except (AttributeError, TypeError, ValueError) as error:
        raise ValueError("observations require an ISO-8601 timestamp with timezone") from error


def validate_capture(root, contract_path):
    record = json.loads((root / "capture.json").read_text(encoding="utf-8"))
    if record.get("schema") != SCHEMA or record.get("observation_type") != "runtime_save_capture" or record.get("status") != "complete":
        raise ValueError("a diagnostic, static or synthetic record cannot qualify as a completed runtime capture")
    validate_reference(record["reference"])
    runtime = record["runtime"]
    if (runtime.get("os") != "Windows" or not runtime.get("os_version")
            or any(type(runtime.get(key)) is not int or runtime[key] <= 0 for key in ("session_id", "process_id"))
            or not runtime.get("process_creation_time_utc")
            or not re.fullmatch(r"[0-9a-f]{64}", runtime.get("collector_source_sha256", ""))):
        raise ValueError("actual Windows process/session/collector provenance is required")
    started = timestamp(runtime["process_creation_time_utc"])
    completed = timestamp(record["recorded_at_utc"])
    artifacts = evidence_files(root, record["artifacts"])
    configuration = record["configuration"]
    if configuration.get("mode") != "portable" or configuration.get("evidence") not in artifacts:
        raise ValueError("portable mode and configuration location must be observed")
    config = owned_path(root, configuration["observed_path"])
    if config != root / "runtime/application/mp3tag.cfg" or not config.is_file() or not config.stat().st_size or digest(config) != configuration.get("sha256"):
        raise ValueError("observed portable configuration is absent or changed")
    defaults = record["defaults"]
    if defaults.get("evidence") not in artifacts or any(type(defaults.get(key)) is not bool for key in DEFAULT_SWITCHES):
        raise ValueError("all selected MPEG read/write/remove defaults require runtime evidence")
    if defaults.get("id3v2_format") not in ("v23_utf16", "v23_latin1", "v24_utf8"):
        raise ValueError("the selected ID3v2 encoding must be captured")
    inspected = [timestamp(configuration["observed_at_utc"]), timestamp(defaults["observed_at_utc"])]
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    if contract.get("schema") != 1 or not contract.get("baseline") or digest(contract_path) != record["contract_sha256"]:
        raise ValueError("prepared input contract is missing or changed")
    cases = {(case["operation"], case["file"]): case for case in record["cases"]}
    expected = {(operation, name) for operation in OPERATIONS for name in contract["baseline"]}
    if cases.keys() != expected or len(cases) != len(record["cases"]):
        raise ValueError("every prepared file/operation requires exactly one capture")
    for (operation, name), case in cases.items():
        if Path(name).name != name or "\\" in name or PureWindowsPath(name).drive or case["input_sha256"] != contract["baseline"][name]["input_sha256"]:
            raise ValueError("reference input identity does not match its prepared pair")
        if not case.get("operator_steps") or not all(step.get("action") and step.get("observed_at_utc") and step.get("evidence") in artifacts for step in case["operator_steps"]):
            raise ValueError("actual operator actions and observations are required")
        actions = [step["action"] for step in case["operator_steps"]]
        if [action for action in actions if action in ("edit", "save", "undo", "redo", "reload")] != ["edit", "save", "undo", "redo", "reload"]:
            raise ValueError("operator steps must include edit, save, undo, redo and reload in order")
        times = [timestamp(step["observed_at_utc"]) for step in case["operator_steps"]]
        if times != sorted(times) or not all(started <= observed <= times[0] for observed in inspected) or times[-1] > completed:
            raise ValueError("configuration/default observations must precede the ordered tagging steps in the same process lifetime")
        if set(case["stages"]) != set(STAGES):
            raise ValueError("save, undo and redo stage files are required")
        for stage, relative in case["stages"].items():
            if relative not in artifacts or relative != f"evidence/{operation}/{stage}/{name}":
                raise ValueError(f"missing or mismatched {stage} capture")
    return record


def export_identity(root, contract, destination):
    record = validate_capture(root, contract)
    # Export a JSON identity only. Never copy runtime/scratch trees or media.
    if destination.exists():
        raise ValueError("export destination must not exist")
    reference = record["reference"]
    identity = {"application": "Mp3tag", "version": "3.36.1", "executable_sha256": reference["executable_sha256"],
        "platform": record["runtime"]["os_version"], "runtime": record["runtime"],
        "recorded_at_utc": record["recorded_at_utc"], "defaults": record["defaults"],
        "evidence": record["artifacts"], "input_sha256": {case["file"]: case["input_sha256"] for case in record["cases"]}}
    with destination.open("x", encoding="utf-8") as output:
        output.write(json.dumps(identity, indent=2, ensure_ascii=False) + "\n")
    return {"status": "validated_operator_identity_export", "output": str(destination), "parity_comparison_performed": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    prepare = commands.add_parser("prepare-runtime")
    prepare.add_argument("workspace", type=Path)
    prepare.add_argument("--installer", type=Path)
    prepare.add_argument("--seven-zip")
    for action in ("validate", "export-identity"):
        command = commands.add_parser(action)
        command.add_argument("workspace", type=Path)
        command.add_argument("--contract", required=True, type=Path)
        if action == "export-identity":
            command.add_argument("--destination", required=True, type=Path)
    args = parser.parse_args()
    if args.command == "prepare-runtime":
        result = prepare_runtime(args.workspace, args.installer, args.seven_zip)
    elif args.command == "validate":
        record = validate_capture(args.workspace, args.contract)
        result = {"status": "valid_capture_manifest", "cases": len(record["cases"]), "parity_comparison_performed": False}
    else:
        result = export_identity(args.workspace, args.contract, args.destination)
    print(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        print(f"Reference capture failed: {error}", file=sys.stderr)
        sys.exit(2)
