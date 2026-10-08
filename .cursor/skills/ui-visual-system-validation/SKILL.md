---
name: ui-visual-system-validation
description: Validate Larry desktop UI against the project visual system, including shell geometry, independent pane scrolling, compact controls, icon consistency, focus states, keyboard interaction, accessibility, and responsive behavior. Use when creating or revising Vue screens, panels, forms, buttons, inputs, tabs, navigation, titlebars, or other interactive UI in Larry.
---

# Larry UI visual system validation

Use this skill for every meaningful UI change in Larry. Read `docs/design.md` before changing layout or interaction patterns.

The goal is a dense, calm, dark desktop workspace for API testing and network investigation. Bruno, Insomnia and Postman are references for product density and panel behavior only. Do not copy their branding, names, icons, colors or proprietary layout details.

## Required workflow

1. Inspect the existing component, its parent layout and the relevant design tokens.
2. Identify the region being changed: titlebar, sidebar, request workspace, response inspector, statusbar, modal or overlay.
3. Check whether the change belongs in a component, a composable, a service or the shell. Keep one responsibility per component.
4. Use realistic Larry content: long URLs, request names, headers, JSON bodies, errors and response metadata.
5. Render the actual Tauri application when possible. A source-only review is not visual validation.
6. Inspect at 320, 360, 390, 414 and one wide viewport. Also inspect a short-height desktop window because the product is desktop-first.
7. Fix issues, render again and report what was checked. If the native Tauri window cannot be captured, state that limitation explicitly.

## Shell and scroll rules

- The app shell fills the available window.
- The main workspace must not create a document-level scrollbar for normal request editing.
- Topbar and statusbar do not scroll with request content.
- Sidebar, request workspace and response inspector own their long-content scroll independently.
- Grid or flex children that scroll use `min-height: 0`.
- Use `overflow: hidden` on the shell and controlled `overflow-y: auto` on content panes.
- Do not use large external padding or `width: calc(100% - 40px)` for the desktop workspace.
- Prefer internal padding of 12 to 20 px and 1 px region dividers.
- Code, JSON, URLs and headers may wrap or scroll inside their own bounded area.
- A fixed `min-height` must not push the scrollbar to `body`.

When reviewing a screenshot, reject the change if one scrollbar moves the titlebar, sidebar, editor and response together.

## Density and hierarchy

- There is one primary action per region.
- The request header follows `[method] [url] [send]` and aligns controls on one baseline.
- Sidebar rows stay compact and expose secondary actions on hover, focus or context menu.
- Panels are work regions, not floating marketing cards.
- Tabs organize related context; do not add tabs only to hide a single field.
- Status is text plus restrained color, never color alone.
- Avoid gradients, glass effects, decorative shadows, sparkle symbols and decorative side borders.

## Controls

Validate these dimensions and states:

- compact buttons: 28 to 32 px high;
- normal buttons and fields: 32 to 36 px high;
- icon buttons: at least 32 px desktop and 40 px touch;
- field label: 6 px above the field;
- related controls: 4 to 12 px spacing;
- primary action has stronger contrast but the same baseline as adjacent fields.

Every interactive control must have intentional states for idle, hover, focus-visible, pressed, disabled and loading when applicable. Do not let loading change button width unexpectedly. Errors require nearby text, not only a red border.

Inputs must keep their label, preserve long values, expose focus, and avoid placeholder-only meaning. Textareas and JSON editors must have bounded internal scrolling.

## Icons and arrows

Use one consistent linear SVG icon set. Do not use Unicode characters as functional icons.

- use a consistent down chevron for selects and menus;
- use right/down chevrons for collapsed/expanded trees;
- use standard line icons for search, copy, save, run, close and window controls;
- provide an accessible name for icon-only controls;
- add a tooltip when the action is not obvious from context;
- keep stroke weight, visual box and baseline consistent;
- do not mix filled and outline icon styles without a documented reason.

## Accessibility and interaction

- Keyboard focus must be visible.
- Every interactive icon needs a semantic label.
- Tabs use `tablist`, `tab`, `tabpanel` semantics when appropriate.
- Selected, disabled, loading and unavailable states must be distinguishable without color alone.
- Hover must not be the only way to discover a destructive or essential action.
- Confirm actions that can lose data or send requests to a risky environment.
- Preserve the original technical error alongside friendly guidance.
- Tooltips complement visible labels; they do not replace labels for primary actions.

## Modal validation

Use the shared `AppModal` pattern for confirmations, information, errors and risky actions. Do not introduce `window.alert`, `window.confirm` or `window.prompt`.

- use `neutral`, `info`, `danger` or `error` according to the consequence;
- give the modal a specific title and an explanation of the consequence;
- name the primary action with the real verb, such as `Remover request` or `Revisar importação`;
- keep a safe secondary action such as `Cancelar` or `Fechar`;
- move focus into the modal when it opens;
- trap Tab and Shift+Tab inside the modal;
- close with Escape only when closing is safe;
- restore focus to the control that opened the modal;
- keep technical errors intact and offer the next step;
- prevent clipping at 320, 360, 390, 414 and short desktop heights;
- do not close a destructive confirmation on backdrop click unless the flow explicitly permits it.

## Review checklist

### Layout

- [ ] Shell reaches the window edges without an unnecessary outer gutter.
- [ ] No global workspace scrollbar appears during normal use.
- [ ] Long content scrolls in the correct pane.
- [ ] `min-height: 0` is present in scrolling grid/flex children.
- [ ] No clipping or horizontal overflow exists.
- [ ] Short-height desktop windows remain usable.

### Controls

- [ ] One primary action is clear.
- [ ] Buttons and fields align and share a sensible height.
- [ ] Idle, hover, focus-visible, pressed, disabled and loading states were checked.
- [ ] Long labels, URLs, headers and errors have deliberate wrapping or truncation.

### Icons and accessibility

- [ ] No Unicode arrow or emoji is used as a functional icon.
- [ ] Icon-only controls have accessible names.
- [ ] Tooltips exist where the icon meaning is not obvious.
- [ ] Keyboard navigation and focus are usable.
- [ ] Status does not rely only on color.

### Product fit

- [ ] The visual language follows `docs/design.md`.
- [ ] The change preserves local-first behavior.
- [ ] No secret is exposed in UI fixtures, logs or screenshots.
- [ ] Metrics show their provenance or say `Indisponível`.
- [ ] The change adds Larry's API testing and network debugging identity rather than cloning a competitor.
- [ ] No native browser alert, confirm or prompt remains in the flow.
- [ ] Modals use the shared template and have focus, Escape and keyboard behavior defined.

## Verification report

End the UI task with:

- reference interpreted;
- files changed;
- viewports and window sizes checked;
- scroll and overflow result;
- interaction states checked;
- accessibility result;
- render URL or native preview location;
- limitations, if the real Tauri window could not be inspected.
