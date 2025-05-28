---
title: "Tutorials"
description: "Step-by-step guides to help you learn Cloacina"
weight: 20
reviewer: "dstorey"
review_date: "2024-03-19"
---

# Tutorials

Welcome to the Cloacina tutorials. These guides will help you get started with Cloacina by walking you through specific tasks step by step.

## Prerequisites

Before starting any tutorial, ensure you have the following installed:

- Git
- Docker Compose
- Angreal (`pip install angreal`)

## Available Tutorials

{{< toc-tree >}}

## How to Use These Tutorials

Each tutorial is designed to be followed in sequence. They build upon each other to give you a complete understanding of how to use Cloacina effectively.

## Running Tutorials with Angreal

To get started with a tutorial:

1. Clone the repository:
```bash
git clone https://github.com/colliery-io/cloacina.git
cd cloacina
```

2. The tutorials are stored in the `docs/content/tutorials` directory, with their corresponding implementations in the `examples` directory. Each tutorial is a separate markdown file that you can also read directly, and its implementation can be found in a matching directory under `examples`.

3. Run the tutorial using angreal:
```bash
angreal tutorial <tutorial-number>
```

For example, to run the "First Task" tutorial:

```bash
angreal tutorial 01
```

{{< hint type=tip >}}
You can run the tutorials from any directory - you don't need to be in the cloned repository. The `angreal` command will handle everything for you.
{{< /hint >}}

The tutorial number corresponds to the order in the table of contents above.
