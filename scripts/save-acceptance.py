#!/usr/bin/env python3
"""Prepare disposable paired inputs, exercise TagGroove, consume reference captures.

No reference application is launched. `compare` requires operator-captured files
and provenance; a successful local run cannot produce a reference parity result.
"""
import argparse
import base64
import copy
import hashlib
import json
import platform
from pathlib import Path
import shutil
import subprocess
import sys
from datetime import datetime, timezone

REPO = Path(__file__).resolve().parents[1]
FORMATS = ("mp3", "flac", "m4a")
OPERATIONS = ("edit-one", "edit-many")
REFERENCE_SHA256 = "484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911"
IDENTITY_FIELDS = ("source_head", "source_diff_sha256", "source_status", "source_files_sha256",
                   "backend_sha256", "cargo_lock_sha256", "rustc", "platform", "scope")


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def command(args):
    return subprocess.check_output(args, cwd=REPO, text=True).strip()


def backend():
    subprocess.run(["cargo", "build", "-p", "tag-core", "--example", "save_acceptance", "--locked"], cwd=REPO, check=True)
    metadata = json.loads(command(["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"]))
    suffix = ".exe" if sys.platform == "win32" else ""
    return Path(metadata["target_directory"]) / "debug" / "examples" / ("save_acceptance" + suffix)


def invoke(executable, *args):
    return json.loads(command([str(executable), *map(str, args)]))


def normalize(snapshot):
    value = copy.deepcopy(snapshot)
    value["audio_payload_sha256"] = sha256(base64.b64decode(value.pop("audio_payload_base64"), validate=True))
    value["opaque_native_sha256"] = sha256(json.dumps(value["opaque_native"], sort_keys=True, ensure_ascii=False).encode("utf-8"))
    if value["native_id3"] is not None:
        for frame in value["native_id3"]:
            frame["raw_body_sha256"] = sha256(base64.b64decode(frame["raw_body_base64"], validate=True))
            if frame["frame"] == "APIC":
                frame["image_sha256"] = sha256(base64.b64decode(frame["image_base64"], validate=True))
    value["native_id3_sha256"] = sha256(json.dumps(value["native_id3"], sort_keys=True, ensure_ascii=False).encode("utf-8"))
    for cover in value["covers"]:
        cover["sha256"] = sha256(base64.b64decode(cover.pop("data_base64"), validate=True))
    return value


def expected(before, operation):
    value = copy.deepcopy(before)
    value["fields"]["TITLE"] = ["Changed café 日本"]
    if operation == "edit-many":
        value["fields"].pop("ALBUM", None)
        value["fields"]["ARTIST"] = ["Third", "Fourth"]
    return value


def differences(expected_value, observed, prefix=""):
    """Compare the complete projected field, cover, opaque and audio structure."""
    if isinstance(expected_value, dict) and isinstance(observed, dict):
        result = []
        for key in sorted(expected_value.keys() | observed.keys()):
            child = f"{prefix}/{key}"
            if key not in expected_value or key not in observed:
                result.append(child)
            else:
                result.extend(differences(expected_value[key], observed[key], child))
        return result
    return [] if expected_value == observed else [prefix or "/"]


def native_id3_semantics(snapshot):
    frames = snapshot["native_id3"]
    if frames is None:
        return None
    values = [{key: value for key, value in frame.items()
               if key not in ("raw_body_base64", "raw_body_sha256", "flags")} for frame in frames]
    return sorted(values, key=lambda value: json.dumps(value, sort_keys=True, ensure_ascii=False))


def native_id3_difference_kind(expected_value, observed):
    if native_id3_semantics(expected_value) != native_id3_semantics(observed):
        return "semantic_observation"
    if expected_value["native_id3"] != observed["native_id3"]:
        return "serialization_only"
    return "none"


