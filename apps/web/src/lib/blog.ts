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
    title: "Why I Built PortZero, a New Local Dev Proxy",
    description:
      "PortZero is a single Rust binary that replaces 5 separate dev tools. Get stable .localhost URLs, traffic inspection, mocking, replay, and sub-ms proxy overhead.",
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
