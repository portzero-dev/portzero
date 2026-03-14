import Link from "next/link";
import { ArrowLeft } from "lucide-react";

export default function BlogLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="min-h-screen">
      <nav className="fixed top-0 left-0 right-0 z-50 border-b border-zinc-800/50 bg-zinc-950/80 backdrop-blur-xl">
        <div className="mx-auto flex h-16 max-w-3xl items-center justify-between px-6">
          <Link
            href="/"
            className="flex items-center gap-2 text-zinc-400 transition-colors hover:text-white"
          >
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-violet-primary font-bold text-sm text-white">
              PZ
            </div>
            <span className="text-lg font-semibold text-white">PortZero</span>
          </Link>
          <div className="flex items-center gap-6">
            <Link
              href="/blog"
              className="text-sm text-zinc-400 transition-colors hover:text-white"
            >
              Blog
            </Link>
            <Link
              href="/docs"
              className="text-sm text-zinc-400 transition-colors hover:text-white"
            >
              Docs
            </Link>
            <Link
              href="https://github.com/portzero-dev/portzero"
              className="text-sm text-zinc-400 transition-colors hover:text-white"
              target="_blank"
              rel="noopener noreferrer"
            >
              GitHub
            </Link>
          </div>
        </div>
      </nav>
      <main className="pt-16">
        <div className="mx-auto max-w-3xl px-6 py-16">{children}</div>
      </main>
      <footer className="border-t border-zinc-800/50 py-8">
        <div className="mx-auto max-w-3xl px-6">
          <Link
            href="/"
            className="inline-flex items-center gap-1 text-sm text-zinc-500 transition-colors hover:text-zinc-300"
          >
            <ArrowLeft className="h-3 w-3" />
            Back to PortZero
          </Link>
        </div>
      </footer>
    </div>
  );
}
