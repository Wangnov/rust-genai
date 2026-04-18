#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC_DIR = ROOT / "spec"
USER_AGENT = "rust-genai-spec-sync/0.2.0"

SOURCES = (
    {
        "file": "openapi3_0.json",
        "kind": "json",
        "url": "https://generativelanguage.googleapis.com/$discovery/OPENAPI3_0",
    },
    {
        "file": "api-versions.md",
        "kind": "text",
        "url": "https://ai.google.dev/gemini-api/docs/api-versions.md",
    },
    {
        "file": "deprecations.md",
        "kind": "text",
        "url": "https://ai.google.dev/gemini-api/docs/deprecations.md",
    },
)


def fetch_text(url: str) -> str:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read().decode("utf-8")


def normalize_text(raw: str, kind: str) -> str:
    if kind == "json":
        return json.dumps(json.loads(raw), indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    return raw if raw.endswith("\n") else raw + "\n"


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def main() -> int:
    SPEC_DIR.mkdir(parents=True, exist_ok=True)

    generated_at = datetime.now(timezone.utc).replace(microsecond=0).isoformat()
    manifest_sources = []
    changed_files: list[str] = []

    for source in SOURCES:
        target = SPEC_DIR / source["file"]
        normalized = normalize_text(fetch_text(source["url"]), source["kind"])
        previous = target.read_text(encoding="utf-8") if target.exists() else None
        if previous != normalized:
            changed_files.append(source["file"])
        target.write_text(normalized, encoding="utf-8")
        manifest_sources.append(
            {
                "file": source["file"],
                "kind": source["kind"],
                "sha256": sha256_text(normalized),
                "size_bytes": len(normalized.encode("utf-8")),
                "url": source["url"],
            }
        )

    manifest = {
        "generated_at": generated_at,
        "sources": manifest_sources,
    }
    manifest_text = json.dumps(manifest, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    manifest_path = SPEC_DIR / "manifest.json"
    previous_manifest = (
        manifest_path.read_text(encoding="utf-8") if manifest_path.exists() else None
    )
    if previous_manifest != manifest_text:
        changed_files.append("manifest.json")
    manifest_path.write_text(manifest_text, encoding="utf-8")

    if changed_files:
        print("Updated:", ", ".join(changed_files))
    else:
        print("Spec snapshot is already current.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
