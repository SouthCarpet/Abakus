from __future__ import annotations

import hashlib
import importlib.util
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("verify-google-image-payloads.py")
SPEC = importlib.util.spec_from_file_location("google_image_payloads", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def _varint(value: int) -> bytes:
    encoded = bytearray()
    while value > 0x7F:
        encoded.append((value & 0x7F) | 0x80)
        value >>= 7
    encoded.append(value)
    return bytes(encoded)


def _field(number: int, value: bytes | str) -> bytes:
    raw = value.encode("utf-8") if isinstance(value, str) else value
    return _varint((number << 3) | 2) + _varint(len(raw)) + raw


def _native_payload(source: Path, media: Path, include_image: bool = True) -> bytes:
    tool = _field(5, _field(4, _field(2, "view_file")))
    source_message = _field(1, _field(2, str(source)))
    if not include_image:
        text = _field(4, _field(1, "text/plain") + _field(5, str(media)))
        return tool + _field(140, source_message + _field(2, text))
    image = _field(4, _field(1, "image/png") + _field(5, str(media)))
    return tool + _field(140, source_message + _field(2, image))


class VerifyGoogleImagePayloadsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.brain = self.root / "brain"
        self.brain.mkdir()
        self.source = self.root / "source.png"
        self.media = self.brain / "media.png"
        image = MODULE.PNG_SIGNATURE + b"synthetic-image"
        self.source.write_bytes(image)
        self.media.write_bytes(image)
        digest = hashlib.sha256(image).hexdigest()
        self.expected = [
            MODULE.ExpectedImage(self.source, MODULE._path_key(self.source), digest)
        ]

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def row(self, payload: bytes | None, step: int = 7) -> tuple[object, ...]:
        return (step, 132, 3, b"", payload, 0)

    def assert_rejected(self, payload: bytes | None, message: str) -> None:
        with self.assertRaisesRegex(MODULE.VerificationError, message):
            MODULE.verify_rows([self.row(payload)], self.expected, self.brain)

    def test_rejects_malformed_protobuf(self) -> None:
        self.assert_rejected(b"\x80", "malformed payload")

    def test_rejects_text_only_view_file_result(self) -> None:
        payload = _native_payload(self.source, self.media, include_image=False)
        self.assert_rejected(payload, "lacks native image content")

    def test_rejects_missing_payload(self) -> None:
        self.assert_rejected(None, "missing step payload")

    def test_rejects_media_hash_mismatch(self) -> None:
        self.media.write_bytes(MODULE.PNG_SIGNATURE + b"different-image")
        payload = _native_payload(self.source, self.media)
        self.assert_rejected(payload, "source/media hash mismatch")

    def test_accepts_and_verifies_duplicate_image_reads(self) -> None:
        payload = _native_payload(self.source, self.media)
        evidence = MODULE.verify_rows(
            [self.row(payload, 7), self.row(payload, 8)], self.expected, self.brain
        )
        self.assertEqual([7, 8], [image["step"] for image in evidence])
        self.assertEqual(1, len({image["source"] for image in evidence}))

    def test_read_rows_sees_committed_wal_and_excludes_other_step_types(self) -> None:
        database = self.root / "live.db"
        with closing(sqlite3.connect(database)) as setup:
            setup.execute("PRAGMA journal_mode = WAL")
            setup.execute(
                "CREATE TABLE steps (idx INTEGER, step_type INTEGER, status INTEGER, "
                "error_details BLOB, step_payload BLOB, step_format INTEGER)"
            )
            setup.commit()
            setup.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        with closing(sqlite3.connect(database)) as writer:
            writer.execute("PRAGMA journal_mode = WAL")
            writer.execute("PRAGMA wal_autocheckpoint = 0")
            writer.execute("INSERT INTO steps VALUES (1, 7, 3, X'', X'0801', 0)")
            writer.execute("INSERT INTO steps VALUES (2, 132, 3, X'', X'0801', 0)")
            writer.commit()
            rows = MODULE._read_rows(database)
        self.assertEqual([(2, 132, 3, b"", b"\x08\x01", 0)], rows)


if __name__ == "__main__":
    unittest.main()
