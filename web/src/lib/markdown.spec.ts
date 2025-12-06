import { describe, it, expect } from 'vitest'
import { linkMentions, renderMarkdown } from './markdown'

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

describe('linkMentions', () => {
  it('links a name on its own', () => {
    expect(linkMentions('<p>@alice_01</p>')).toBe(
      '<p><a href="/u/alice_01" class="mention">@alice_01</a></p>',
    )
  })

  it('links a name in a sentence and keeps what surrounds it', () => {
    expect(linkMentions('<p>I agree with @alice_01 here</p>')).toBe(
      '<p>I agree with <a href="/u/alice_01" class="mention">@alice_01</a> here</p>',
    )
  })

  it('leaves an address alone', () => {
    expect(linkMentions('<p>write to alice@example.com</p>')).toBe(
      '<p>write to alice@example.com</p>',
    )
  })

  it('never rewrites inside a tag', () => {
    expect(linkMentions('<a href="/u/@alice_01">x</a>')).toBe('<a href="/u/@alice_01">x</a>')
  })

  it('leaves a name too short to be one alone', () => {
    expect(linkMentions('<p>@ab</p>')).toBe('<p>@ab</p>')
  })

  it('links several names', () => {
    const linked = linkMentions('<p>@alice_01 and @bob_02</p>')
    expect(linked).toContain('href="/u/alice_01"')
    expect(linked).toContain('href="/u/bob_02"')
  })

  it('links a mention in rendered text', () => {
    expect(renderMarkdown('hello @alice_01')).toContain('href="/u/alice_01"')
  })
})
