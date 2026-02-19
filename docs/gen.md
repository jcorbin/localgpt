---
sidebar_position: 14
---

# LocalGPT Gen (Experimental)

:::warning Experimental
LocalGPT Gen is an early-stage experimental feature. Scene output quality depends heavily on the LLM's spatial reasoning. Consider it a proof of concept.
:::

**LocalGPT Gen** is a built-in world generation mode. You type natural language, and the AI builds explorable worlds — geometry, materials, lighting, and camera. All inside the same single Rust binary, powered by [Bevy](https://bevyengine.org/).

## Demo Videos

<iframe width="100%" height="400" src="https://www.youtube.com/embed/n18qnSDmBK0" title="LocalGPT Gen Demo" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

<br/>

<iframe width="100%" height="400" src="https://www.youtube.com/embed/cMCGW7eMUNE" title="LocalGPT Gen Demo" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

## Installation

```bash
# Install the standalone Gen binary
cargo install localgpt-gen

# Or from a source checkout
cargo install --path crates/gen
```

## Usage

```bash
# Interactive mode — type prompts in the terminal
localgpt-gen

# Start with an initial prompt
localgpt-gen "create a heart outline with spheres and cubes"

# Load an existing glTF/GLB scene
localgpt-gen --scene ./scene.glb

# Verbose logging
localgpt-gen --verbose

# Combine options
localgpt-gen -v -s ./scene.glb "add warm lighting"

# Custom agent ID (default: "gen")
localgpt-gen --agent my-gen-agent
```

The agent receives your prompt and iteratively builds a world — spawning shapes, adjusting materials, positioning the camera, and taking screenshots to course-correct. Type `/quit` or `/exit` in the terminal to close.

## Gen Tools

The gen agent has access to 11 specialized tools:

| Tool | Description |
|------|-------------|
| `gen_scene_info` | Get complete scene hierarchy |
| `gen_screenshot` | Capture viewport screenshot |
| `gen_entity_info` | Get detailed info about a named entity |
| `gen_spawn_primitive` | Spawn geometric primitives (sphere, cube, cylinder, torus, etc.) |
| `gen_modify_entity` | Modify entity transform, material, or visibility |
| `gen_delete_entity` | Remove an entity and its children |
| `gen_set_camera` | Position and orient the camera |
| `gen_set_light` | Configure scene lighting |
| `gen_set_environment` | Set background color and ambient light |
| `gen_spawn_mesh` | Spawn custom mesh geometry |
| `gen_export_screenshot` | Export high-res image to file |

## Architecture

Bevy requires ownership of the main thread (macOS windowing/GPU requirement), so LocalGPT Gen uses a split-thread architecture:

- **Main thread** — Bevy engine runs the render loop and processes scene commands
- **Background thread** — Tokio runtime runs the agent loop, making LLM calls and issuing tool commands
- **Communication** — Async mpsc channels bridge the two threads

```
┌─────────────────────┐     mpsc channels     ┌─────────────────────┐
│    Main Thread      │◄─────────────────────►│  Background Thread  │
│                     │                        │                     │
│  Bevy Engine        │   ToolRequest ──►      │  Tokio Runtime      │
│  - Rendering        │   ◄── ToolResult       │  - Agent Loop       │
│  - Scene Graph      │                        │  - LLM API Calls    │
│  - Window/GPU       │                        │  - Tool Execution   │
└─────────────────────┘                        └─────────────────────┘
```

## Current Limitations

- Visual output depends on the LLM's spatial reasoning ability
- Only geometric primitives are supported (no imported 3D models yet)
- Requires a GPU-capable display for rendering

## Share Your Creations

Created something awesome with LocalGPT Gen? We'd love to see it! Join the community on [Discord](https://discord.gg/yMQ8tfxG) and share your world generation results — showcase your creative prompts, stunning scenes, and experimental ideas with fellow LocalGPT users.