def identity(executable):
    sources = ["scripts/save-acceptance.py", "crates/tag-core/examples/save_acceptance.rs",
        "crates/tag-core/tests/support/native_id3.rs",
        "crates/tag-core/tests/support/save_corpus.rs", "crates/tag-core/tests/save_acceptance.rs",
        "scripts/tests/test_save_acceptance.py", "Cargo.lock"]
    sources.extend(str(p.relative_to(REPO)) for p in sorted((REPO / "crates/tag-core/tests/fixtures/save-corpus").iterdir()))
    return {
        "source_head": command(["git", "rev-parse", "HEAD"]),
        "source_diff_sha256": sha256(subprocess.check_output(["git", "diff", "HEAD", "--binary"], cwd=REPO)),
        "source_status": command(["git", "status", "--porcelain"]),
        "source_files_sha256": {name: sha256((REPO / name).read_bytes()) for name in sources},
        "backend_sha256": sha256(executable.read_bytes()),
        "cargo_lock_sha256": sha256((REPO / "Cargo.lock").read_bytes()),
        "rustc": command(["rustc", "--version"]),
        "platform": platform.platform(),
        "recorded_at_utc": datetime.now(timezone.utc).isoformat(),
        "scope": "tag-core backend; no native UI, packaged app or release validation",
    }


def require_identity(producer, current, context):
    changed = [key for key in IDENTITY_FIELDS
               if key not in producer or key not in current or producer[key] != current[key]]
    if changed:
        raise ValueError(f"{context} identity differs: {', '.join(changed)}")


def stage_hashes(root, operation, name):
    return {stage: sha256((root / "taggroove" / operation / stage / name).read_bytes())
            for stage in ("saved", "undone", "redone")}


def replay_byte_errors(input_sha256, hashes):
    return {
        "undo_bytes": [] if hashes["undone"] == input_sha256 else ["undone bytes differ from the retained input"],
        "redo_bytes": [] if hashes["redone"] == hashes["saved"] else ["redone bytes differ from the retained saved stage"],
    }


def prepare(root, executable, corpus=None):
    producer = identity(executable)
    root.mkdir()  # Never overwrite an existing workspace.
    if corpus is None:
        defaults = invoke(executable, "prepare", root / "inputs")
        provenance = {"kind": "independent synthetic retained corpus"}
    else:
        corpus = corpus.resolve()
        sources = sorted(p for p in corpus.iterdir() if p.suffix.lower().lstrip(".") in FORMATS)
        if not sources or any(not p.is_file() or p.is_symlink() for p in sources):
            raise ValueError("captured corpus must contain regular MP3, FLAC or M4A files")
        (root / "inputs").mkdir()
        for source in sources:
            shutil.copyfile(source, root / "inputs" / source.name)
        defaults = {"taggroove_defaults": invoke(executable, "defaults", root)}
        provenance = {"kind": "operator-supplied captured input copies", "source_directory": str(corpus),
            "source_sha256": {p.name: sha256(p.read_bytes()) for p in sources}}
    baseline = {}
    for path in sorted((root / "inputs").iterdir()):
        name = path.name
        baseline[name] = {"input_sha256": sha256(path.read_bytes()), "snapshot": normalize(invoke(executable, "observe", path))}
        for operation in OPERATIONS:
            for app in ("taggroove", "reference"):
                directory = root / app / operation / "input"
                directory.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(path, directory / name)
    require_identity(producer, identity(executable), "Completed preparation producer")
    write_json(root / "contract.json", {"schema": 1, "producer": producer, "input_provenance": provenance, **defaults,
        "reference": {"version": "Mp3tag 3.36.1 Windows x64", "executable_sha256": REFERENCE_SHA256,
            "runtime_comparison": "not performed", "defaults": "not captured"}, "baseline": baseline,
        "operations": {op: {name: expected(record["snapshot"], op)["fields"] for name, record in baseline.items()} for op in OPERATIONS}})
    (root / "REFERENCE-INSTRUCTIONS.txt").write_text(
        "Use only the disposable reference/<operation>/input files. Verify each input SHA-256 against contract.json.\n"
        "Capture the reference executable identity, Windows/runtime identity, and actual Options defaults before editing.\n"
        "Run each operation independently using the contract's expected field changes. Preserve every other field and cover.\n"
        "Save, copy the files to reference/<operation>/saved/, then Undo and copy to undone/, then Redo and copy to redone/.\n"
        "Close/reopen and inspect the saved copies. Do not save again after Undo before capturing it.\n"
        "Record actual defaults as an object and capture filenames in an identity JSON with application, version,\n"
        "executable_sha256, platform, runtime, recorded_at_utc, defaults and evidence (nonempty array).\n"
        "Add input_sha256 mapping each filename to the verified pre-edit hash from contract.json.\n"
        "Run: python3 scripts/save-acceptance.py compare WORKSPACE --identity IDENTITY.json\n"
        "Cancellation/interruption, native gain-consumer behavior and UI behavior need separate evidence.\n", encoding="utf-8")
    return {"workspace": str(root), "reference_comparison": "not performed"}


