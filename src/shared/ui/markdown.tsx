import ReactMarkdown from "react-markdown";
import type { Components } from "react-markdown";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import remarkGfm from "remark-gfm";

import { cn } from "@/shared/lib/utils";

/**
 * Strict sanitiser schema for LLM-generated markdown. Tightens
 * `defaultSchema` so:
 *
 *   - `<script>` and `<style>` are stripped (the default already
 *     does this; we keep it explicit for §8.6 audit-grep).
 *   - Every `href` / `src` must be `http(s):`, `mailto:`, `data:`
 *     (for embedded images), or one of our own deep-link schemes.
 *     `javascript:` and `data:text/html` are rejected.
 *   - `target` is forced to `_blank` and `rel="noopener noreferrer"`
 *     so a clickjacked LLM link cannot script the rendering page.
 *
 * `docs/CODE_STYLE.md` §8.6: "LLM markdown/HTML output runs through
 * a sanitiser before render." This is that sanitiser.
 */
const SANITIZE_SCHEMA = {
  ...defaultSchema,
  protocols: {
    ...defaultSchema.protocols,
    href: ["http", "https", "mailto", "attune", "obsidian"],
    src: ["http", "https", "data"],
  },
  attributes: {
    ...defaultSchema.attributes,
    a: [
      ...((defaultSchema.attributes && defaultSchema.attributes.a) || []),
      ["target", "_blank"],
      ["rel", "noopener noreferrer"],
    ],
  },
};

interface Props {
  children: string;
  className?: string;
}

/**
 * Renders LLM-produced markdown with consistent in-app styling.
 *
 * We keep this small and intentional rather than reaching for
 * @tailwindcss/typography — the prose plugin's defaults clash with
 * Attune's compact dark/light palette in a few places (headings too
 * big for an inline panel, code blocks with their own background).
 * The element overrides below give the agent output enough structure
 * to read well (paragraph spacing, bullets, bold, inline code,
 * headings, links, simple tables) without a global typography reset.
 *
 * GitHub-flavored markdown is enabled via remark-gfm so the LLMs'
 * common output (task lists, tables, strikethrough) renders correctly.
 */
export function Markdown({ children, className }: Props) {
  return (
    <div className={cn("text-sm leading-relaxed text-foreground", className)}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[[rehypeSanitize, SANITIZE_SCHEMA]]}
        components={COMPONENTS}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
}

const COMPONENTS: Components = {
  p: ({ children }) => <p className="mb-3 last:mb-0">{children}</p>,
  ul: ({ children }) => (
    <ul className="mb-3 ml-5 list-disc space-y-1 marker:text-muted-foreground last:mb-0">
      {children}
    </ul>
  ),
  ol: ({ children }) => (
    <ol className="mb-3 ml-5 list-decimal space-y-1 marker:text-muted-foreground last:mb-0">
      {children}
    </ol>
  ),
  li: ({ children }) => <li className="pl-1">{children}</li>,
  h1: ({ children }) => (
    <h1 className="mb-3 mt-4 text-base font-semibold first:mt-0">{children}</h1>
  ),
  h2: ({ children }) => (
    <h2 className="mb-2 mt-4 text-sm font-semibold first:mt-0">{children}</h2>
  ),
  h3: ({ children }) => (
    <h3 className="mb-2 mt-3 text-sm font-semibold first:mt-0">{children}</h3>
  ),
  h4: ({ children }) => (
    <h4 className="mb-1 mt-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground first:mt-0">
      {children}
    </h4>
  ),
  strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
  em: ({ children }) => <em className="italic">{children}</em>,
  a: ({ href, children }) => (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="text-primary underline underline-offset-2 hover:text-primary/80"
    >
      {children}
    </a>
  ),
  blockquote: ({ children }) => (
    <blockquote className="mb-3 border-l-2 border-border pl-3 text-muted-foreground last:mb-0">
      {children}
    </blockquote>
  ),
  hr: () => <hr className="my-4 border-border" />,
  code: ({ className: codeClassName, children, ...props }) => {
    // react-markdown 10 hands the language hint via className (e.g.
    // "language-rust"). Block code arrives wrapped in <pre><code>;
    // inline code is just <code>. The `inline` prop was removed in
    // v10 — detection now goes by presence of a class (block) vs not
    // (inline). We treat anything with a language- prefix as block.
    const isBlock =
      typeof codeClassName === "string" && codeClassName.startsWith("language-");
    if (isBlock) {
      return (
        <code
          className="block whitespace-pre-wrap break-words font-mono text-2xs"
          {...props}
        >
          {children}
        </code>
      );
    }
    return (
      <code className="rounded bg-muted px-1 py-0.5 font-mono text-2xs" {...props}>
        {children}
      </code>
    );
  },
  pre: ({ children }) => (
    <pre className="mb-3 overflow-x-auto rounded-md border border-border bg-muted/60 p-3 last:mb-0">
      {children}
    </pre>
  ),
  table: ({ children }) => (
    <div className="mb-3 overflow-x-auto last:mb-0">
      <table className="w-full border-collapse text-xs">{children}</table>
    </div>
  ),
  thead: ({ children }) => <thead className="border-b border-border">{children}</thead>,
  th: ({ children }) => (
    <th className="px-2 py-1 text-left font-semibold">{children}</th>
  ),
  td: ({ children }) => (
    <td className="border-b border-border/50 px-2 py-1 align-top">{children}</td>
  ),
  input: ({ checked, disabled, type }) =>
    type === "checkbox" ? (
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        readOnly
        className="mr-1.5 align-middle accent-primary"
      />
    ) : null,
};
