# Burr Design System

Burr should feel like one precise local CAD instrument. The file browser and
the Look viewport share a semantic theme; neither surface chooses an unrelated
palette.

## Principles

1. **Geometry is primary.** Navigation and controls stay quieter than the model.
2. **Use industrial neutrals.** Graphite and warm aluminium provide structure;
   steel blue identifies CAD information.
3. **Reserve orange for attention.** Burr orange marks the selected item,
   keyboard focus, or a condition that deserves inspection. It is not general
   decoration.
4. **Use status colours only for status.** Green means healthy watching; red
   means the watcher or model load needs attention.
5. **Theme the whole viewer.** Switching modes changes the shell, canvas, and
   Look controls together.

## Semantic colour tokens

Components consume semantic tokens rather than raw palette names.

| Token | Dark | Light | Purpose |
| --- | --- | --- | --- |
| `canvas` | `#0c0d10` | `#c9ced0` | Model viewport and empty/loading states |
| `sidebar` | `#111215` | `#f4f3ef` | Navigation background |
| `surface` | `#17191d` | `#fbfaf7` | Header, footer, and raised controls |
| `surface-hover` | `#1d2024` | `#e9e7e1` | Hovered rows and controls |
| `surface-selected` | `#24272c` | `#ffffff` | Current model row |
| `border` | `#2a2d33` | `#d5d2cb` | Standard separators |
| `border-strong` | `#3b4047` | `#b9b7b0` | Focused or selected boundaries |
| `text` | `#f3f4f5` | `#1b1d1f` | Primary labels |
| `text-secondary` | `#b6bbc0` | `#555b60` | File and folder labels |
| `text-muted` | `#858c92` | `#777e83` | Metadata and supporting copy |
| `steel` | `#aebfc8` | `#4f6671` | CAD formats and folder geometry |
| `steel-surface` | `#263239` | `#dce4e7` | Steel information backgrounds |
| `accent` | `#f08a32` | `#d86d16` | Inspection, selection, and focus |
| `success` | `#6fbd88` | `#3d8a58` | Healthy watcher state |
| `danger` | `#dd746d` | `#b94f48` | Failed watcher or model state |

## Shape, spacing, and type

- Base spacing is `4px`; compose common gaps and padding in 8, 12, 16, and
  24px steps.
- Small controls and tree rows use a `6px` radius; larger panels use `10px`;
  switches and counts use a pill radius.
- Folder and file labels use 11px medium-weight type. Supporting labels use
  9–10px type with restrained tracking. Project and model titles use 14–16px
  semibold or bold type.
- Tree labels stay on one line and truncate with an ellipsis. The complete path
  remains available through the hover title.

## Theme contract

The shell stores the selected `dark` or `light` mode in `localStorage` under
`burr-theme`; without a stored choice it follows the operating-system colour
preference. The root `data-theme` attribute selects shell tokens.

Every model request includes the same theme. Burr gives Look the matching
WebGL clear colour and injects a narrow, pinned CSS override for Look's header,
toolbar, legend, and controls. Look continues to own parsing, tessellation,
camera interaction, and rendering.

The switch must therefore be tested as an end-to-end contract: shell tokens,
iframe theme marker, canvas colour, persistence after reload, and model
interaction all change or survive together.

## Interaction rules

- Folders are collapsed initially and preserve explicit expansion while Burr
  polls for file changes.
- Models and Checks share one tabbed sidebar. Switching tools must not resize or
  cover the geometry viewport, and opening a finding must keep its check context
  visible.
- The selected model uses the accent only as a narrow edge, never as a large
  fill.
- Check outcomes use green for `pass`, orange for `incomplete`, and red for
  `fail`. A project with no enabled packs says “not run”; it never receives a
  healthy status treatment.
- Pack cards show outcome and evaluated-target coverage before individual
  findings. Findings preserve Burr's actionable title, evidence summary, and
  remediation; model-linked findings behave as buttons rather than making every
  row look clickable.
- Interactive controls have visible hover and `:focus-visible` states in both
  themes.
- Text and meaningful controls target WCAG AA contrast. Decorative logo imagery
  has empty alternative text because the adjacent `Burr` label names the app.
