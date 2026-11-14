"""Update the checked-in shared-protocol snapshot from the adjacent hardware repo.
Review and commit the resulting vendor diff with the server change; CI never fetches a moving branch.
"""
from pathlib import Path
import argparse, hashlib, json, shutil, tomllib, re
root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--check", action="store_true", help="verify checked-in snapshot hashes without requiring amigo checkout")
args = parser.parse_args()
destination = root / "vendor/amigo"
if args.check:
    manifest = json.loads((destination / "snapshot.json").read_text())
    actual = {p.relative_to(destination).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in sorted(destination.rglob("*")) if p.is_file() and p.name != "snapshot.json"}
    if actual != manifest["sha256"]: raise SystemExit("protocol snapshot changed: run sync-amigo-protocol.py and review the diff")
else:
    source = root.parent / "amigo/crates/amigo"
    workspace = tomllib.loads((source.parents[1] / "Cargo.toml").read_text())
    destination.mkdir(parents=True, exist_ok=True)
    # Explicit source files only; never copy build products or local configuration.
    for folder in ["src", "tests", "examples"]:
        for path in sorted((source / folder).rglob("*")):
            if path.is_file():
                target = destination / path.relative_to(source)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(path, target)
    text = (source / "Cargo.toml").read_text()
    for key, value in workspace["workspace"]["package"].items():
        text = re.sub(r"^" + re.escape(key) + r"\.workspace = true$", lambda _: f"{key} = {json.dumps(value)}", text, flags=re.M)
    original = tomllib.loads((source / "Cargo.toml").read_text())
    for key, own in original["dependencies"].items():
        shared = workspace["workspace"]["dependencies"][key]
        spec = {"version": shared} if isinstance(shared, str) else dict(shared)
        features = spec.pop("features", []) + own.get("features", [])
        spec.update({k:v for k,v in own.items() if k not in ("workspace", "features")})
        if features: spec["features"] = list(dict.fromkeys(features))
        replacement = key + " = { " + ", ".join(f"{k} = {json.dumps(v)}" for k,v in spec.items()) + " }"
        text = re.sub(r"^" + re.escape(key) + r" = .*?$", lambda _: replacement, text, flags=re.M)
    text = text.replace("[lints]\nworkspace = true", "[workspace]")
    (destination / "Cargo.toml").write_text(text)
    hashes = {p.relative_to(destination).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in sorted(destination.rglob("*")) if p.is_file() and p.name != "snapshot.json"}
    (destination / "snapshot.json").write_text(json.dumps({"source": "amigo/crates/amigo", "sha256": hashes}, indent=2) + "\n")
