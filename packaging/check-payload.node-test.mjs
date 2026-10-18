import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { checkPayload, inspectPe } from './check-payload.mjs';

// Oracle: Microsoft PE/COFF format. One x64 PE32+ image section at RVA 0x1000.
function peFixture() {
  const bytes = Buffer.alloc(1024);
  bytes.write('MZ');
  bytes.writeUInt32LE(64, 60);
  bytes.writeUInt32LE(0x4550, 64);
  bytes.writeUInt16LE(0x8664, 68);
  bytes.writeUInt16LE(1, 70);
  bytes.writeUInt16LE(240, 84);
  bytes.writeUInt16LE(0x22, 86);
  bytes.writeUInt16LE(0x20b, 88);
  bytes.writeUInt32LE(0x1000, 104);
  bytes.writeUInt32LE(0x2000, 144);
  bytes.writeUInt32LE(512, 148);
  bytes.writeUInt32LE(16, 196);
  bytes.writeUInt32LE(512, 336);
  bytes.writeUInt32LE(0x1000, 340);
  bytes.writeUInt32LE(512, 344);
  bytes.writeUInt32LE(512, 348);
  return bytes;
}

test('valid minimal x64 PE reports its architecture', () => {
  assert.equal(inspectPe(peFixture()).machine, 'x64');
});

test('truncated raw section is rejected before packaging', () => {
  assert.throws(() => inspectPe(peFixture().subarray(0, 800)), /section/i);
});

test('32-bit optional header in x64 COFF is rejected', () => {
  const bytes = peFixture();
  bytes.writeUInt16LE(0x10b, 88);
  assert.throws(() => inspectPe(bytes), /PE32\+/);
});

test('wrong architecture is rejected', () => {
  const bytes = peFixture();
  bytes.writeUInt16LE(0xaa64, 68);
  assert.throws(() => inspectPe(bytes), /x64/);
});

test('x86 architecture is rejected', () => {
  const bytes = peFixture();
  bytes.writeUInt16LE(0x14c, 68);
  assert.throws(() => inspectPe(bytes), /x64/);
});

test('non-PE data is rejected', () => {
  assert.throws(() => inspectPe(Buffer.from('damaged')), /DOS header/);
});

test('delay imports require a new dependency review', () => {
  const bytes = peFixture();
  bytes.writeUInt32LE(0x1100, 304);
  bytes.writeUInt32LE(64, 308);
  assert.throws(() => inspectPe(bytes), /delay imports/i);
});

test('arbitrary valid x64 DLL fails the trusted PDFium pin', () => {
  // Oracle: pdfium-pin.json is tied to the independently verified archive.
  const folder = mkdtempSync(join(tmpdir(), 'abakus-payload-'));
  try {
    const exe = join(folder, 'abakus.exe');
    const dll = join(folder, 'pdfium.dll');
    writeFileSync(exe, peFixture());
    writeFileSync(dll, peFixture());
    assert.throws(() => checkPayload(exe, dll), /PDFium SHA256/);
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
});

test('missing executable fails with the named input path', () => {
  const folder = mkdtempSync(join(tmpdir(), 'abakus-missing-'));
  try {
    assert.throws(() => checkPayload(join(folder, 'missing.exe'), join(folder, 'pdfium.dll')), /missing.exe/);
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
});

function importedFixture(name) {
  const bytes = peFixture();
  bytes.writeUInt32LE(0x1000, 208);
  bytes.writeUInt32LE(40, 212);
  bytes.writeUInt32LE(0x1040, 524);
  bytes.write(name, 576, 'ascii');
  return bytes;
}

test('known Windows import is reported by name', () => {
  // Oracle: verified inventory of shipped DLLs, Microsoft PE import descriptor.
  assert.deepEqual(inspectPe(importedFixture('KERNEL32.dll')).imports, ['kernel32.dll']);
});

test('new external runtime import blocks packaging pending dependency review', () => {
  assert.throws(() => inspectPe(importedFixture('VCRUNTIME140.dll')), /Unaudited runtime import: vcruntime140.dll/);
});

test('import name outside the image blocks packaging', () => {
  const bytes = importedFixture('KERNEL32.dll');
  bytes.writeUInt32LE(0x2000, 524);
  assert.throws(() => inspectPe(bytes), /Import RVA outside/);
});

test('unterminated import directory blocks packaging', () => {
  const bytes = importedFixture('KERNEL32.dll');
  bytes.writeUInt32LE(20, 212);
  assert.throws(() => inspectPe(bytes), /Unterminated import table/);
});

test('PE header offset beyond the file blocks packaging', () => {
  const bytes = peFixture();
  bytes.writeUInt32LE(4000, 60);
  assert.throws(() => inspectPe(bytes), /Missing PE header/);
});
