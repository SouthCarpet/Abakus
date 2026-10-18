import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const windowsDlls = new Set([
  'advapi32.dll', 'bcryptprimitives.dll', 'comctl32.dll', 'crypt32.dll',
  'dwmapi.dll', 'gdi32.dll', 'iphlpapi.dll', 'kernel32.dll', 'ntdll.dll',
  'ole32.dll', 'oleaut32.dll', 'shell32.dll', 'shlwapi.dll', 'user32.dll', 'ws2_32.dll',
  'api-ms-win-core-synch-l1-2-0.dll',
  ...['math', 'string', 'runtime', 'convert', 'environment', 'stdio', 'heap',
    'utility', 'time', 'locale'].map(name => `api-ms-win-crt-${name}-l1-1-0.dll`),
]);

function range(bytes, offset, size, label) {
  if (offset < 0 || size < 0 || offset + size > bytes.length) {
    throw new Error(`Truncated ${label}`);
  }
}

function sectionsOf(bytes, optional, optionalSize, count) {
  if (count === 0) throw new Error('Missing PE sections');
  range(bytes, optional + optionalSize, count * 40, 'section table');
  return Array.from({ length: count }, (_, index) => {
    const offset = optional + optionalSize + index * 40;
    const size = bytes.readUInt32LE(offset + 16);
    const raw = bytes.readUInt32LE(offset + 20);
    range(bytes, raw, size, 'raw section');
    return { rva: bytes.readUInt32LE(offset + 12), raw, size };
  });
}

function fileOffset(sections, rva, size) {
  const section = sections.find(item => rva >= item.rva && rva + size <= item.rva + item.size);
  if (!section) throw new Error('Import RVA outside raw sections');
  return section.raw + rva - section.rva;
}

function importName(bytes, sections, rva) {
  const offset = fileOffset(sections, rva, 1);
  const end = bytes.indexOf(0, offset);
  if (end < 0 || end - offset > 260) throw new Error('Invalid import name');
  fileOffset(sections, rva, end - offset + 1);
  const name = bytes.toString('ascii', offset, end).toLowerCase();
  if (!windowsDlls.has(name)) throw new Error(`Unaudited runtime import: ${name}`);
  return name;
}

function importsOf(bytes, optional, sections) {
  const rva = bytes.readUInt32LE(optional + 120);
  const size = bytes.readUInt32LE(optional + 124);
  if (rva === 0 && size === 0) return [];
  const offset = fileOffset(sections, rva, size);
  const imports = [];
  for (let index = 0; index + 20 <= size; index += 20) {
    const descriptor = bytes.subarray(offset + index, offset + index + 20);
    if (descriptor.every(byte => byte === 0)) return imports;
    imports.push(importName(bytes, sections, descriptor.readUInt32LE(12)));
  }
  throw new Error('Unterminated import table');
}

export function inspectPe(bytes) {
  if (bytes.length < 64 || bytes.toString('ascii', 0, 2) !== 'MZ') {
    throw new Error('Missing DOS header');
  }
  const pe = bytes.readUInt32LE(60);
  if (pe + 24 > bytes.length || bytes.readUInt32LE(pe) !== 0x4550) {
    throw new Error('Missing PE header');
  }
  if (bytes.readUInt16LE(pe + 4) !== 0x8664) throw new Error('Expected x64 PE');
  const optional = pe + 24;
  const optionalSize = bytes.readUInt16LE(pe + 20);
  range(bytes, optional, optionalSize, 'optional header');
  if (optionalSize < 240 || bytes.readUInt16LE(optional) !== 0x20b) {
    throw new Error('Expected PE32+ optional header');
  }
  if (bytes.readUInt32LE(optional + 108) < 16) throw new Error('Missing PE data directories');
  if (bytes.readUInt32LE(optional + 216) || bytes.readUInt32LE(optional + 220)) {
    throw new Error('Unaudited delay imports');
  }
  const sections = sectionsOf(bytes, optional, optionalSize, bytes.readUInt16LE(pe + 6));
  range(bytes, 0, bytes.readUInt32LE(optional + 60), 'PE headers');
  return { machine: 'x64', imports: importsOf(bytes, optional, sections) };
}

function inspectFile(path) {
  const bytes = readFileSync(path);
  try {
    return { ...inspectPe(bytes), sha256: createHash('sha256').update(bytes).digest('hex') };
  } catch (error) {
    throw new Error(`${path}: ${error.message}`, { cause: error });
  }
}

export function checkPayload(exePath, pdfiumPath) {
  const exe = inspectFile(exePath);
  const pdfium = inspectFile(pdfiumPath);
  const pin = JSON.parse(readFileSync(new URL('./pdfium-pin.json', import.meta.url), 'utf8'));
  const archivePin = readFileSync(new URL('../scripts/pdfium.sha256', import.meta.url), 'utf8').trim();
  if (archivePin !== pin.archiveSha256) throw new Error('PDFium archive pin changed; review DLL provenance');
  if (pdfium.sha256 !== pin.dllSha256) throw new Error(`${pdfiumPath}: PDFium SHA256 differs from trusted archive member`);
  return { exe, pdfium };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const result = checkPayload(process.argv[2], process.argv[3]);
  process.stdout.write(JSON.stringify(result, null, 2) + '\n');
}
