# Tasks and schedules

This chapter tells how teams plan, assign, and track the jobs of a publication.

## Tasks and tickets

A task is an assignment for a person. A ticket is a problem, a question, or a correction in the content. Refer to [Domain model](domain-model.md).

## Views

All views show the same records. A change in one view shows immediately in all other views.

| View | Content |
| --- | --- |
| List | Tasks and tickets in rows, with filters, sorts, and groups. |
| Board | Cards in columns, for example one column for each stage or each status. |
| Timeline | Tasks and milestones on a time axis, with dependencies and the critical path. |
| Calendar | Due dates, milestones, and the dates of issues, by day, week, or month. |
| Table | A grid with custom fields, changes to many rows at a time, and calculated columns. |

A person can save a view with its filters. A team can share saved views.

The calendar has an ICS feed. Thus other calendar applications can show the Presswork dates. The feed address contains a secret token.

## Planning

- A task can have dependencies on other tasks. A dependency has a type: finish to start, start to start, or finish to finish.
- A milestone is a date that marks the end of a phase, for example "Proofs to author".
- Each publication kind has schedule templates. A template makes the tasks, the dependencies, and the milestones from one date, for example the print date.
- If a task is late, Presswork calculates a new schedule from the dependencies. It shows the new schedule as a proposal. A production manager accepts or changes the proposal.
- The timeline shows the critical path: the sequence of tasks that controls the end date.

## Progress

Presswork calculates the progress from the real sources:

- The words and pages of each chapter, against the targets.
- The workflow stage of each chapter.
- The open tickets of each chapter.
- The difference between the planned dates and the real dates.

## Time

- A person can start a timer on a task, or add a time entry manually.
- A timesheet shows the time entries of one person for a week or a month.
- A person or a role can have a rate. A time entry with a rate becomes a cost item in the budget.
- Time entries are visible only to the person, to production managers, and to persons with the `time.read` permission.

## Budgets and costs

- Each publication can have a budget with cost lines, for example copyedit, design, print, and licenses.
- A cost item is planned or actual. Actual costs come from time entries, from clearance fees, and from manual entries.
- Each budget has one currency. A cost in a different currency has a fixed exchange rate.
- The budget report shows the planned costs, the actual costs, and the difference.

## Rights and clearances

Clearances record the right to use a text or an image. Refer to [Production](production.md).
