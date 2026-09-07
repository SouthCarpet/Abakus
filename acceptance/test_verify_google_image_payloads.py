from __future__ import annotations

import hashlib
import importlib.util
import sys
import tempfile
import unittest
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

    def row(self, payload: bytes | None) -> tuple[object, ...]:
        return (7, 132, 3, b"", payload, 0)

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


if __name__ == "__main__":
    unittest.main()
