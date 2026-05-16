import MarkdownIt from 'markdown-it'
import hljs from 'highlight.js'
import katex from 'katex'

const md = new MarkdownIt({
  html: false,
  linkify: true,
  typographer: true,
  highlight(str: string, lang: string) {
    if (lang && hljs.getLanguage(lang)) {
      try {
        const highlighted = hljs.highlight(str, { language: lang }).value
        return `<pre class="hljs-code-block" data-lang="${lang}"><code>${highlighted}</code></pre>`
      } catch (_) {}
    }
    const escaped = md.utils.escapeHtml(str)
    return `<pre class="hljs-code-block"><code>${escaped}</code></pre>`
  },
})

// 行内展示公式 $$...$$ (优先级高于单 $)
md.inline.ruler.after('escape', 'math_inline_display', (state, silent) => {
  if (state.src[state.pos] !== '$' || state.src[state.pos + 1] !== '$') return false

  const start = state.pos + 2
  let end = start
  while (end + 1 < state.posMax) {
    if (state.src[end] === '$' && state.src[end + 1] === '$') break
    end++
  }
  if (end + 1 >= state.posMax) return false
  if (!silent) {
    const token = state.push('math_inline_display', 'math', 0)
    token.content = state.src.slice(start, end)
  }
  state.pos = end + 2
  return true
})

md.renderer.rules.math_inline_display = (tokens, idx) => {
  try {
    return `<div class="katex-block">${katex.renderToString(tokens[idx].content, { throwOnError: false, displayMode: true })}</div>`
  } catch (_) {
    return `<pre><code>${md.utils.escapeHtml(tokens[idx].content)}</code></pre>`
  }
}

// 行内数学公式 $...$
md.inline.ruler.after('escape', 'math_inline', (state, silent) => {
  if (state.src[state.pos] !== '$') return false
  if (state.src[state.pos + 1] === '$') return false

  const start = state.pos + 1
  let end = start
  while (end < state.posMax && state.src[end] !== '$') {
    if (state.src[end] === '\\') end++
    end++
  }
  if (end >= state.posMax) return false
  if (!silent) {
    const token = state.push('math_inline', 'math', 0)
    token.content = state.src.slice(start, end)
  }
  state.pos = end + 1
  return true
})

md.renderer.rules.math_inline = (tokens, idx) => {
  try {
    return katex.renderToString(tokens[idx].content, { throwOnError: false })
  } catch (_) {
    return `<code>${md.utils.escapeHtml(tokens[idx].content)}</code>`
  }
}

// 块级数学公式 $$...$$ (多行)
md.block.ruler.before('fence', 'math_block', (state, startLine, endLine, silent) => {
  const startPos = state.bMarks[startLine] + state.tShift[startLine]
  if (state.src.slice(startPos, startPos + 2) !== '$$') return false
  if (silent) return true

  const lineEnd = state.eMarks[startLine]
  const firstLineContent = state.src.slice(startPos + 2, lineEnd).trim()

  // 单行 $$...$$ 情况
  if (firstLineContent.endsWith('$$') && firstLineContent.length > 2) {
    const content = firstLineContent.slice(0, -2).trim()
    const token = state.push('math_block', 'math', 0)
    token.content = content
    token.map = [startLine, startLine + 1]
    state.line = startLine + 1
    return true
  }

  let nextLine = startLine + 1
  while (nextLine < endLine) {
    const pos = state.bMarks[nextLine] + state.tShift[nextLine]
    if (state.src.slice(pos, pos + 2) === '$$') break
    nextLine++
  }

  const content = firstLineContent
    ? firstLineContent + '\n' + state.getLines(startLine + 1, nextLine, state.tShift[startLine], false).trim()
    : state.getLines(startLine + 1, nextLine, state.tShift[startLine], false).trim()
  const token = state.push('math_block', 'math', 0)
  token.content = content
  token.map = [startLine, nextLine + 1]
  state.line = nextLine + 1
  return true
})

md.renderer.rules.math_block = (tokens, idx) => {
  try {
    return `<div class="katex-block">${katex.renderToString(tokens[idx].content, { throwOnError: false, displayMode: true })}</div>`
  } catch (_) {
    return `<pre><code>${md.utils.escapeHtml(tokens[idx].content)}</code></pre>`
  }
}

export function renderMarkdown(text: string): string {
  return md.render(text)
}

export { md }
