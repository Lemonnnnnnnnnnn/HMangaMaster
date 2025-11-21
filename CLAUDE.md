# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

HMangaMaster is a desktop manga download management application built with **Tauri + Vue 3 + TypeScript**. The application combines a modern web frontend with a Rust backend for native desktop capabilities, providing manga downloading from various sources with progress tracking and library management.

## Development Commands

### Frontend Development
```bash
# Start Vite development server
bun run dev
# or
vite

# Build frontend with type checking
bun run build

# Preview built application
bun run preview
```

### Tauri Development
```bash
# Start Tauri development mode (frontend + backend)
tauri dev

# Build complete application
tauri build

# Access Tauri CLI through package.json
bun run tauri [command]
```

### Code Quality
```bash
# Lint with ESLint
eslint src/

# Format with Prettier
prettier --write src/

# Type checking only
vue-tsc --noEmit
```

## Architecture Overview

### Frontend (Vue 3 + TypeScript)
- **Framework**: Vue 3 with Composition API (`script setup`)
- **State Management**: Pinia stores
- **Routing**: Vue Router with 5 main routes:
  - `/` - Home page
  - `/download` - Download management
  - `/setting` - Application settings
  - `/manga/:path` - Manga detail view
  - `/history` - Download history
- **Styling**: Tailwind CSS with Vite integration
- **UI Components**: Custom components in `src/components/`
- **Path Aliases**: `@/` maps to `src/`

### Backend (Rust + Tauri)
- **Entry Point**: `src-tauri/src/main.rs`
- **Commands**: All Tauri commands defined in `src-tauri/src/commands.rs`
- **Core Modules**:
  - `crawler/` - Web scraping for manga content
  - `download/` - Download management system
  - `library/` - Manga library organization
  - `history/` - Download history tracking
  - `config/` - Configuration management
  - `progress/` - Progress tracking system
  - `request/` - HTTP client handling
  - `services/` - Business logic services

### Key Architecture Patterns

**Parser System**: Modular crawler architecture with:
- `SiteParser` trait for different manga sites
- Auto-detection by domain
- Factory pattern for parser creation
- Progress reporting support

**Task Management**: Centralized task service handling:
- Download task lifecycle
- Progress tracking
- Cancellation support
- History persistence

**Configuration**: Centralized config service with:
- JSON-based settings storage
- Parser-specific configurations
- Runtime updates
- Library management

## Important File Locations

### Configuration Files
- `package.json` - Frontend dependencies and scripts
- `src-tauri/Cargo.toml` - Rust dependencies
- `src-tauri/tauri.conf.json` - Tauri application configuration
- `vite.config.ts` - Vite build configuration
- `tsconfig.app.json` - TypeScript configuration

### Source Code Organization
- `src/views/` - Page components (Home, Download, Setting, MangaDetail, History)
- `src/components/` - Reusable UI components
- `src-tauri/src/commands.rs` - All Tauri command definitions
- `src-tauri/src/crawler/parsers/` - Site-specific manga parsers
- `src/router/index.ts` - Vue Router configuration

### State Management
- Pinia stores are located in `src/stores/`
- Application state shared between frontend and backend via Tauri commands

## Development Notes

### Port Configuration
- Frontend development server runs on port **1420**
- HMR (Hot Module Replacement) uses port **1421**
- Tauri expects fixed port configuration

### Build Process
1. Frontend builds to `dist/` directory
2. Tauri bundles frontend into native application
3. TypeScript validation required before build (`vue-tsc --noEmit`)

### Code Quality Tools
- **ESLint** with Oxlint for modern JavaScript/TypeScript linting
- **Prettier** for code formatting
- **TypeScript** for type safety
- No formal testing framework currently configured

## Key Features Implementation

### Download System
- Tasks managed through `TaskService`
- Progress tracking via `ProgressReporter`
- Concurrent download support with configurable limits
- Persistent task history

### Parser Extensibility
- New manga sites supported by implementing `SiteParser` trait
- Domain-based auto-selection
- Configuration per parser
- Custom download headers support

### File Organization
- Manga files organized by site/series structure
- Sanitized filenames using `sanitize-filename`
- Metadata stored alongside downloaded content
- Library scanning and management

## Cursor Rules Integration

The project includes Cursor rules that emphasize:
- **Refactoring Principles**: No unused features, maintain existing functionality, avoid compatibility code
- **Code Organization**: Use folder structure for separation by functionality
- **TypeScript + Rust**: Maintain type safety across both language boundaries

These principles should be followed when making changes to ensure code quality and maintainability.