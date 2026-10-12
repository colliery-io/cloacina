---
title: "Packaging"
description: "The distributable unit: the .cloacina package."
sidebar:
  order: 40
---

The **`.cloacina` package** is the distributable unit — a `package.toml` plus
source (a Python module tree, or Rust compiled on load) — that lets the same
workflow or computation graph move between the embedded library, the daemon, and
the server unchanged.

- [Package](package/)
