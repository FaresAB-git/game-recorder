/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        bg: {
          base: "#0a0b0f",
          surface: "#111319",
          card: "#161922",
          elevated: "#1d212c",
          hover: "#232836",
        },
        line: {
          DEFAULT: "#232734",
          strong: "#323848",
        },
        fg: {
          DEFAULT: "#eceef4",
          muted: "#9aa1b5",
          subtle: "#5f667b",
        },
        accent: {
          DEFAULT: "#7c6cff",
          hover: "#6a59f5",
          soft: "rgba(124,108,255,0.14)",
        },
        lol: {
          DEFAULT: "#c8aa6e",
          soft: "rgba(200,170,110,0.14)",
        },
        valorant: {
          DEFAULT: "#ff4655",
          soft: "rgba(255,70,85,0.14)",
        },
        live: "#ff3b4e",
        success: "#2fd281",
        warning: "#f5b544",
        danger: "#f0525f",
      },
      fontFamily: {
        sans: ["Inter", "Segoe UI", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "Consolas", "ui-monospace", "monospace"],
      },
      boxShadow: {
        card: "0 1px 0 rgba(255,255,255,0.03) inset, 0 8px 24px -12px rgba(0,0,0,0.6)",
        glow: "0 0 0 1px rgba(124,108,255,0.5), 0 8px 30px -8px rgba(124,108,255,0.45)",
      },
      animation: {
        "fade-in": "fadeIn 0.18s ease-out",
        "slide-up": "slideUp 0.22s ease-out",
        "pulse-slow": "pulse 2.4s cubic-bezier(0.4, 0, 0.6, 1) infinite",
      },
      keyframes: {
        fadeIn: {
          "0%": { opacity: "0" },
          "100%": { opacity: "1" },
        },
        slideUp: {
          "0%": { opacity: "0", transform: "translateY(8px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
      },
    },
  },
  plugins: [],
};
