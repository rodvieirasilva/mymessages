import { describe, expect, it } from 'vitest';
import { formatBytes, formatDate, getVersionLabel } from './format';

describe('format helpers', () => {
  it('formats dates as YYYY-MM-DD HH:MM:SS', () => {
    expect(formatDate(1700000000)).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
  });

  it('formats byte sizes cleanly', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1024)).toBe('1.00 KB');
    expect(formatBytes(1048576)).toBe('1.00 MB');
  });

  it('returns version label string', () => {
    expect(getVersionLabel()).toContain('v');
  });
});
