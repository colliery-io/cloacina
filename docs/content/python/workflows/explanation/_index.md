---
title: "Explanation"
description: "Architecture and design notes specific to the Python workflow surface."
weight: 40
---

# Python Workflow Explanation

Conceptual material — the why behind the Python surface. For the underlying Rust model, the [Rust workflow explanations]({{< ref "/workflows/explanation" >}}) apply verbatim; this section adds Python-specific concerns.

## In this section

- [Python Runtime Architecture]({{< ref "python-runtime-architecture" >}}) — PyO3 boundary, the `cloacina-python` crate split (CLOACI-T-0529 / CLOACI-T-0532), the GIL trade-off, and the FFI surface the Python module wraps.

## See also

- [Rust · Workflow Explanation]({{< ref "/workflows/explanation" >}}) — every doc here applies to Python as well; the Python module is a thin wrapper over the same runtime.
