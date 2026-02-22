import { Sidebar } from "@/components/sidebar";

export default function DocsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="min-h-screen">
      <Sidebar />
      <main className="lg:pl-64">
        <div className="mx-auto max-w-3xl px-6 py-16 lg:px-12">
          <article className="docs-content">{children}</article>
        </div>
      </main>
    </div>
  );
}
