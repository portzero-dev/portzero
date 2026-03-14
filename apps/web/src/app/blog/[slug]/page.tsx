import { notFound } from "next/navigation";
import { getAllPosts, getPost } from "@/lib/blog";
import { BlogPostJsonLd } from "@/components/blog-post-json-ld";
import type { Metadata } from "next";

export function generateStaticParams() {
  return getAllPosts().map((post) => ({ slug: post.slug }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>;
}): Promise<Metadata> {
  const { slug } = await params;
  const post = getPost(slug);
  if (!post) return {};
  return {
    title: `${post.title} | PortZero`,
    description: post.description,
    alternates: {
      canonical: `https://goport0.dev/blog/${post.slug}`,
    },
    openGraph: {
      title: post.title,
      description: post.description,
      url: `https://goport0.dev/blog/${post.slug}`,
      siteName: "PortZero",
      type: "article",
      publishedTime: post.date,
      modifiedTime: post.date,
      authors: [post.author],
    },
    twitter: {
      card: "summary_large_image",
      title: post.title,
      description: post.description,
      creator: "@davidviejodev",
      site: "@davidviejodev",
    },
  };
}

export default async function BlogPost({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const post = getPost(slug);
  if (!post) notFound();

  const PostContent = (await import(`../posts/${slug}`)).default;

  return (
    <>
      <BlogPostJsonLd post={post} />
      <article className="blog-content">
        <header className="mb-12">
          <div className="flex items-center gap-3 text-sm text-zinc-500">
            <time dateTime={post.date}>
              {new Date(post.date).toLocaleDateString("en-US", {
                year: "numeric",
                month: "long",
                day: "numeric",
              })}
            </time>
            <span>&middot;</span>
            <span>{post.readingTime}</span>
          </div>
          <h1 className="mt-3 text-4xl font-bold tracking-tight leading-tight">
            {post.title}
          </h1>
          <p className="mt-4 text-lg text-zinc-400 leading-relaxed">
            {post.description}
          </p>
          <div className="mt-4 flex items-center gap-3">
            <a
              href="https://x.com/davidviejodev"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-3 transition-colors hover:text-zinc-200"
            >
              <div className="flex h-8 w-8 items-center justify-center rounded-full bg-violet-600 text-xs font-bold text-white">
                DV
              </div>
              <span className="text-sm text-zinc-400">{post.author}</span>
            </a>
          </div>
        </header>
        <PostContent />
      </article>
    </>
  );
}
