# File Explorer Sidebar Design

## Scope

Rebuild only the Omafil sidebar to closely match the supplied Windows 11 File Explorer reference. The main content area, toolbar, file data, cloud-provider integration, and backend behavior are out of scope.

## Visual Structure

The sidebar is a fixed-width, light-gray vertical surface. It contains:

1. A top-level Home row with a blue selected background and a right-side blue accent rail.
2. A Pinned section with an icon heading and compact child rows for Desktop and Concepts.
3. Individual OneDrive and Google Drive rows as visual placeholders only; they do not imply connected accounts.
4. A Your Files section with Downloads, Documents, Pictures, Videos, and Music.
5. A Drives section with C: (OS) and D: (USB Stick).
6. A Tags section with colored dots for Design, Dev, and School plus a Create new tag action.

## Interaction

Each selectable row updates the locally held active item and receives the selected visual state. No file listing, external account, drive, tag creation, or persistence behavior is introduced in this slice.

## Implementation Boundaries

- Svelte components contain markup and local interaction state only.
- All visual rules stay in external CSS files under `src/src/lib/styles/` or the sidebar CSS file.
- Fluent SVG icons are used for iconography.
- Static labels are UI placeholders, not evidence of local files, attached drives, or authenticated cloud storage.

## Acceptance Criteria

- Sidebar hierarchy, spacing, selection treatment, and tags visually resemble the supplied reference.
- Home has the default active state.
- Keyboard focus is visible for selectable items.
- `bun run check` and `bun run build` pass.
