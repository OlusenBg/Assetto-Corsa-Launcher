import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  // Inline component CSS into the JS instead of separate virtual CSS modules.
  // The dev server could serve a stylesheet before its component was compiled,
  // which left parts of the UI unstyled.
  compilerOptions: { css: "injected" },
};
