import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["scripts/**/*.test.ts", "oxlint-plugins/**/*.test.ts", "packages/*/src/**/*.test.ts"],
  },
});
