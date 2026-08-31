import { describe, expect, it } from 'vitest'
import { fromSkParts, maskIban } from './iban'

describe('iban', () => {
  it('builds a valid IBAN from SK parts, mirroring parser::iban::from_sk_parts', () => {
    expect(fromSkParts('1100', '000000', '0012345678')).toBe('SK4411000000000012345678')
  })
  it('zero-pads short parts the same way as the Rust helper', () => {
    expect(fromSkParts('1200', '19', '8742637541')).toBe('SK3112000000198742637541')
  })
  it('masks the middle of an IBAN', () => {
    expect(maskIban('SK4411000000000012345678')).toBe('SK44...5678')
  })
})
