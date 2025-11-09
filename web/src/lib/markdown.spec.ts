import { describe, it, expect } from 'vitest'
import { renderMarkdown } from './markdown'

describe('renderMarkdown', () => {
  it('renders bold text', () => {
    expect(renderMarkdown('**hello**')).toContain('<strong>hello</strong>')
  })

  it('renders a link', () => {
    const html = renderMarkdown('[site](https://example.com)')
    expect(html).toContain('<a href="https://example.com">site</a>')
  })

  it('renders a list', () => {
    const html = renderMarkdown('- one\n- two')
    expect(html).toContain('<li>one</li>')
    expect(html).toContain('<li>two</li>')
  })

  it('strips a script tag', () => {
    const html = renderMarkdown('hello<script>alert(1)</script>')
    expect(html).not.toContain('<script')
    expect(html).not.toContain('alert(1)')
  })

  it('strips an event-handler attribute from an inline image', () => {
    const html = renderMarkdown('<img src=x onerror="alert(1)">')
    expect(html).not.toContain('onerror')
  })

  it('strips a javascript: link target', () => {
    const html = renderMarkdown('[click me](javascript:alert(1))')
    expect(html).not.toContain('javascript:')
  })

  it('leaves plain text untouched by sanitization', () => {
    expect(renderMarkdown('just plain text')).toContain('just plain text')
  })
})
