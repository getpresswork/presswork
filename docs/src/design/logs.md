# Logs and diagnostics

This chapter tells how Presswork records events, so that persons can find the cause of a problem quickly.

## Structured logs

- All crates write structured logs with `tracing`. Each entry has a time, a level, an area, a message, and fields, for example the project, the file, or the job.
- Each operation has a span. Thus the log shows which entries belong to one operation, for example one export.
- Logs never contain document text, passwords, tokens, or keys. The log layer replaces the value of each field with a secret name with "[removed]". Secret names end in, for example, `password`, `token`, `key`, `secret`, `text`, or `content`.
- The desktop application writes its logs as JSON lines to files in its data folder. It starts a new file each day and keeps the files of the last 14 days.
- The server writes JSON lines to the standard output.
- The `PRESSWORK_LOG` environment variable sets the log filter at start, for example `info,presswork_typst=debug`.
- An administrator can change the log level of each area while the program runs.

## Log panel

- The log panel shows the log entries live.
- Filters select entries by level, area, project, and time.
- A search finds text in the entries.
- Each entry links to its object, for example a file, a task, a job, or a build report.

## Build reports

Presswork keeps a build report for each compilation and each export. A build report contains:

- The Typst version and the engine: embedded or managed toolchain.
- The inputs and the variant.
- The fonts, with their files.
- The packages, with their versions.
- The warnings.
- The time of each step.
- The digest of each output file.

You can compare two build reports. Presswork shows the differences in fonts, packages, inputs, and pages. Each release export keeps its build report with its revision.

## Support bundle

- The command "Make support bundle" collects the logs, the versions, the settings, and the system data.
- Presswork removes personal data from the bundle.
- Presswork shows the content of the bundle before you send it. It sends nothing automatically.

## OpenTelemetry

- The server can send traces, metrics, and logs to an OpenTelemetry collector of the operator, for example for Grafana or Jaeger.
- The AI gateway adds a span for each AI request, with the tokens and the time.
- Nothing goes to the Presswork project.