def validate_inputs(root, executable, contract):
    if contract.get("schema") != 1 or not isinstance(contract.get("baseline"), dict) or not contract["baseline"]:
        raise ValueError("contract requires a nonempty captured input baseline")
    for name, baseline in contract["baseline"].items():
        if Path(name).name != name:
            raise ValueError("input filename must remain inside the workspace")
        path = root / "inputs" / name
        if sha256(path.read_bytes()) != baseline["input_sha256"]:
            raise ValueError(f"retained input changed: {name}")
        if differences(baseline["snapshot"], normalize(invoke(executable, "observe", path))):
            raise ValueError(f"baseline does not match the retained input: {name}")


def run(root, executable):
    contract = json.loads((root / "contract.json").read_text(encoding="utf-8"))
    producer = identity(executable)
    require_identity(contract["producer"], producer, "Prepared/run producer")
    validate_inputs(root, executable, contract)
    receipt_path = root / "taggroove-results.json"
    if receipt_path.exists():
        raise ValueError("results already exist; prepare a new disposable workspace")
    cases = []
    for operation in OPERATIONS:
        for name, baseline in contract["baseline"].items():
            path = root / "taggroove" / operation / "input" / name
            if sha256(path.read_bytes()) != baseline["input_sha256"]:
                raise ValueError(f"paired input changed: {path}")
            result = invoke(executable, "exercise", path, operation, path.parent.parent)
            stages = {stage: normalize(invoke(executable, "observe", path.parent.parent / stage / name))
                      for stage in ("saved", "undone", "redone")}
            hashes = stage_hashes(root, operation, name)
            wanted = expected(baseline["snapshot"], operation)
            errors = {stage: differences(baseline["snapshot"] if stage == "undone" else wanted, observed) for stage, observed in stages.items()}
            errors.update(replay_byte_errors(baseline["input_sha256"], hashes))
            for key in ("undo_errors", "redo_errors"):
                if result[key]:
                    errors[key] = result[key]
            for key in ("invalid_write_rejected", "rejection_unchanged"):
                if result[key] is not True:
                    errors[key] = ["failed"]
            cases.append({"operation": operation, "file": name, "expected_saved": wanted, "observed": stages,
                          "file_sha256": hashes, "errors": errors})
    passed = all(not any(case["errors"].values()) for case in cases)
    policy = invoke(executable, "policy", root / "mpeg-policy")
    policy_passed = all(record["audio_preserved"] and record["undo_exact"] and not record["undo_errors"] for record in policy.values())
    policy_passed = policy_passed and policy["default"]["native_ape_after"] is None
    native = policy["explicit_write_ape"]["native_ape_after"]
    policy_passed = policy_passed and native == {"mp3gain_undo": 'Text("+001,+001,N")',
        "mp3gain_minmax": 'Text("094,205")', "opaque_companion": "Binary([1, 0, 255, 17])"}
    passed = passed and policy_passed
    require_identity(producer, identity(executable), "Completed run producer")
    receipt = {"passed": passed, "producer": producer, "reference_comparison": "not performed", "mpeg_policy": policy, "cases": cases}
    write_json(receipt_path, receipt)
    return {"passed": passed, "cases": len(cases), "receipt": str(receipt_path), "reference_comparison": "not performed"}


