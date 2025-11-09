import { marked } from 'marked'
import DOMPurify from 'dompurify'

marked.setOptions({ breaks: true })

export function renderMarkdown(source: string): string {
  const html = marked.parse(source, { async: false })
  return DOMPurify.sanitize(html)
}
