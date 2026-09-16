"""Shared coverage execution policy. No process launches or feature changes."""
import hashlib
import json
import os
import re
import subprocess

MODES = ("non-rendered", "full-ui")
BASELINE = "docs/plans/5-testing-v2/coverage-baseline.json"


def acceptance(root, mode, value=None):
    value = value or ("baseline" if mode == "non-rendered" else "target")
    if value not in {"baseline", "target"}:
        raise ValueError("unknown coverage acceptance policy")
    if value == "baseline":
        if mode != "non-rendered":
            raise ValueError("baseline acceptance is only for non-rendered execution")
        return {"policy": value, "baseline_sha256": hashlib.sha256((root / BASELINE).read_bytes()).hexdigest()}
    return {"policy": value}


def resolve(mode, environment=None):
    environment = os.environ if environment is None else environment
    flag = environment.get("UI_E2E_ENABLED", "0")
    if flag not in {"0", "1"}:
        raise ValueError("UI_E2E_ENABLED must be exactly 0 or 1")
    if mode not in MODES:
        raise ValueError("unknown coverage execution mode")
    if mode == "full-ui" and flag != "1":
        raise ValueError("full-ui coverage requires UI_E2E_ENABLED=1")
    return {"schema_version": 1, "mode": mode, "ui_e2e_enabled": flag == "1"}


def comparison_base(root, value):
    return subprocess.check_output(
        ["git", "rev-parse", "--verify", "--end-of-options", f"{value}^{{commit}}"],
        cwd=root, text=True,
    ).strip()


def validate(document, root, expected_mode, expected_base=None, expected_acceptance=None):
    """Bind the expected mode to the policy captured before instrumentation."""
    if expected_mode not in MODES or document.get("mode") != expected_mode:
        raise ValueError("coverage execution mode mismatch")
    proof = document.get("execution_policy", {})
    path = (root / proof.get("path", "")).resolve()
    if not path.is_relative_to((root / "target/coverage" / expected_mode).resolve()):
        raise ValueError("execution policy is outside its coverage mode")
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != proof.get("sha256"):
        raise ValueError("execution policy changed since instrumentation")
    policy = json.loads(data)
    captured_acceptance = policy.pop("acceptance", None)
    if expected_acceptance is not None and captured_acceptance != expected_acceptance:
        raise ValueError("coverage acceptance policy changed since instrumentation")
    if captured_acceptance is not None:
        if document.get("acceptance") != captured_acceptance or captured_acceptance != acceptance(root, expected_mode, captured_acceptance.get("policy")):
            raise ValueError("coverage baseline or acceptance policy changed since instrumentation")
    captured_base = policy.pop("comparison_base", None)
    if captured_base is not None and not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", captured_base):
        raise ValueError("invalid captured comparison base")
    if expected_base is not None and captured_base != expected_base:
        raise ValueError("coverage comparison base missing or changed since instrumentation")
    if type(policy.get("ui_e2e_enabled")) is not bool:
        raise ValueError("invalid captured UI execution flag")
    if policy != resolve(expected_mode, {"UI_E2E_ENABLED": "1" if policy["ui_e2e_enabled"] else "0"}):
        raise ValueError("captured execution policy mismatch")
