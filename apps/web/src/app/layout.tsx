import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "PortZero - Local Dev Reverse Proxy & Traffic Inspector",
  description:
    "Assign stable .localhost URLs to your dev servers, capture all HTTP traffic for inspection, and get request replay, mocking, and network simulation -- all from a single Rust binary.",
  openGraph: {
    title: "PortZero - Local Dev Reverse Proxy & Traffic Inspector",
    description:
      "Assign stable .localhost URLs to your dev servers. Traffic inspector, request replay, mocking, and more.",
    type: "website",
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <body className="bg-zinc-950 text-zinc-100 antialiased">{children}</body>
    </html>
  );
}
