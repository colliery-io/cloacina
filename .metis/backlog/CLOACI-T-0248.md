---
id: doc-review
level: task
title: "Doc Review"
short_code: "CLOACI-T-0248"
created_at: 2026-03-25T02:56:00.793599+00:00
updated_at: 2026-03-25T02:56:00.793599+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"


exit_criteria_met: false
initiative_id: NULL
---

1\. **Tutorials** — Learning-oriented guides that take the reader through

a series of steps to complete a project. These should be reproducible,

focused on learning (not the end result), and assume no prior knowledge

of the system.

2\. **How-To Guides** — Task-oriented recipes that show how to solve

specific problems. These assume the reader already has basic knowledge

and needs to accomplish something concrete. Each guide should have a

clear goal and actionable steps.

3\. **Reference** — Information-oriented technical descriptions of the

system's machinery. This includes APIs, configuration options, CLI

flags, data models, environment variables, and architecture. Reference

docs should be austere, accurate, and structured for lookup — not for

reading end-to-end.

4\. **Explanation** — Understanding-oriented discussion that clarifies

concepts, design decisions, trade-offs, and the "why" behind the system.

This is where architectural rationale, mental models, and context

belong.

\## Process

\### Phase 1: Deep Discovery

Before writing anything, thoroughly explore the entire codebase:

- Read every configuration file, entrypoint, and README

- Trace the main workflows end-to-end

- Identify all user-facing interfaces (CLI commands, APIs, config files,

environment variables)

- Map dependencies and integration points

- Understand the build, test, and deployment lifecycle

- Note any implicit knowledge or tribal conventions that aren't

documented

\### Phase 2: Documentation Plan

Produce an outline of every document you intend to write, organized by

Diátaxis category. For each document, include:

- Title

- Target audience

- Key topics covered

- Dependencies on other documents

\### Phase 3: Write Documentation

Write each document fully. Do not leave placeholders or TODOs. Every

document should be complete, accurate, and usable by its target

audience.

\### Phase 4: Review

Launch review agents in parallel to evaluate the documentation:

- **Accuracy Agent**: Cross-reference every claim against the actual

codebase. Flag any discrepancies between what the docs say and what the

code does.

- **Completeness Agent**: Identify gaps — features, flags, config

options, workflows, or edge cases that exist in the code but are missing

from the docs.

- **Clarity Agent**: Read each document from the perspective of its

target audience. Flag jargon without definition, unclear steps, missing

context, or assumed knowledge that isn't warranted.

- **Diátaxis Compliance Agent**: Verify each document stays in its lane

— tutorials don't drift into reference, how-to guides don't become

explanations, etc. Flag any category violations.

Incorporate all review feedback before finalizing.

\## Requirements

- Be exhaustive. Document every feature, option, and workflow — not just

the happy path.

- Include concrete examples with real values, not abstract placeholders.

- Cross-link between documents (e.g., a tutorial should link to the

relevant reference docs).

- Surface implicit knowledge — the things a new team member would

struggle with that aren't obvious from the code alone.
