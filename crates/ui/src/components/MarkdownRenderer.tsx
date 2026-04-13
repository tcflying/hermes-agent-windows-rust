import React from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { createHighlighter, Highlighter } from "shiki";

let highlighterPromise: Promise<Highlighter> | null = null;

function getHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = createHighlighter({
      themes: ["github-dark"],
      langs: ["javascript", "typescript", "python", "rust", "go", "java", "bash", "json", "yaml", "html", "css", "markdown"],
    });
  }
  return highlighterPromise;
}

interface CodeProps {
  inline?: boolean;
  className?: string;
  children?: React.ReactNode;
}

function Code({ inline, className, children }: CodeProps) {
  const [highlighter, setHighlighter] = React.useState<Highlighter | null>(null);

  React.useEffect(() => {
    getHighlighter().then(setHighlighter);
  }, []);

  const match = /language-(\w+)/.exec(className || "");
  const lang = match ? match[1] : "";
  const code = String(children).replace(/\n$/, "");

  if (inline || !lang || !highlighter) {
    return (
      <code className={className} style={{
        background: "var(--bg-tertiary)",
        padding: "2px 6px",
        borderRadius: "4px",
        fontFamily: "var(--font-mono)",
        fontSize: "0.9em",
      }}>
        {children}
      </code>
    );
  }

  try {
    const html = highlighter.codeToHtml(code, { lang, theme: "github-dark" });
    return (
      <div
        style={{ margin: "12px 0", borderRadius: "8px", overflow: "hidden" }}
        dangerouslySetInnerHTML={{ __html: html }}
      />
    );
  } catch {
    return (
      <code className={className} style={{
        background: "var(--bg-tertiary)",
        padding: "2px 6px",
        borderRadius: "4px",
        fontFamily: "var(--font-mono)",
        fontSize: "0.9em",
      }}>
        {children}
      </code>
    );
  }
}

interface MarkdownRendererProps {
  content: string;
}

export function MarkdownRenderer({ content }: MarkdownRendererProps) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        code: Code as any,
        pre: ({ children }) => <>{children}</>,
      }}
    >
      {content}
    </ReactMarkdown>
  );
}
