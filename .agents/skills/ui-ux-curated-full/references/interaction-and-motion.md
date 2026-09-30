# Interaction and Motion

Motion communicates continuity, hierarchy, and state; it is not decoration. Prefer transform/opacity animations. Keep common UI transitions quick and cancellable. Enter can be slightly slower than exit. Avoid springy motion on serious error/destructive states.

Use spatial continuity for drawers, sheets, assistants, and command surfaces. When a region expands, preserve the user's anchor. Never make the user wait for an animation before acting. Respect `prefers-reduced-motion`.
