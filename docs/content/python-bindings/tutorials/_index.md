---
title: "Cloaca Tutorials"
description: "Step-by-step guides to learn Cloaca"
weight: 10
reviewer: "automation"
review_date: "2025-01-07"
---

# Cloaca Tutorials

These tutorials will guide you through building workflows with Cloaca. Each tutorial builds upon the previous one to give you a complete understanding of the framework.

## Prerequisites

Before starting any tutorial, ensure you have:

- Python 3.9 or higher
- pip (Python package installer)
- Basic knowledge of Python
- A code editor of your choice

## Available Tutorials

{{< toc-tree >}}

## How to Use These Tutorials

Each tutorial is designed to be followed in sequence. They build upon each other to give you a complete understanding of how to use Cloaca effectively.

## Installation

For all tutorials, you'll need to install Cloaca:

{{< tabs "tutorial-install" >}}
{{< tab "SQLite (Recommended for tutorials)" >}}
```bash
pip install cloaca[sqlite]
```
{{< /tab >}}

{{< tab "PostgreSQL (Production)" >}}
```bash
pip install cloaca[postgres]
```
{{< /tab >}}
{{< /tabs >}}

{{< hint type="tip" title="Virtual Environment" >}}
We recommend using a virtual environment for the tutorials:

```bash
python -m venv tutorial-env
source tutorial-env/bin/activate  # On Windows: tutorial-env\Scripts\activate
pip install cloaca[sqlite]
```
{{< /hint >}}

## Tutorial Structure

Each tutorial follows this structure:

1. **Learning Objectives**: What you'll learn
2. **Prerequisites**: Required knowledge and setup
3. **Time Estimate**: Expected completion time
4. **Code Examples**: Working Python code
5. **Explanation**: Detailed breakdown of concepts
6. **Exercises**: Practice what you've learned
7. **Next Steps**: Where to go from here

## Getting Help

If you encounter issues:

- Check the [API Reference](/python-bindings/api-reference/) for detailed method documentation
- Review the [Examples](/python-bindings/examples/) for more code samples
- Consult the [How-to Guides](/python-bindings/how-to-guides/) for specific solutions

## Sample Code Repository

All tutorial code examples are available in the [examples directory](https://github.com/dstorey/cloacina/tree/main/examples) of the Cloacina repository.