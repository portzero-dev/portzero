import { codeToHtml } from "shiki";

interface CodeBlockProps {
  code: string;
  lang?: string;
  filename?: string;
}

export async function CodeBlock({
  code,
  lang = "shellscript",
  filename,
}: CodeBlockProps) {
  const html = await codeToHtml(code.trim(), {
    lang,
    theme: "vesper",
  });

  return (
    <div className="code-block">
      {filename && (
        <div className="flex items-center gap-2 border-b border-zinc-800 px-4 py-2.5">
          <span className="text-xs text-zinc-500">{filename}</span>
        </div>
      )}
      <div dangerouslySetInnerHTML={{ __html: html }} />
    </div>
  );
}
