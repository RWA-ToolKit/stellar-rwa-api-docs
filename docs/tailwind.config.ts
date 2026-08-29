import type { Config } from "tailwindcss";

const config: Config = {
  darkMode: "class",
  content: [
    "./app/**/*.{ts,tsx,mdx}",
    "./components/**/*.{ts,tsx}",
    "./mdx-components.tsx",
  ],
  theme: {
    extend: {
      colors: {
        base: {
          50: "#f6f8fb",
          100: "#e5e7eb",
          200: "#c7cbd4",
          300: "#a5abba",
          600: "#333a49",
          700: "#232834",
          800: "#171b24",
          850: "#11141b",
          900: "#0c0e13",
          950: "#08090c",
        },
        brand: {
          200: "#a7f3d0",
          300: "#6ee7b7",
          400: "#34d399",
          500: "#10b981",
          600: "#059669",
          900: "#064e3b",
        },
        gold: { 300: "#fcd34d", 400: "#fbbf24", 500: "#f59e0b" },
      },
      fontFamily: {
        sans: ["Inter", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "ui-monospace", "monospace"],
      },
    },
  },
  plugins: [],
};

export default config;
