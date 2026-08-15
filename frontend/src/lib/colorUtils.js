/**
 * Utility functions for generating and converting colours.
 */

/**
 * Converts HSL values (h: 0..360, s: 0..100, l: 0..100) to a CSS hex string (#rrggbb).
 * @param {number} h Hue in degrees (0-360)
 * @param {number} s Saturation in percent (0-100)
 * @param {number} l Lightness in percent (0-100)
 * @returns {string} Hex colour string (e.g., "#79b2ec")
 */
export function hslToHex(h, s, l) {
  const sat = s / 100;
  const light = l / 100;

  const c = (1 - Math.abs(2 * light - 1)) * sat;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = light - c / 2;

  let r = 0, g = 0, b = 0;

  if (0 <= h && h < 60) {
    r = c; g = x;
  } else if (60 <= h && h < 120) {
    r = x; g = c;
  } else if (120 <= h && h < 180) {
    g = c; b = x;
  } else if (180 <= h && h < 240) {
    g = x; b = c;
  } else if (240 <= h && h < 300) {
    r = x; b = c;
  } else if (300 <= h && h <= 360) {
    r = c; b = x;
  }

  const rHex = Math.round((r + m) * 255).toString(16).padStart(2, '0');
  const gHex = Math.round((g + m) * 255).toString(16).padStart(2, '0');
  const bHex = Math.round((b + m) * 255).toString(16).padStart(2, '0');

  return `#${rHex}${gHex}${bHex}`;
}

/**
 * Generates a random pastel-like hex colour (#rrggbb).
 * Pastel colours use a random hue (0..360), high lightness (68..75%),
 * and moderate-to-high saturation (65..85%).
 * @returns {string} Hex colour string
 */
export function getRandomPastelColor() {
  const h = Math.floor(Math.random() * 360);
  const s = Math.floor(Math.random() * 20) + 65; // 65% - 85%
  const l = Math.floor(Math.random() * 8) + 68;  // 68% - 75%

  return hslToHex(h, s, l);
}
