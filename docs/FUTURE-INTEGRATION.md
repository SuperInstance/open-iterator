# Future Integration: open-iterator (Lapce Fork)

## Current State
A fork of the Lapce lightning-fast code editor with SuperInstance extensions. Lapce is built in Rust with Floem UI, Wasm runtime, and plugin support via WASI.

> **Note:** This is a fork of the Lapce open-source project. We preserve upstream code and add SuperInstance-specific features.

## Integration Opportunities

### With ternary editing patterns
Lapce's Rust-native architecture makes it ideal for ternary-aware code editing. Tree-sitter grammars for ternary syntax, semantic highlighting for ternary values (red/negative, gray/zero, green/positive), and structural editing for ternary strategy vectors. When you edit a ternary strategy file, Lapce understands the domain.

### With room-as-codespace development
Lapce runs in the browser via WASM and natively on desktop. In a Codespace room, Lapce provides the development environment with ternary-specific extensions: room skill editor, cell grid visualizer, and strategy debugger. The WASM build runs directly in the Codespace's web editor.

### With hermit-zed
Lapce and Zed are complementary editors in the fleet. Zed (hermit-zed) provides spectral analysis of codebases. Lapce provides fast, modal editing with ternary syntax support. Together they form the fleet's development toolkit.

## Our Integration (Not Upstream Changes)
We do NOT modify Lapce's core editor. Our integration is:
- Custom Tree-sitter grammars for ternary syntax
- SuperInstance theme with ternary color coding
- Plugin API extensions for room interaction
- WASM build customization for browser rooms

## Potential in Mature Systems
Lapce becomes the fleet's ternary IDE. Custom grammars understand ternary strategy files. Plugins interact with rooms. The WASM build provides zero-install room development in any browser. Fast, Rust-native, ternary-aware.

## Cross-Pollination Ideas
- **hermit-zed**: Zed's spectral analysis complements Lapce's editing
- **Spreadsheet-moment**: Spreadsheet UI for room visualization; Lapce for room development
- **open-application (Tauri)**: Desktop app combining Lapce editing with room management

## Dependencies for Next Steps
- Ternary Tree-sitter grammar
- SuperInstance theme with ternary colors
- Room interaction plugin
