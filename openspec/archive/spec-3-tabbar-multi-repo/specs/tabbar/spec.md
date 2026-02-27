# TabBar 组件规格

## ADDED Requirements

### Requirement: TabBar Container

The application must display a TabBar at the top of the window for multi-repository management.

#### Scenario: TabBar Layout

- **WHEN** multiple repositories are open
- **THEN** a TabBar is displayed at the top of the window
- **AND** it has a height of 40px
- **AND** it has a dark background (#252526)
- **AND** a bottom border separates it from the content area

#### Scenario: Single Repository

- **GIVEN** only one repository is open
- **WHEN** viewing the application
- **THEN** the TabBar still shows one tab
- **AND** it displays the repository name

### Requirement: Tab Component

Each tab represents an open repository workspace.

#### Scenario: Tab Display

- **WHEN** a repository is opened
- **THEN** a new tab appears in the TabBar
- **AND** the tab shows the repository name (e.g., "myproject")
- **AND** the tab has an icon indicating the repository type (📁)

#### Scenario: Active Tab

- **WHEN** a tab is active/selected
- **THEN** it has a highlighted background (#37373d)
- **AND** it has a top border indicator (blue #007acc)
- **AND** the text is bright white

#### Scenario: Inactive Tab

- **WHEN** a tab is inactive
- **THEN** it has a normal background
- **AND** the text is muted gray (#858585)
- **AND** hovering shows a slightly lighter background

#### Scenario: Close Button

- **WHEN** viewing a tab
- **THEN** a close button (×) appears on hover or when active
- **AND** clicking it closes that repository tab
- **AND** if it's the last tab, return to startup page

#### Scenario: Click to Switch

- **WHEN** the user clicks on an inactive tab
- **THEN** that tab becomes active
- **AND** the Sidebar updates to show that repository's worktrees
- **AND** the TerminalView updates to show that repository's terminal
- **AND** the transition is immediate (< 100ms)

### Requirement: New Tab Button

A button to add new repositories must be available.

#### Scenario: Add Button Display

- **WHEN** viewing the TabBar
- **THEN** a [+] button appears at the right end
- **AND** it has a secondary button style
- **AND** it has a tooltip "Open Repository"

#### Scenario: Add Button Click

- **WHEN** the user clicks the [+] button
- **THEN** a file picker dialog opens
- **AND** selecting a repository adds a new tab
- **AND** the new tab becomes active

### Requirement: Tab Overflow

When there are too many tabs to fit, overflow handling is required.

#### Scenario: Many Tabs

- **GIVEN** there are more tabs than can fit in the window width
- **WHEN** viewing the TabBar
- **THEN** tabs shrink to a minimum width (120px)
- **AND** if still overflowing, scroll arrows appear

#### Scenario: Scroll Tabs

- **GIVEN** tabs overflow the available space
- **WHEN** the user clicks the left/right arrows
- **THEN** the tab list scrolls horizontally
- **AND** smooth animation is applied

## UI Specifications

### Colors

- Background: #252526
- Active background: #37373d
- Active indicator: #007acc (blue)
- Text active: #ffffff
- Text inactive: #858585
- Hover background: #2a2d2e
- Close button hover: #c75450 (red)
- Border: #3e3e42

### Typography

- Tab text: 13px, medium weight
- Max width: 200px (truncated with ellipsis)
- Min width: 120px

### Spacing

- TabBar height: 40px
- Tab padding: 8px 16px
- Tab gap: 1px
- Close button size: 16px
- Add button margin-left: 8px

### Icons

- Repository: 📁
- Close: ×
- Add: +
- Scroll left: ‹
- Scroll right: ›
