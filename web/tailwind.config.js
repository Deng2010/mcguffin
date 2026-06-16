/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  safelist: [
    'dark:shadow-xl',
    'dark:shadow-black/50',
    'dark:ring-1',
    'dark:ring-white/[0.08]',
  ],
  plugins: [],
}
