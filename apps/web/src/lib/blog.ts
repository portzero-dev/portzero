export interface BlogPost {
  slug: string;
  title: string;
  description: string;
  date: string;
  author: string;
  tags: string[];
  readingTime: string;
}

export const posts: BlogPost[] = [
  {
    slug: "introducing-portzero",
    title: "Introducing PortZero: Why I Built a New Local Dev Proxy",
    description:
      "The story behind PortZero — a single Rust binary that gives your local dev servers stable URLs, traffic inspection, and more.",
    date: "2026-03-14",
    author: "David Viejo",
    tags: ["announcement", "rust", "developer-tools"],
    readingTime: "6 min read",
  },
];

export function getAllPosts(): BlogPost[] {
  return posts.sort(
    (a, b) => new Date(b.date).getTime() - new Date(a.date).getTime()
  );
}

export function getPost(slug: string): BlogPost | undefined {
  return posts.find((p) => p.slug === slug);
}
