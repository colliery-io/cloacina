---
title: "Documentation"
description: "How to write and maintain Cloacina documentation"
weight: 61
reviewer: "dstorey"
review_date: "2024-03-19"
---


This guide provides practical information about writing and maintaining documentation for the Cloacina project.

## Documentation Structure

Our documentation follows the [Diátaxis Framework](https://diataxis.fr/) and is organized into:
- `docs/content/tutorials/` - Step-by-step guides and examples that teach Cloacina features
- `docs/content/how-to/` - Task-oriented guides for specific operations
- `docs/content/reference/` - Technical reference and API documentation
- `docs/content/explanation/` - Conceptual documentation and deep dives

## Writing Guidelines

### General Principles
- Write clear, concise, and accurate documentation
- Use active voice and present tense
- Include practical examples where appropriate
- Keep documentation up-to-date with code changes
- Consider the reader's perspective and experience level

### API Documentation and Cross-Linking

When documenting API features or referring to API components in the documentation, use the `api-link` shortcode. It uses Rust's namespace syntax to create links:

`{{</* api-link path="path::to::component" */>}}`
Renders as: {{< api-link path="cloacina::models" type="module" >}}

You can also customize the display text:
`{{</* api-link path="path::to::component" display="Custom Text" */>}}`
Example: {{< api-link path="cloacina::models" type="module" display="Data Models" >}}

#### Item Types

The shortcode supports different Rust item types through the optional `type` parameter:

```markdown
{{< api-link path="cloacina::context::Context" type="struct" >}}    <!-- For structs -->
{{< api-link path="cloacina::task::Task" type="trait" >}}           <!-- For traits -->
{{< api-link path="cloacina::models" type="module" >}}              <!-- For modules -->
{{< api-link path="cloacina::error::Error" type="enum" >}}          <!-- For enums -->
{{< api-link path="cloacina::types::Result" type="type" >}}         <!-- For type aliases -->
{{< api-link path="cloacina::utils::format_error" type="fn" >}}     <!-- For functions -->
```

Available types:
- `struct` - For structs (default if not specified)
- `enum` - For enums
- `trait` - For traits
- `type` - For type aliases
- `fn` - For functions
- `module` - For modules (uses index.html)

These links will automatically stay up-to-date with the API documentation.

## Documentation Review Process

1. **Self-Review**
   - Check for technical accuracy
   - Verify all links work
   - Ensure examples are up-to-date
   - Review for clarity and completeness

2. **Peer Review**
   - Documentation changes should be reviewed by at least one other contributor
   - Focus on both technical accuracy and clarity
   - Consider the perspective of new users

## Tools and Resources

- Use the Hugo development server to preview documentation changes:
  ```bash
  hugo server -D
  ```
- Check the [Hugo documentation](https://gohugo.io/documentation/) for markdown syntax and shortcodes
- Review existing documentation for style and format consistency

## Need Help?

If you need assistance with documentation:
- Check existing documentation for examples
- Ask in the project's communication channels
- Review the [Diátaxis Framework](https://diataxis.fr/) for guidance on documentation types
