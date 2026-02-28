## ADDED Requirements

### Requirement: Batch cell rendering into style runs
The terminal renderer SHALL group consecutive cells with identical styling into single text segments, reducing element count from O(cells) to O(style-runs).

#### Scenario: Uniform text row
- **GIVEN** a terminal row with 80 cells all having the same color and style
- **WHEN** the row is rendered
- **THEN** the system creates 1 text element (not 80) for that row

#### Scenario: Mixed styling row
- **GIVEN** a row with text "hello" in white, "world" in red, "!" in white
- **WHEN** the row is rendered
- **THEN** the system creates 3 styled segments (white, red, white)

#### Scenario: Preserved styling attributes
- **GIVEN** cells with bold, underline, or inverse flags set
- **WHEN** grouped into segments
- **THEN** the segment inherits all applicable style flags
- **AND** the rendered output matches original per-cell rendering

### Requirement: Viewport culling for scrollback
The renderer SHALL only render rows visible in the current viewport, skipping off-screen scrollback lines.

#### Scenario: Large scrollback buffer
- **GIVEN** a terminal with 10,000 lines of scrollback
- **AND** viewport showing only 24 rows
- **WHEN** rendering the terminal
- **THEN** the system processes only the 24 visible rows (not 10,000)

#### Scenario: Scrolled viewport
- **GIVEN** a terminal with scroll_offset of 100 rows
- **WHEN** rendering
- **THEN** the system renders rows 100-123 (visible range)
- **AND** skips rows 0-99 and 124+

### Requirement: Row-level render caching
The renderer SHALL cache rendered rows keyed by content hash, reusing unchanged rows between frames.

#### Scenario: Static content
- **GIVEN** a terminal row that hasn't changed between frames
- **WHEN** rendering the next frame
- **THEN** the system reuses the cached elements for that row
- **AND** does not rebuild the row from cells

#### Scenario: Cache invalidation on change
- **GIVEN** a cached row
- **WHEN** the row content changes (new output, scrolling)
- **THEN** the cache entry is invalidated
- **AND** the row is rebuilt on next render

#### Scenario: Cache size limits
- **GIVEN** a configured cache size of 200 rows
- **WHEN** more than 200 unique rows are rendered
- **THEN** the least recently used entries are evicted

### Requirement: Cursor overlay rendering
The cursor SHALL be rendered as a separate overlay element positioned correctly over batch-rendered content.

#### Scenario: Cursor in batched row
- **GIVEN** a row rendered with batch style-run approach
- **WHEN** the cursor is positioned within that row
- **THEN** a cursor element is placed at the correct column position
- **AND** the cursor appearance matches original implementation

#### Scenario: TUI active cursor hiding
- **GIVEN** a TUI application (vim, neovim) is active in the terminal
- **WHEN** rendering occurs
- **THEN** the batch renderer respects should_show_cursor() logic
- **AND** hides the cursor overlay when TUI is active
