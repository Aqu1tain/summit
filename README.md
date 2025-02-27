# Summit - Celeste Map Editor

Summit is a graphical editor for Celeste map files. It uses Cairn for file conversions between Celeste's binary format and JSON.

## Features

- View and edit Celeste map files
- Visual grid-based tile editor
- View all rooms at once or focus on a specific room
- Customizable key bindings
- Simple and intuitive UI

## Requirements

- Cairn (must be installed and available in your PATH)
- Rust (for building from source)

## Building from Source

```bash
git clone https://github.com/yourusername/summit.git
cd summit
cargo build --release
```

The binary will be located in `target/release/summit`.

## Usage

1. Open Summit
2. Click File > Open to select a Celeste .bin map file
3. Edit the map by placing or removing tiles
4. Save your changes with File > Save

### Controls

- Pan: Middle Mouse Button (default)
- Place Block: Left Mouse Button (default)
- Remove Block: Right Mouse Button (default)
- Zoom In: E key or mouse wheel up
- Zoom Out: Q key or mouse wheel down
- Save: Ctrl+S
- Open: Ctrl+O

All key bindings can be customized in the View > Key Bindings menu.

## Project Structure

```
summit/
├── src/
│   ├── main.rs                 # Entry point
│   ├── app.rs                  # CelesteMapEditor app implementation
│   ├── map/
│   │   ├── mod.rs              # Module exports
│   │   ├── loader.rs           # Map loading/saving functions
│   │   └── editor.rs           # Map editing functions
│   ├── ui/
│   │   ├── mod.rs              # Module exports
│   │   ├── render.rs           # Rendering functions
│   │   ├── dialogs.rs          # UI dialogs (open, save, etc.)
│   │   └── input.rs            # Input handling
│   └── config/
│       ├── mod.rs              # Module exports
│       └── keybindings.rs      # Key bindings management
├── Cargo.toml
└── README.md
```

## Acknowledgments

- [Cairn](https://github.com/Aqu1tain/cairn) - Celeste Map Encoder/Decoder
- [egui](https://github.com/emilk/egui) - Immediate mode GUI library for Rust

## License

MIT License