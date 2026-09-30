# Nuxt / Vue / Tailwind Guidance

Keep design tokens in CSS variables/Tailwind theme and components semantic. Do not scatter raw colors, shadows, or radii through templates. Prefer composable primitives for Button, Input, Select, Dialog/Sheet, Tooltip, Badge, Table/List row, and Surface.

In Vue, keep visual variants explicit and bounded; avoid components with dozens of boolean props. Use slots for structured extension. For Nuxt, preserve SSR-safe behavior and avoid layout shifts from client-only measurement. Use responsive utilities to change information architecture, not merely font size.
