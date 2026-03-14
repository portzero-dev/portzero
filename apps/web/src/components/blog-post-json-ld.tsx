import type { BlogPost } from "@/lib/blog";

export function BlogPostJsonLd({ post }: { post: BlogPost }) {
  const url = `https://goport0.dev/blog/${post.slug}`;

  const jsonLd = {
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "BlogPosting",
        "@id": `${url}#article`,
        headline: post.title,
        description: post.description,
        datePublished: post.date,
        dateModified: post.date,
        author: {
          "@type": "Person",
          "@id": "https://goport0.dev/#author",
          name: "David Viejo",
          url: "https://x.com/davidviejodev",
          sameAs: ["https://x.com/davidviejodev"],
        },
        publisher: {
          "@type": "Organization",
          "@id": "https://goport0.dev/#organization",
          name: "PortZero",
          url: "https://goport0.dev",
          logo: {
            "@type": "ImageObject",
            url: "https://goport0.dev/icon.svg",
          },
        },
        mainEntityOfPage: {
          "@type": "WebPage",
          "@id": url,
        },
        keywords: post.tags.join(", "),
        wordCount: parseInt(post.readingTime) * 200,
        inLanguage: "en-US",
        isPartOf: {
          "@type": "Blog",
          "@id": "https://goport0.dev/blog#blog",
          name: "PortZero Blog",
          publisher: {
            "@id": "https://goport0.dev/#organization",
          },
        },
      },
      {
        "@type": "BreadcrumbList",
        "@id": `${url}#breadcrumb`,
        itemListElement: [
          {
            "@type": "ListItem",
            position: 1,
            name: "Home",
            item: "https://goport0.dev",
          },
          {
            "@type": "ListItem",
            position: 2,
            name: "Blog",
            item: "https://goport0.dev/blog",
          },
          {
            "@type": "ListItem",
            position: 3,
            name: post.title,
            item: url,
          },
        ],
      },
    ],
  };

  return (
    <script
      type="application/ld+json"
      dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
    />
  );
}
