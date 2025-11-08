export type ValidationResult = { ok: true } | { ok: false; error: string }

export function validateUsername(raw: string): ValidationResult {
  const value = raw.trim()
  if (value.length < 3) return { ok: false, error: 'Username must be at least 3 characters.' }
  if (value.length > 32) return { ok: false, error: 'Username must be at most 32 characters.' }
  if (!/^[A-Za-z]/.test(value)) return { ok: false, error: 'Username must start with a letter.' }
  if (!/^[A-Za-z0-9_-]+$/.test(value)) return { ok: false, error: 'Username has invalid characters.' }
  return { ok: true }
}

export function validateEmail(raw: string): ValidationResult {
  const value = raw.trim()
  if (value.length === 0) return { ok: false, error: 'Email is required.' }
  const at = value.indexOf('@')
  if (at <= 0) return { ok: false, error: 'Email must contain a local part and @.' }
  const domain = value.slice(at + 1)
  if (domain.length === 0) return { ok: false, error: 'Email domain is required.' }
  if (!domain.includes('.')) return { ok: false, error: 'Email domain must contain a dot.' }
  return { ok: true }
}

export function validatePassword(raw: string): ValidationResult {
  if (raw.length < 8) return { ok: false, error: 'Password must be at least 8 characters.' }
  return { ok: true }
}