def compare(root, executable, identity_path):
    contract = json.loads((root / "contract.json").read_text(encoding="utf-8"))
    local = json.loads((root / "taggroove-results.json").read_text(encoding="utf-8"))
    consumer = identity(executable)
    require_identity(contract["producer"], local["producer"], "Prepared/local capture producer")
    require_identity(local["producer"], consumer, "Local producer/current comparison")
    validate_inputs(root, executable, contract)
    reference = json.loads(identity_path.read_text(encoding="utf-8"))
    for key in ("application", "version", "executable_sha256", "platform", "runtime", "recorded_at_utc", "defaults", "evidence", "input_sha256"):
        if not reference.get(key):
            raise ValueError(f"missing reference provenance: {key}")
    if reference["application"] != "Mp3tag" or reference["version"] != "3.36.1" or reference["executable_sha256"] != REFERENCE_SHA256:
        raise ValueError("reference identity does not match the pinned original")
    if not isinstance(reference["defaults"], dict) or not isinstance(reference["evidence"], list):
        raise ValueError("defaults must be an object and evidence a nonempty array")
    if local["passed"] is not True:
        raise ValueError("local preservation checks failed")
    local_cases = {(case["operation"], case["file"]): case for case in local["cases"]}
    wanted_cases = {(operation, name) for operation in OPERATIONS for name in contract["baseline"]}
    if local_cases.keys() != wanted_cases or len(local["cases"]) != len(wanted_cases):
        raise ValueError("local receipt does not contain exactly the captured cases")
    cases = []
    for operation in OPERATIONS:
        for name in contract["baseline"]:
            if reference["input_sha256"].get(name) != contract["baseline"][name]["input_sha256"]:
                raise ValueError(f"unverified reference input: {name}")
            hashes = stage_hashes(root, operation, name)
            byte_errors = replay_byte_errors(contract["baseline"][name]["input_sha256"], hashes)
            if any(byte_errors.values()):
                raise ValueError(f"retained local replay bytes failed preservation: {operation}/{name}: {byte_errors}")
            if hashes != local_cases[operation, name].get("file_sha256"):
                raise ValueError(f"retained local capture bytes changed since the producer run: {operation}/{name}")
            for stage in ("saved", "undone", "redone"):
                path = root / "reference" / operation / stage / name
                if not path.is_file():
                    raise ValueError(f"missing original-runtime capture: {path}")
                local_path = root / "taggroove" / operation / stage / name
                local_observed = normalize(invoke(executable, "observe", local_path))
                wanted = contract["baseline"][name]["snapshot"] if stage == "undone" else expected(contract["baseline"][name]["snapshot"], operation)
                if differences(wanted, local_observed):
                    raise ValueError(f"retained local output failed preservation: {local_path}")
                observed = normalize(invoke(executable, "observe", path))
                cases.append({"file": name, "operation": operation, "stage": stage,
                    "local_file_sha256": hashes[stage],
                    "reference_file_sha256": sha256(path.read_bytes()), "observed_reference": observed,
                    "native_id3_difference_kind": native_id3_difference_kind(local_observed, observed),
                    "differences_from_taggroove": differences(local_observed, observed)})
    passed = all(not c["differences_from_taggroove"] for c in cases)
    require_identity(consumer, identity(executable), "Completed comparison")
    receipt = {"passed": passed, "producer": local["producer"], "prepared_by": contract["producer"],
        "consumer": consumer, "reference": reference,
        "comparison_interpretation": "passed means strict observed equality; serialization-only native differences are not data-loss or incompatibility findings",
        "scope": "operator-supplied saved/undo/redo files; decoded through the same TagGroove reader; no complete parity certification", "cases": cases}
    write_json(root / "paired-results.json", receipt)
    return {"passed": passed, "cases": len(cases), "receipt": str(root / "paired-results.json")}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("prepare", "run", "compare"))
    parser.add_argument("workspace", type=Path)
    parser.add_argument("--identity", type=Path)
    parser.add_argument("--corpus", type=Path, help="prepare from captured regular MP3/FLAC/M4A files; originals are read only")
    args = parser.parse_args()
    if args.command == "compare" and args.identity is None:
        parser.error("compare requires --identity with captured reference provenance")
    executable = backend()
    root = args.workspace.resolve()
    if args.corpus is not None and args.command != "prepare":
        parser.error("--corpus applies only to prepare")
    result = prepare(root, executable, args.corpus) if args.command == "prepare" else run(root, executable) if args.command == "run" else compare(root, executable, args.identity)
    print(json.dumps(result, indent=2))
    return 0 if result.get("passed", True) else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"Save acceptance failed: {error}", file=sys.stderr)
        sys.exit(2)
