# FileChecker

FileChecker is a fast, lightweight, and visual directory diff and synchronization tool built with Tauri v2, React 19, and TypeScript.

## Core Features

- **Pure Minimalist Portal**: Clean aurora entrance with smooth animations.
- **Peer Directory Comparison**: Compare any two directories recursively with no forced primary/secondary hierarchy.
- **Live In-Tree Visual Preview**: Direct visual representation of planned actions on both file trees before any disk I/O occurs:
  - **Ghost Nodes**: Preview files to be created with dashed green borders (`+`).
  - **Deletion Strikethrough**: Preview files to be removed with red strikethrough (`✕`).
  - **Overwrite Warnings**: Visual badges indicating direction of overwrite.
- **Center Action Gutter**: Intuitive icon-based action selector (`➔`, `⬅`, `✕`, `─`) for each diff item.
- **Batch Presets**: One-click strategy application (Sync Newer, Mirror A to B, Mirror B to A, Reset).
- **Embedded DiffViewer**: Side-by-side and inline syntax-highlighted diff for modified text files.
- **Portable & Standalone**: Single standalone executable with no installation required.

## Packaging a Portable Version

To build a standalone, portable version (an `.exe` file that doesn't require installation), run:

```bash
npm run build:portable
```

The compiled standalone executable will be located at:
`src-tauri/target/release/filechecker.exe`
