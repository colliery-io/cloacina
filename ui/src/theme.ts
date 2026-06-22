/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

import { createTheme, type MantineColorsTuple } from "@mantine/core";

/**
 * Aurora Dark theme (CLOACI-I-0129). A dark, "cold Nordic" identity — slate
 * surfaces + aurora accents (ice / teal / violet) and IBM Plex type. The exact
 * token hexes live in `theme.css` as CSS custom properties (components use
 * those directly); the Mantine palettes below mirror them so built-in Mantine
 * components inherit the same surfaces and accents.
 *
 * The app is locked to dark (`forceColorScheme="dark"` in main.tsx); the `dark`
 * tuple is therefore the active surface scale: index 0 = primary text, 6 =
 * card/input surface, 7 = body background, 9 = inset.
 */

// Slate surfaces. Mantine dark semantics: text=dark[0], body=dark[7],
// surface=dark[6], border=dark[4], dimmed=dark[2].
const dark: MantineColorsTuple = [
  "#e6e9ee",
  "#c3cbd5",
  "#8b95a3",
  "#5b6573",
  "#232a34",
  "#1b2129",
  "#161a21",
  "#0e1116",
  "#0b0e12",
  "#0a0c10",
];

// Aurora accents — the token hex sits at index 4 (== primaryShade).
const ice: MantineColorsTuple = [
  "#eaf2ff",
  "#cfe2ff",
  "#a9c9ff",
  "#93bcff",
  "#7fb2ff",
  "#669ff2",
  "#5e93ec",
  "#4f7fd0",
  "#3f6ab0",
  "#2f5590",
];

const teal: MantineColorsTuple = [
  "#e3faf6",
  "#c2f2eb",
  "#93e6db",
  "#73dccf",
  "#5fd0c5",
  "#4cbcb1",
  "#3da99e",
  "#2f8a80",
  "#236b63",
  "#184c46",
];

const violet: MantineColorsTuple = [
  "#efeaff",
  "#dcd4ff",
  "#bdadff",
  "#ab9bff",
  "#9d8cff",
  "#8a76f5",
  "#7b66e6",
  "#6753c4",
  "#52419e",
  "#3d3078",
];

const gold: MantineColorsTuple = [
  "#fbf2df",
  "#f4e2b8",
  "#ead08a",
  "#e1bb6e",
  "#d8a657",
  "#c4923f",
  "#a9792f",
  "#875f24",
  "#66461b",
  "#4a3214",
];

const ok: MantineColorsTuple = [
  "#e3faec",
  "#bff0d2",
  "#86e3aa",
  "#63da92",
  "#4bd07f",
  "#3bb86c",
  "#2f9d5b",
  "#247c47",
  "#1a5c35",
  "#123f25",
];

const bad: MantineColorsTuple = [
  "#ffe9e9",
  "#ffcaca",
  "#ff9d9d",
  "#f87e7e",
  "#f06464",
  "#e04d4d",
  "#c83e3e",
  "#a52f2f",
  "#7e2424",
  "#5a1919",
];

export const theme = createTheme({
  primaryColor: "ice",
  primaryShade: { light: 4, dark: 4 },
  defaultRadius: "md",
  colors: { dark, ice, teal, violet, gold, ok, bad },
  fontFamily:
    "'IBM Plex Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
  fontFamilyMonospace: "'IBM Plex Mono', ui-monospace, SFMono-Regular, Menlo, monospace",
  headings: {
    fontFamily: "'IBM Plex Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
    fontWeight: "600",
  },
});
