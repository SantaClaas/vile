import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: [
      "scripts/**/*.test.ts",
      "oxlint-plugins/**/*.test.ts",
      "packages/*/src/**/*.test.ts",
      "spikes/*/runner/**/*.test.ts",
      // The fixtures' own tests must pass unmutated, as vile's baseline needs (see fixtures/README.md).
      "fixtures/**/*.test.ts",
    ],
  },
});
