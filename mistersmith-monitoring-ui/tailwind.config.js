/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        discovery: {
          pattern: '#3B82F6',      // blue
          anomaly: '#EF4444',      // red
          connection: '#8B5CF6',   // purple
          solution: '#10B981',     // green
          question: '#F59E0B',     // amber
          insight: '#06B6D4',      // cyan
        }
      }
    },
  },
  plugins: [],
}