# File Explorer Sidebar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the Omafil sidebar as a static Windows 11 File Explorer-style navigation surface.

**Architecture:** `Sidebar.svelte` owns static navigation data and active-row state. `sidebar.css` owns presentation and `variables.css` supplies reusable color tokens. No routes, filesystem, cloud, drive, or tag persistence is added.

**Tech Stack:** Svelte 5, TypeScript, Vite, external CSS, Fluent UI SVG icons.

---

## File Structure

- Modify: `src/src/lib/components/Sidebar/Sidebar.svelte` — grouped navigation markup and local selection state.
- Modify: `src/src/lib/components/Sidebar/sidebar.css` — Explorer-style visual hierarchy and interaction states.
- Modify: `src/src/lib/styles/variables.css` — shared neutral, selected, and tag color tokens.
- Verify: `src/package.json` scripts `check` and `build`.

### Task 1: Define the static sidebar hierarchy

**Files:**

- Modify: `src/src/lib/components/Sidebar/Sidebar.svelte`

- [ ] **Step 1: Establish the acceptance check before implementation**

Run:

```bash
cd /home/dave/projects/omafil/src
bun run check
```

Expected: record the current result before replacing the hierarchy.

- [ ] **Step 2: Replace component data with reference-aligned groups**

Define arrays for `pinnedItems`, `fileItems`, `driveItems`, and `tags`. Every entry has an `id`, `label`, and Fluent SVG `icon`; each tag additionally has a `color` of `red`, `yellow`, or `blue`.

Use this active state:

```ts
let activeItem = $state('home')
```

- [ ] **Step 3: Render semantic grouped navigation**

Render, in order: Home; Pinned; standalone OneDrive and Google Drive placeholder rows; Your Files; Drives; Tags; and Create new tag. Use `<section>`, `<h2>`, and `<nav>` labels. Every selectable row uses:

```svelte
class:file-sidebar__item--active={activeItem === item.id}
aria-current={activeItem === item.id ? 'page' : undefined}
onclick={() => (activeItem = item.id)}
```

The OneDrive and Google Drive rows are visual-only; they must not call an API or assert an authenticated connection.

- [ ] **Step 4: Re-run the type check**

Run:

```bash
bun run check
```

Expected: `svelte-check found 0 errors and 0 warnings`.

### Task 2: Apply the File Explorer visual system

**Files:**

- Modify: `src/src/lib/styles/variables.css`
- Modify: `src/src/lib/components/Sidebar/sidebar.css`

- [ ] **Step 1: Add visual tokens**

Add tokens for the selected blue surface and accent rail, muted label text, and tag dot colors:

```css
--surface-selected: #dceeff;
--accent-selected: #0078d4;
--text-muted: #616161;
--tag-red: #ff5f57;
--tag-yellow: #f7c948;
--tag-blue: #2d9bf0;
```

- [ ] **Step 2: Style Home as the primary active row**

Give `.file-sidebar__item--active` a pale blue background, blue text, and a right accent rail:

```css
box-shadow: inset -3px 0 0 var(--accent-selected);
```

- [ ] **Step 3: Style hierarchy and tag dots**

Use smaller muted section headings, tighter child-row padding, 16–20px icon alignment, and a `.file-sidebar__tag-dot` class with a 12px circular shape. Add modifier classes for red, yellow, and blue dots.

### Task 3: Build and manually verify the desktop UI

**Files:**

- Verify: `src/src/lib/components/Sidebar/Sidebar.svelte`
- Verify: `src/src/lib/components/Sidebar/sidebar.css`

- [ ] **Step 1: Run static validation**

Run:

```bash
cd /home/dave/projects/omafil/src
bun run check
bun run build
```

Expected: both commands exit successfully and Vite emits `dist/index.html` plus hashed CSS and JavaScript assets.

- [ ] **Step 2: Run the desktop app**

Run:

```bash
cd /home/dave/projects/omafil/src-tauri
cargo tauri dev
```

Expected: the native Omafil window opens with the sidebar visible. Selecting Home, a pinned item, a file location, a drive, or Tags moves the selected appearance to that row.
