const MINUTE = 60
const HOUR = MINUTE * 60
const DAY = HOUR * 24
const MONTH = DAY * 30
const YEAR = DAY * 365

function plural(count: number, unit: string) {
  return `${count} ${unit}${count === 1 ? '' : 's'} ago`
}

export function readableWhen(iso: string, now: Date = new Date()): string {
  const written = new Date(iso)
  if (Number.isNaN(written.getTime())) return iso
  const seconds = Math.floor((now.getTime() - written.getTime()) / 1000)
  if (seconds < 0) return 'just now'
  if (seconds < MINUTE) return 'just now'
  if (seconds < HOUR) return plural(Math.floor(seconds / MINUTE), 'minute')
  if (seconds < DAY) return plural(Math.floor(seconds / HOUR), 'hour')
  if (seconds < MONTH) return plural(Math.floor(seconds / DAY), 'day')
  if (seconds < YEAR) return plural(Math.floor(seconds / MONTH), 'month')
  return plural(Math.floor(seconds / YEAR), 'year')
}

export function exactWhen(iso: string): string {
  const written = new Date(iso)
  if (Number.isNaN(written.getTime())) return iso
  return written.toISOString().replace('T', ' ').slice(0, 16)
}
