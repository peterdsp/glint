// Glint themes. Each theme is a set of CSS custom properties layered over the
// window's native vibrancy. Tints are semi-transparent so the desktop shows
// through the glass. Add a theme by dropping another entry here — that is the
// whole "custom theme" surface (a future build reads user themes from disk).

window.GLINT_THEMES = {
  aurora: {
    label: "Aurora — translucent, cool",
    swatch: "#8ea8ff",
    vars: {
      "--accent": "#5b8cff",
      "--tint": "rgba(255,255,255,0.42)",
      "--tint-2": "rgba(255,255,255,0.30)",
      "--stroke": "rgba(255,255,255,0.60)",
      "--ink": "#1b1d22",
      "--ink-2": "#5a5f6b",
    },
  },
  midnight: {
    label: "Midnight — deep, glassy dark",
    swatch: "#1f2333",
    vars: {
      "--accent": "#7c8cff",
      "--tint": "rgba(30,34,48,0.50)",
      "--tint-2": "rgba(255,255,255,0.08)",
      "--stroke": "rgba(255,255,255,0.18)",
      "--ink": "#f2f3f8",
      "--ink-2": "#a7adbf",
    },
  },
  sunset: {
    label: "Sunset — warm, vivid",
    swatch: "#ff9a6a",
    vars: {
      "--accent": "#ff7a59",
      "--tint": "rgba(255,244,238,0.42)",
      "--tint-2": "rgba(255,255,255,0.30)",
      "--stroke": "rgba(255,255,255,0.60)",
      "--ink": "#2a1c18",
      "--ink-2": "#7a5a4e",
    },
  },
  forest: {
    label: "Forest — calm, natural",
    swatch: "#5fb8a0",
    vars: {
      "--accent": "#1d9e75",
      "--tint": "rgba(238,248,244,0.42)",
      "--tint-2": "rgba(255,255,255,0.30)",
      "--stroke": "rgba(255,255,255,0.60)",
      "--ink": "#17251d",
      "--ink-2": "#4e6a5c",
    },
  },
  graphite: {
    label: "Graphite — quiet, neutral",
    swatch: "#c2c5cd",
    vars: {
      "--accent": "#3a3f4a",
      "--tint": "rgba(245,246,248,0.46)",
      "--tint-2": "rgba(0,0,0,0.05)",
      "--stroke": "rgba(255,255,255,0.70)",
      "--ink": "#1b1d22",
      "--ink-2": "#5f636e",
    },
  },
};
