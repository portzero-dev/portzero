import Link from "next/link";
import { getAllPosts } from "@/lib/blog";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Blog - PortZero",
  description:
    "Updates, tutorials, and behind-the-scenes from the PortZero project.",
};

export default function BlogIndex() {
  const posts = getAllPosts();

  return (
    <>
      <h1 className="text-4xl font-bold tracking-tight">Blog</h1>
      <p className="mt-2 text-lg text-zinc-400">
        Updates, tutorials, and behind-the-scenes from the PortZero project.
      </p>

      <div className="mt-12 space-y-10">
        {posts.map((post) => (
          <article key={post.slug} className="group">
            <Link href={`/blog/${post.slug}`} className="block">
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
              <h2 className="mt-2 text-2xl font-semibold tracking-tight text-zinc-100 transition-colors group-hover:text-violet-400">
                {post.title}
              </h2>
              <p className="mt-2 text-zinc-400 leading-relaxed">
                {post.description}
              </p>
              <div className="mt-3 flex gap-2">
                {post.tags.map((tag) => (
                  <span
                    key={tag}
                    className="rounded-full bg-zinc-800/60 px-2.5 py-0.5 text-xs text-zinc-400"
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </Link>
          </article>
        ))}
      </div>
    </>
  );
}
