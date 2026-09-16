"""Reviewed non-rendered coverage ratchet; never a test/provenance waiver.

Floors are per-package and aggregate line/function ratios from recorded runs.
Unmapped and unmeasured files remain visible and may not silently expand/change.
The strict long-term/changed-code report is retained separately by coverage.py.
"""
from fractions import Fraction
import hashlib
import re


def file_hashes(root, names):
    return {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in sorted(names)}


def evaluate(document, scopes, source_names, unmapped, support, root):
    if set(document) != {"schema_version", "mode", "approved", "evidence", "scopes", "sources", "unmapped", "unmeasured_support"}:
        raise ValueError("baseline fields do not match the reviewed schema")
    if document["schema_version"] != 1 or document["mode"] != "non-rendered" or document["approved"] != "2026-09-16 user-approved no-regression policy":
        raise ValueError("invalid baseline schema/mode/approval")
    if not isinstance(document["evidence"], list) or not document["evidence"]:
        raise ValueError("baseline must reference executed evidence")
    for proof in document["evidence"]:
        if set(proof) != {"revision", "run", "report_sha256"} or not re.fullmatch(r"[0-9a-f]{40}", proof["revision"]) or not re.fullmatch(r"[0-9a-f]{64}", proof["report_sha256"]) or not proof["run"]:
            raise ValueError("invalid baseline evidence identity")
    if not isinstance(document["sources"], list) or len(set(document["sources"])) != len(document["sources"]):
        raise ValueError("baseline source inventory must be unique")
    failures = []
    if set(document["scopes"]) != set(scopes):
        failures.append("baseline package inventory changed; explicit review required")
    for name, floor in document["scopes"].items():
        if set(floor) != {"line_covered", "line_total", "function_covered", "function_total"}:
            raise ValueError("invalid baseline scope counters")
        for kind in ("line", "function"):
            hit, total = floor[f"{kind}_covered"], floor[f"{kind}_total"]
            if type(hit) is not int or type(total) is not int or not 0 <= hit <= total or total <= 0:
                raise ValueError("invalid baseline ratio")
            current = scopes.get(name)
            if current is None:
                continue
            if current[f"{kind}_total"] <= 0 or Fraction(current[f"{kind}_covered"], current[f"{kind}_total"]) < Fraction(hit, total):
                failures.append(f"baseline regression: {name} {kind} coverage")
    # New production files require explicit measurement/review rather than
    # inheriting a historical allowance for a different source inventory.
    added = sorted(set(source_names) - set(document["sources"]))
    if added:
        failures.append("new production sources require baseline review: " + ", ".join(added))
    for name, recorded in (("unmapped", document["unmapped"]), ("unmeasured_support", document["unmeasured_support"])):
        names = unmapped if name == "unmapped" else support
        if not isinstance(recorded, dict) or any(not re.fullmatch(r"[0-9a-f]{64}", value) for value in recorded.values()):
            raise ValueError("invalid unmeasured-source hashes")
        current = file_hashes(root, names)
        changed = [path for path, digest in current.items() if recorded.get(path) != digest]
        if changed:
            failures.append(f"{name} inventory changed without measurement/review: " + ", ".join(changed))
    return failures
