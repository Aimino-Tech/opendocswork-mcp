# UI defect taxonomy

Classify before fixing.

## H — Hierarchy
Examples: primary action unclear; every card has equal weight; status and action compete; section titles fail to organize scanning.

## L — Layout
Examples: misalignment; accidental gaps; cramped clusters; overflow; excessive card nesting; important content below unnecessary chrome.

## T — Typography
Examples: weak type hierarchy; unreadable size/line-height; too many weights; long line lengths; secondary text too faint.

## C — Component consistency
Examples: inconsistent button/input shapes; arbitrary radii/shadows; duplicate component patterns; icons from mixed visual languages.

## M — Material/color
Examples: glass reduces legibility; low contrast; effect overload; gradients/glow used without hierarchy value; semantic colors inconsistent.

## S — State/interaction
Examples: missing loading/error/empty/selected/disabled state; focus invisible; click target unclear; destructive action insufficiently distinct.

## R — Responsive
Examples: desktop layout merely squeezed; clipped controls; priority changes lost on mobile; tables unusable; fixed widths cause horizontal scroll.

## A — Accessibility
Examples: contrast issue; unlabeled control; keyboard focus missing; touch target too small; color-only status; motion ignores reduced-motion preference.

## P — Product mismatch
Examples: new screen invents a different design language; density differs radically from adjacent workflow; novel visual effect conflicts with product purpose.

## Severity

- **Blocker**: prevents task completion, hides critical state, creates accessibility failure, or introduces broken responsive behavior.
- **Major**: materially slows comprehension or makes the product look inconsistent/unreliable.
- **Minor**: polish issue with no meaningful task impact.
