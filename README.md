# Paintbrush

Tools for agents (and humans) for interacting with [Canvas LMS](https://github.com/instructure/canvas-lms).

If the Human is the artist, then this is the paintbrush for interacting with Canvas.

## What this is

A Rust CLI that wraps the Canvas LMS API into commands that are easy for both
humans and coding agents to drive. The CLI's own `--help` output is written to
be self-sufficient — an agent should be able to discover everything it needs
(commands, arguments, auth setup) from `paintbrush --help` alone, without
reading source.

## Scope

This is a dogfooding project — features get added incrementally, as I need
them for my own Canvas workflows, not as a push to cover the whole Canvas
API. If there's something you want that isn't here, open an Issue to
request it, or contribute it directly via PR.

## Repo layout

- `src/` — the CLI itself (Rust)
- `ref/canvas` — submodule, full [canvas-lms](https://github.com/instructure/canvas-lms) source, for reference
- `ref/canvas_android` — submodule, full [canvas-android](https://github.com/instructure/canvas-android) source, for reference

The `ref/` submodules exist so real Canvas behavior can be checked when the
public API docs are ambiguous. Prefer the [Canvas REST API
docs](https://canvas.instructure.com/doc/api/) over reading `ref/` source;
only dig into `ref/` when the docs don't answer the question.

## Getting the ref submodules

```sh
git submodule update --init --recursive
```

## Building

```sh
cargo build
```

## Opening Canvas resources

When a user asks an agent to open a Canvas resource in their browser, pass
`--web` to the resource's `view` command. The command opens the user's default
browser; returning the URL alone is not the browser-opening action.

```sh
paintbrush assignment view ASSIGNMENT_ID --course COURSE_ID --web
paintbrush announcement view ANNOUNCEMENT_ID --course COURSE_ID --web
paintbrush course view COURSE_ID --web
paintbrush page https://canvas.example/courses/COURSE_ID/quizzes/QUIZ_ID view --web
```
