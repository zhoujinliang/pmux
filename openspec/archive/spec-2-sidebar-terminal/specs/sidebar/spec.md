# Sidebar 组件规格

## ADDED Requirements

### Requirement: Sidebar Container

The application must display a sidebar on the left side of the window.

#### Scenario: Layout

- **WHEN** the workspace view is displayed
- **THEN** a sidebar is shown on the left side
- **AND** the sidebar has a fixed width of 250px
- **AND** the sidebar has a dark background color (#252526)
- **AND** a border separates the sidebar from the terminal view

#### Scenario: Repository Header

- **WHEN** viewing the sidebar
- **THEN** the repository name is displayed at the top
- **AND** it uses a folder icon (📁)
- **AND** it uses a larger, bold font

### Requirement: Worktree List

The sidebar must display all worktrees in the repository.

#### Scenario: Display Worktrees

- **WHEN** worktrees are discovered
- **THEN** each worktree is displayed as an item in the list
- **AND** items are vertically stacked
- **AND** there is appropriate spacing between items

#### Scenario: Main Branch Highlighting

- **WHEN** displaying the main branch (main/master)
- **THEN** it appears at the top of the list
- **AND** it has a distinctive indicator

#### Scenario: Scrollable List

- **GIVEN** there are more than 10 worktrees
- **WHEN** the list exceeds the available height
- **THEN** the list becomes scrollable
- **AND** a scrollbar appears on the right

### Requirement: Worktree Item

Each worktree must be displayed with relevant information.

#### Scenario: Branch Name

- **WHEN** viewing a worktree item
- **THEN** the branch name is prominently displayed
- **AND** it uses the format "branch-name" or "● branch-name" if selected

#### Scenario: Path Display

- **WHEN** viewing a worktree item
- **THEN** the path is displayed below the branch name
- **AND** long paths are truncated with ellipsis
- **AND** the home directory is shown as "~"

#### Scenario: Ahead/Behind Count

- **GIVEN** a worktree has unpushed commits
- **WHEN** viewing the worktree item
- **THEN** "+N" is displayed indicating ahead count
- **AND** it uses a muted color

- **GIVEN** a worktree is behind remote
- **WHEN** viewing the worktree item
- **THEN** "-N" is displayed indicating behind count

#### Scenario: Selection State

- **WHEN** a worktree is selected
- **THEN** the item has a highlighted background
- **AND** a blue indicator appears on the left
- **AND** the status icon changes to ● (filled)

- **WHEN** a worktree is not selected
- **THEN** the item has a normal background
- **AND** the status icon is ○ (empty)

#### Scenario: Click to Select

- **WHEN** the user clicks on a worktree item
- **THEN** that worktree becomes selected
- **AND** the terminal view updates to show that worktree's pane
- **AND** the visual state updates immediately

### Requirement: New Branch Button

A button to create new branches must be available.

#### Scenario: Button Display

- **WHEN** viewing the sidebar
- **THEN** a "[+ New Branch]" button is at the bottom
- **AND** it has a secondary button style
- **AND** it spans the width of the sidebar

#### Scenario: Button Click

- **WHEN** the user clicks the "New Branch" button
- **THEN** a dialog/input appears for branch name entry
- **AND** creating the branch creates a new worktree and pane

## UI Specifications

### Colors

- Background: #252526
- Border: #3e3e42
- Selected background: #094771
- Text primary: #cccccc
- Text secondary: #858585
- Accent (ahead): #4ec9b0
- Status indicator: #75beff

### Typography

- Repo name: 16px, semibold
- Branch name: 14px, medium
- Path: 12px, regular, muted
- Count: 11px, regular

### Spacing

- Sidebar padding: 12px
- Item padding: 8px 12px
- Item gap: 2px
- Section gap: 16px
