import { marked } from 'marked'
import DOMPurify from 'dompurify'

marked.setOptions({ breaks: true })

const MENTION = /(^|[^A-Za-z0-9_@])@([A-Za-z][A-Za-z0-9_]{2,31})\b/g

export function linkMentions(html: string): string {
  const parts = html.split(/(<[^>]*>)/)
  return parts
    .map((part, index) =>
      index % 2 === 1
        ? part
        : part.replace(MENTION, (_whole, before: string, name: string) =>
            `${before}<a href="/u/${name}" class="mention">@${name}</a>`,
          ),
    )
    .join('')
}

export function renderMarkdown(source: string): string {
  const html = marked.parse(source, { async: false })
  return DOMPurify.sanitize(linkMentions(html))
}
