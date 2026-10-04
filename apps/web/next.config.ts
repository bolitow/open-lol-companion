import type { NextConfig } from "next";

const config: NextConfig = {
  // `@olc/shared` est publié en TypeScript source dans le monorepo.
  transpilePackages: ["@olc/shared"],
  poweredByHeader: false,
  reactStrictMode: true,
  async redirects() {
    return [{ source: "/", destination: "/fr", permanent: false }];
  },
};

export default config;
