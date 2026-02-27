# GUI 实现规格

## ADDED Requirements

### Requirement: GUI Startup Page

The application shall display a graphical startup page when no workspace is configured.

#### Scenario: First Launch Display

- **WHEN** the application launches without a saved workspace
- **THEN** a centered window shall appear with:
  - Application title "pmux"
  - Welcome message
  - Description text explaining the purpose
  - A prominent "Select Workspace" button

#### Scenario: Button Interaction

- **WHEN** the user clicks the "Select Workspace" button
- **THEN** a native file picker dialog shall open
- **AND** the dialog title shall be "选择 Git 仓库"

#### Scenario: Valid Git Repository Selection

- **WHEN** the user selects a valid Git repository
- **THEN** the path shall be saved to configuration
- **AND** the view shall transition to workspace confirmation screen

#### Scenario: Invalid Directory Selection

- **WHEN** the user selects a non-Git directory
- **THEN** an error message shall display below the button
- **AND** the message shall be in Chinese: "所选目录不是 Git 仓库。请选择包含 .git 的文件夹。"
- **AND** the startup page shall remain visible

### Requirement: Visual Design

The GUI shall follow modern design principles.

#### Scenario: Color Scheme

- **WHEN** the application renders
- **THEN** it shall use a dark theme with:
  - Background: #1e1e1e or similar dark color
  - Card background: #252526
  - Primary button: blue accent color
  - Text: light gray (#cccccc) for body, white for headings
  - Error text: red (#f48771)

#### Scenario: Layout and Spacing

- **WHEN** components are rendered
- **THEN** they shall have appropriate spacing:
  - Window padding: 48px
  - Card padding: 32px
  - Element gaps: 16px-24px
  - Border radius: 8px for cards, 4px for buttons

### Requirement: Window Management

The application window shall behave like a native desktop app.

#### Scenario: Window Properties

- **WHEN** the application starts
- **THEN** the window shall have:
  - Title: "pmux"
  - Minimum size: 600x400 pixels
  - Default size: 800x600 pixels
  - Centered on screen

#### Scenario: Window Close

- **WHEN** the user closes the window
- **THEN** the application shall exit cleanly
- **AND** any unsaved state warnings shall be displayed (if applicable)

### Requirement: State Persistence

The application shall remember the selected workspace.

#### Scenario: Subsequent Launch

- **GIVEN** a workspace was previously selected
- **WHEN** the application launches again
- **THEN** it shall skip the startup page
- **AND** display the workspace view directly
