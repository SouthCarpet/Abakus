#!/usr/bin/env python3
"""Verify native Google view_file image payloads without reading conversation text."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sqlite3
import sys
from contextlib import closing
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence
from urllib.parse import quote


CONVERSATION_ID = re.compile(
    r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
)
SHA256 = re.compile(r"^[0-9a-f]{64}$")
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
DATABASE_ROOT = Path(r"C:\Users\Asus\.gemini\antigravity-cli\conversations")
BRAIN_ROOT = Path(r"C:\Users\Asus\.gemini\antigravity-cli\brain")
TOOL_PATH = (5, 4, 2)
SOURCE_PATH = (140, 1, 2)
MIME_PATH = (140, 2, 4, 1)
MEDIA_PATH = (140, 2, 4, 5)


class VerificationError(RuntimeError):
    """Raised when native image evidence is incomplete or inconsistent."""


class WireError(ValueError):
    """Raised when a protobuf wire payload is malformed."""


@dataclass(frozen=True)
class ExpectedImage:
    source: Path
    source_key: str
    sha256: str


@dataclass(frozen=True)
class ImagePayload:
    step: int
    status: int
    source: Path
    source_key: str
    media: Path
    mime: str
    error_details_empty: bool


def _read_varint(data: bytes, offset: int) -> tuple[int, int]:
    value = 0
    shift = 0
    while offset < len(data) and shift <= 63:
        byte = data[offset]
        offset += 1
        value |= (byte & 0x7F) << shift
        if byte < 0x80:
            return value, offset
        shift += 7
    raise WireError("truncated or overlong varint")


def _decode_text(value: bytes) -> str | None:
    try:
        text = value.decode("utf-8")
    except UnicodeDecodeError:
        return None
    if any(ord(char) < 32 and char not in "\r\n\t" for char in text):
        return None
    return text


def _parse_message(
    data: bytes, prefix: tuple[int, ...], depth: int
) -> list[tuple[tuple[int, ...], str]]:
    if depth > 12:
        raise WireError("protobuf nesting exceeds limit")
    offset = 0
    strings: list[tuple[tuple[int, ...], str]] = []
    while offset < len(data):
        key, offset = _read_varint(data, offset)
        field_number, wire_type = key >> 3, key & 7
        if field_number == 0 or wire_type not in (0, 1, 2, 5):
            raise WireError("invalid protobuf field")
        offset, found = _parse_field(data, offset, wire_type, prefix + (field_number,), depth)
        strings.extend(found)
    return strings


def _parse_field(
    data: bytes,
    offset: int,
    wire_type: int,
    path: tuple[int, ...],
    depth: int,
) -> tuple[int, list[tuple[tuple[int, ...], str]]]:
    if wire_type == 0:
        _, offset = _read_varint(data, offset)
        return offset, []
    if wire_type in (1, 5):
        width = 8 if wire_type == 1 else 4
        if offset + width > len(data):
            raise WireError("truncated fixed-width field")
        return offset + width, []
    length, offset = _read_varint(data, offset)
    end = offset + length
    if end > len(data):
        raise WireError("truncated length-delimited field")
    value = data[offset:end]
    found = [(path, text)] if (text := _decode_text(value)) is not None else []
    try:
        found.extend(_parse_message(value, path, depth + 1))
    except WireError:
        pass
    return end, found


def decode_wire_strings(payload: bytes) -> list[tuple[tuple[int, ...], str]]:
    if not payload:
        raise WireError("missing step payload")
    return _parse_message(payload, (), 0)


def _one(strings: Sequence[tuple[tuple[int, ...], str]], path: tuple[int, ...]) -> str | None:
    values = {value for candidate_path, value in strings if candidate_path == path}
    if not values:
        return None
    if len(values) != 1:
        raise WireError(f"conflicting values at protobuf path {path}")
    return values.pop()


def _one_absolute_png(
    strings: Sequence[tuple[tuple[int, ...], str]], path: tuple[int, ...]
) -> str | None:
    values = {
        value
        for candidate_path, value in strings
        if candidate_path == path and Path(value).is_absolute() and Path(value).suffix.lower() == ".png"
    }
    if not values:
        return None
    if len(values) != 1:
        raise WireError(f"conflicting PNG paths at protobuf path {path}")
    return values.pop()


def _path_key(path: Path | str) -> str:
    return os.path.normcase(os.path.normpath(str(path).replace("/", "\\")))


def _blob_empty(value: object) -> bool:
    if value is None:
        return True
    if isinstance(value, memoryview):
        value = value.tobytes()
    return value in (b"", "")


def _payload_from_row(row: Sequence[object]) -> ImagePayload | None:
    idx, step_type, status, error_details, payload, step_format = row
    if step_type != 132:
        raise VerificationError(f"step {idx}: unexpected step_type")
    if step_format != 0:
        raise VerificationError(f"step {idx}: unsupported step_format {step_format}")
    try:
        strings = decode_wire_strings(bytes(payload or b""))
    except (TypeError, WireError) as error:
        raise VerificationError(f"step {idx}: malformed payload ({error})") from error
    try:
        tool = _one(strings, TOOL_PATH)
        mime = _one(strings, MIME_PATH)
        media = _one_absolute_png(strings, MEDIA_PATH)
        if tool != "view_file" or (mime is None and media is None):
            return None
        source = _one_absolute_png(strings, SOURCE_PATH)
    except WireError as error:
        raise VerificationError(f"step {idx}: ambiguous native result ({error})") from error
    if not source or mime != "image/png" or not media:
        raise VerificationError(f"step {idx}: view_file lacks native image content")
    return ImagePayload(
        step=int(idx),
        status=int(status),
        source=Path(source),
        source_key=_path_key(source),
        media=Path(media),
        mime=mime,
        error_details_empty=_blob_empty(error_details),
    )


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _assert_png(path: Path, label: str) -> None:
    if not path.is_file():
        raise VerificationError(f"{label} is missing: {path}")
    with path.open("rb") as stream:
        if stream.read(len(PNG_SIGNATURE)) != PNG_SIGNATURE:
            raise VerificationError(f"{label} is not a PNG: {path}")


def _is_descendant(path: Path, root: Path) -> bool:
    try:
        path.resolve(strict=True).relative_to(root.resolve(strict=True))
    except (FileNotFoundError, ValueError):
        return False
    return True


def _read_expected(path: Path, conversation_id: str) -> list[ExpectedImage]:
    document = json.loads(path.read_text(encoding="utf-8"))
    manifest_id = document.get("conversation_id")
    if manifest_id is not None and manifest_id != conversation_id:
        raise VerificationError("manifest conversation_id does not match")
    records = document.get("images") or document.get("steps")
    if not isinstance(records, list) or not records:
        raise VerificationError("manifest must contain a non-empty images or steps array")
    expected = [_expected_record(record) for record in records]
    if len({image.source_key for image in expected}) != len(expected):
        raise VerificationError("manifest contains duplicate source paths")
    return expected


def _expected_record(record: object) -> ExpectedImage:
    if not isinstance(record, dict):
        raise VerificationError("each expected image must be an object")
    source_value = record.get("path", record.get("source"))
    hash_value = record.get("sha256", record.get("source_sha256"))
    if not isinstance(source_value, str) or not Path(source_value).is_absolute():
        raise VerificationError("each expected source must be an absolute path")
    if not isinstance(hash_value, str) or not SHA256.fullmatch(hash_value.lower()):
        raise VerificationError(f"invalid expected SHA-256 for {source_value}")
    return ExpectedImage(Path(source_value), _path_key(source_value), hash_value.lower())


def _verify_image(payload: ImagePayload, expected: ExpectedImage, brain_root: Path) -> dict[str, object]:
    if payload.status != 3 or not payload.error_details_empty:
        raise VerificationError(f"step {payload.step}: tool call did not complete cleanly")
    if not _is_descendant(payload.media, brain_root):
        raise VerificationError(f"step {payload.step}: media path is outside conversation storage")
    _assert_png(expected.source, "source image")
    _assert_png(payload.media, "native media image")
    source_hash = _sha256(expected.source)
    media_hash = _sha256(payload.media)
    if source_hash != expected.sha256 or media_hash != expected.sha256:
        raise VerificationError(f"step {payload.step}: source/media hash mismatch")
    return {
        "step": payload.step,
        "status": payload.status,
        "tool": "view_file",
        "source": str(expected.source),
        "source_sha256": source_hash,
        "mime": payload.mime,
        "media_path": str(payload.media),
        "media_sha256": media_hash,
        "error_details_empty": True,
    }


def verify_rows(
    rows: Iterable[Sequence[object]],
    expected: Sequence[ExpectedImage],
    brain_root: Path,
) -> list[dict[str, object]]:
    payloads = [candidate for row in rows if (candidate := _payload_from_row(row)) is not None]
    expected_by_source = {image.source_key: image for image in expected}
    actual_keys = {payload.source_key for payload in payloads}
    expected_keys = {image.source_key for image in expected}
    missing = expected_keys - actual_keys
    unexpected = actual_keys - expected_keys
    if missing or unexpected:
        raise VerificationError(
            f"native image coverage differs: missing={len(missing)}, unexpected={len(unexpected)}"
        )
    return [
        _verify_image(payload, expected_by_source[payload.source_key], brain_root)
        for payload in payloads
    ]


def _read_rows(database: Path) -> list[tuple[object, ...]]:
    uri = f"file:{quote(database.as_posix(), safe='/:')}?mode=ro"
    with closing(sqlite3.connect(uri, uri=True)) as connection:
        connection.execute("PRAGMA query_only = ON")
        return connection.execute(
            "SELECT idx, step_type, status, error_details, step_payload, step_format "
            "FROM steps WHERE step_type = 132 ORDER BY idx"
        ).fetchall()


def _write_result(result: dict[str, object], output: Path | None) -> None:
    encoded = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    if output is None:
        sys.stdout.write(encoded)
        return
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("x", encoding="utf-8", newline="\n") as stream:
        stream.write(encoded)


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--conversation-id", required=True)
    parser.add_argument("--expected-manifest", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    conversation_id = args.conversation_id.lower()
    if not CONVERSATION_ID.fullmatch(conversation_id):
        raise VerificationError("conversation ID must be a lowercase UUID")
    database = DATABASE_ROOT / f"{conversation_id}.db"
    if not database.is_file():
        raise VerificationError(f"conversation database is missing: {database}")
    expected = _read_expected(args.expected_manifest, conversation_id)
    images = verify_rows(_read_rows(database), expected, BRAIN_ROOT / conversation_id)
    unique_image_count = len({str(image["source"]).lower() for image in images})
    _write_result(
        {
            "conversation_id": conversation_id,
            "verified_image_call_count": len(images),
            "verified_unique_image_count": unique_image_count,
            "all_completed_with_native_image_content": True,
            "exact_source_media_hash_matches": True,
            "images": images,
        },
        args.output,
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, sqlite3.Error, json.JSONDecodeError, VerificationError) as error:
        print(f"verification failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
