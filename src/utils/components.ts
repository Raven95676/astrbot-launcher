import type { ComponentStatus } from '../types';

/**
 * Return the component id required by a given AstrBot version.
 * v4.14.6 and earlier -> "python310", v4.14.7+ -> "python312".
 */
export function requiredPythonComponent(version: string): string {
  const v = version.startsWith('v') ? version.slice(1) : version;
  const parts = v.split('.').map(Number);
  const [major = 0, minor = 0, patch = 0] = parts;

  if (major < 4 || (major === 4 && minor < 14) || (major === 4 && minor === 14 && patch <= 6)) {
    return 'python310';
  }
  return 'python312';
}

/**
 * Check if the Python runtime required for a specific version is installed.
 */
export function isPythonAvailableForVersion(
  version: string,
  components: ComponentStatus[]
): boolean {
  const needed = requiredPythonComponent(version);
  const comp = components.find((c) => c.id === needed);
  return comp?.installed ?? false;
}
