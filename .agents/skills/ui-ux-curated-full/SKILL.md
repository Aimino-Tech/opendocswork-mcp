---
name: ui-ux-curated-full
description: High-quality product UI/UX design system for web and mobile with curated glassmorphism, liquid glass, and liquid morph patterns. Use before implementing UI, when redesigning a surface, or when reviewing interfaces that feel generic, inconsistent, dense, or visually weak. Produces a stable UI contract and page-level design decisions before code.
---
# UI/UX Curated Full

Use this skill as the design-intelligence layer before implementation. Do not jump directly from a vague request to CSS. First understand the existing product, define the information hierarchy, pick a restrained visual direction, and create a UI contract.

## Required workflow
1. Read the existing product language and reuse existing components/tokens whenever reasonable.
2. Read `references/ux-foundations.md` and the most relevant product-pattern reference.
3. For a new or materially redesigned surface, choose at most one primary style family from `styles/`.
4. Fill `templates/ui-contract.md` before implementation. For multi-page work, also create `templates/design-system-master.md` plus page overrides.
5. Use the closest page/component templates as structure, not as pixel-perfect output.
6. Implement with semantic tokens and explicit states.
7. Review the rendered result with the separate visual-verifier skill. Fix observed defects, not imagined ones.

## Style policy
- Default product surfaces are solid, legible, and task-first.
- Glass is an elevation/hierarchy material, not wallpaper.
- Liquid effects belong on focal, floating, transient, or assistant surfaces; data tables and long-form forms stay calmer.
- Never mix glassmorphism, liquid glass, and liquid morph on one screen unless one is clearly dominant and the others are limited to tiny accents.

## Load only what is relevant
- General UX: `references/ux-foundations.md`
- Accessibility: `references/accessibility.md`
- Mobile: `references/mobile-first.md`
- Dashboards: `references/dashboards.md`
- Assistant/chat: `references/chat-assistant.md`
- Planning/verification tools: `references/planning-verification.md`
- Vue/Nuxt/Tailwind: `references/nuxt-vue-tailwind.md`
- Visual style: one file from `styles/`
- Page/component pattern: one or two files from `templates/`

## Non-negotiable anti-patterns
Do not ship generic AI-SaaS styling: purple/pink gradient as identity by default, glow everywhere, excessive pills, every section in a floating card, unreadable translucent tables, random radii/shadows, giant marketing typography inside productivity screens, decorative icons without meaning, or motion that delays task completion.

## Output contract
Before coding, state: hierarchy, primary action, layout regions, density, type scale, semantic colors, surface/elevation rules, component states, responsive behavior, accessibility constraints, chosen style family, reusable existing components, and explicit anti-patterns. Use `templates/ui-contract.md`.
