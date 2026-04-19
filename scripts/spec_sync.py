#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import re
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC_DIR = ROOT / "spec"
USER_AGENT = "rust-genai-spec-sync/0.2.0"
GOOGLE_API_KEY_RE = re.compile(r"AIza[0-9A-Za-z\-_]{35}")

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
    # ai.google.dev documentation pages can embed public site bootstrap keys in HTML.
    # Keep the snapshot content while removing secret-scanning hits from generated files.
    sanitized = GOOGLE_API_KEY_RE.sub("REDACTED_GOOGLE_API_KEY", raw)
    return sanitized if sanitized.endswith("\n") else sanitized + "\n"


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def render_spec_diff_report(
    generated_at: str,
    manifest_sources: list[dict[str, object]],
    previous_hashes: dict[str, str],
) -> str:
    lines = [
        "# Spec Diff Report",
        "",
        f"Generated at: `{generated_at}`",
        "",
        "| File | Status | Previous SHA256 | Current SHA256 |",
        "|------|--------|-----------------|----------------|",
    ]

    for source in manifest_sources:
        file_name = str(source["file"])
        current_sha = str(source["sha256"])
        previous_sha = previous_hashes.get(file_name)
        if previous_sha is None:
            status = "new"
            previous_value = "-"
        elif previous_sha == current_sha:
            status = "unchanged"
            previous_value = previous_sha
        else:
            status = "updated"
            previous_value = previous_sha
        lines.append(
            f"| `{file_name}` | {status} | `{previous_value}` | `{current_sha}` |"
        )

    lines.extend(
        [
            "",
            "Use this report together with `git diff -- spec` during review.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    SPEC_DIR.mkdir(parents=True, exist_ok=True)

    generated_at = datetime.now(timezone.utc).replace(microsecond=0).isoformat()
    manifest_sources = []
    changed_files: list[str] = []
    manifest_path = SPEC_DIR / "manifest.json"
    previous_manifest = (
        json.loads(manifest_path.read_text(encoding="utf-8"))
        if manifest_path.exists()
        else None
    )
    previous_hashes = {
        entry["file"]: entry["sha256"]
        for entry in previous_manifest.get("sources", [])
    } if previous_manifest else {}

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
    previous_manifest_text = (
        json.dumps(previous_manifest, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
        if previous_manifest
        else None
    )
    if previous_manifest_text != manifest_text:
        changed_files.append("manifest.json")
    manifest_path.write_text(manifest_text, encoding="utf-8")

    report_text = render_spec_diff_report(generated_at, manifest_sources, previous_hashes)
    report_path = SPEC_DIR / "spec-diff.md"
    previous_report = report_path.read_text(encoding="utf-8") if report_path.exists() else None
    if previous_report != report_text:
        changed_files.append("spec-diff.md")
    report_path.write_text(report_text, encoding="utf-8")

    if changed_files:
        print("Updated:", ", ".join(changed_files))
    else:
        print("Spec snapshot is already current.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
