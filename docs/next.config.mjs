import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import createMDX from "@next/mdx";

const redirectsMap = JSON.parse(
  readFileSync(fileURLToPath(new URL("./redirects.json", import.meta.url)), "utf8"),
);

/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "standalone",
  reactStrictMode: true,
  pageExtensions: ["ts", "tsx", "mdx"],
  // Renamed/removed doc pages are tracked in redirects.json rather than
  // hardcoded here, so adding a redirect never requires touching this file.
  async redirects() {
    return redirectsMap.redirects.map(({ source, destination, permanent }) => ({
      source,
      destination,
      permanent,
    }));
  },
};

const withMDX = createMDX({
  extension: /\.mdx?$/,
});

export default withMDX(nextConfig);
