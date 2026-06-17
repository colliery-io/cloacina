---
title: "Embed the Library"
description: "Integrate the Cloacina engine into your own Rust or Python application as a library."
weight: 10
aliases:
  - "/python/"
  - "/python/workflows/"

---

# Embed the Library

Run orchestration **inside your application**. You add a dependency — `cloacina`
(Rust) or `cloaca` (Python) — and the engine runs in your process, backed by a
database you already operate. No separate service to stand up.

This is a first-class, production-legitimate way to run Cloacina. If instead you
want to operate Cloacina as a shared, multi-tenant service, see
**[Run the Service]({{< ref "/service" >}})**. For what the two share — the core
objects you author against — see **[Engine & Primitives]({{< ref "/engine" >}})**.

**Who this is for:** application developers embedding the engine as a library
inside their own Rust or Python code.

**Prerequisites:** an existing Rust or Python project to add the dependency to,
and a database you already operate (SQLite is fine to start).

{{< toc-tree >}}
