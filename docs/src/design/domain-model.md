# Domain model

This chapter tells the main records of Presswork and their relations.

## Structure

```text
Organization
+-- Teams ............... groups of users
+-- Roles ............... named sets of permissions
+-- Role assignments .... roles for users and teams, in a scope
+-- Collections ......... series, periodicals, journals
|   +-- Publications
+-- Publications
|   +-- Project ......... the Typst sources
|   +-- Components ...... chapters, articles, sections
|   +-- Workflow instances
|   +-- Revisions
|   +-- Issues and tasks
|   +-- Checklists
+-- Workflow definitions
+-- Checklist templates
+-- Routines
+-- Reports
```

## Organizations and people

| Record | Description |
| --- | --- |
| User | A person with an account on the server. |
| Organization | The owner of all other records. A server can have many organizations. |
| Team | A group of users in an organization, for example "Copyedit desk". |
| Role | A named set of permissions. |
| Role assignment | A role for a user or a team, in a scope: the organization, a collection, a publication, or a component. |

## Publications

| Record | Description |
| --- | --- |
| Collection | An optional group of publications: a book series, a periodical, a journal, or a documentation set. |
| Publication | One work that you can publish: a book, an edition of a periodical, a manual, or a paper. It has metadata and export profiles. |
| Project | The Typst sources of a publication. Each publication has one project. |
| Component | A part of a publication with its own workflow stage: a chapter, an article, or a section. It refers to one or more files of the project. |

The publication kind sets the default labels. For example, the periodical template calls a publication "Edition" and a component "Article". Each organization can change these labels.

Presswork uses the word "edition" for one number of a periodical. The word "issue" always means an item in the issue list.

## Issues and tasks

Issues and tasks have a common base:

- A title, a description, and labels.
- A status, assignees, and watchers.
- Comments and attachments.
- Links to publications, components, revisions, and other items.

Each kind adds its own data:

| Kind | Purpose | Data |
| --- | --- | --- |
| Issue | A problem, a question, or a correction in the content. Author queries are issues. | Anchor, issue type, severity, resolution |
| Task | Planned work for a person. | Start date, due date, estimate, dependencies, subtasks, checklists |

Each organization can add issue types, for example "Query", "Correction", or "Fact check".

## Checklists

A checklist template is a list of items. Each item has:

- A text, for example "All images have alternative text".
- A kind: manual or automatic.
- A flag that tells if the item is necessary.

A person signs off a manual item. Presswork sets an automatic item from the result of a check. Examples of checks are a preflight rule, a veraPDF validation, and "No open queries in this component".

A checklist instance records who completed each item, and when. A checklist can attach to a task, a component, a publication, or a workflow transition.

## Workflows

A workflow definition is a state machine with these parts:

- **Stages**, for example Manuscript, Edit, Copyedit, Typeset, Proof, and Final. Each stage has a category: not started, active, or done.
- **Transitions** between stages. A transition can have guards:
  - A role that can do the transition.
  - Checklists that must be complete.
  - Approvals: a number of persons with a given role must approve the current revision.
  - Preflight rules that must pass.
- **Actions**: routines that start when a stage starts or stops.

A workflow instance attaches to a publication or to a component. It records the current stage and the history of all transitions.

Each definition has versions. An instance keeps the definition version that it started with. An administrator can move instances to a new version.

The `presswork-domain` crate contains the transition rules as pure functions.

## Workflow templates

Presswork supplies a workflow template for each publication kind:

| Kind | Stages |
| --- | --- |
| Book | Manuscript, Development edit, Copyedit, Typeset, First proof, Corrections, Final proof, Print and ebook |
| Periodical | Commission, Draft, Edit, Layout, Proof, Approved, Published |
| Technical documentation | Draft, Technical review, Language review, Approved, Released |
| Academic | Submitted, Peer review, Revision, Accepted, Copyedit, Proof, Published |

## Routines

A routine is a sequence of steps. A schedule or an event starts it.

| Part | Examples |
| --- | --- |
| Trigger | Each Monday at 09:00. A component goes into the Proof stage. A person records a revision. |
| Steps | Make tasks from a template. Attach a checklist. Move a stage. Compile and export. Run preflight. Validate with veraPDF. Send a notification. Call a webhook. |
| Scope | An organization, a collection, or a publication. |

For example, a newsletter routine runs each Monday. It makes the next edition from a template, makes a task for each article, and sends a notification to the editor.

Each run records its steps, its output files, and its log.

## Reports

A report is a saved query and a layout for its results. Presswork supplies these reports:

- Publication status: each component and its stage.
- Schedule: milestones, due dates, and late tasks.
- Workload: the open tasks of each person.
- Issue trend: the open and closed issues in a period.
- Content metrics: the words and pages of each component over time.
- Preflight summary: the problems of each publication.

Reports show in the Plan workspace. Presswork can export a report as CSV, or as PDF through a Typst template. Thus teams can make their own report layouts in Typst.
