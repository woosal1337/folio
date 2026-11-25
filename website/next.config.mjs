// STATIC_EXPORT=1 builds the static site for Cloudflare. Without it, the
// build stays a Node server for Coolify on igris.
const staticExport = process.env.STATIC_EXPORT === "1";

const nextConfig = {
    ...(staticExport && { output: "export", images: { unoptimized: true } }),
    reactStrictMode: true,
    poweredByHeader: false,
};

export default nextConfig;
