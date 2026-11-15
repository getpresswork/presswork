app-title = Presswork

workspace-plan = Plan
workspace-research = Research
workspace-write = Write
workspace-translate = Translate
workspace-review = Review
workspace-typeset = Typeset
workspace-publish = Publish

workspace-plan-description = This workspace shows the tasks, the schedules, the budgets, and the reports of your publications.
workspace-research-description = This workspace shows the team library, the annotations, and the research notes.
workspace-write-description = This workspace shows the outline, the editor, and the preview of your project.
workspace-translate-description = This workspace shows the source text and its translation side by side.
workspace-review-description = This workspace shows the proofs, the comments, the tickets, and the suggestions.
workspace-typeset-description = This workspace shows the page design tools, the fonts, and the preflight results.
workspace-publish-description = This workspace shows the export profiles, the validation results, and the releases.

status-no-project = No project open
status-project = { $name } · { $file }
status-not-saved = Not saved

open-folder = Open folder
open-folder-title = Open a project folder

project-error-no-manifest = This folder is not a Presswork project. A project folder contains a presswork.toml file.
project-error-invalid-manifest = The presswork.toml file is not correct: { $problem }
project-error-main-outside = The main file must be in the project folder.
project-error-main-missing = The main file { $file } does not exist.
project-error-read = Presswork cannot read { $file }.
save-error = Presswork cannot save { $file }.
status-position = Line { $line }, column { $column }
status-compiled = Compiled in { $milliseconds } ms

preview-empty = The preview shows the pages after the first compilation.
preview-error = Line { $line }: { $message }
preview-error-without-line = { $message }

logs-toggle = Logs
logs-title = Logs
logs-level-error = Error
logs-level-warn = Warning
logs-level-info = Information
logs-level-debug = Debug
logs-level-trace = Trace
logs-column-time = Time
logs-column-level = Level
logs-column-area = Area
logs-column-message = Message
logs-find = Find in the logs
logs-empty = No log entries agree with the filter.

status-typst = Typst { $version }
status-typst-missing = Typst { $pin } is not installed
status-typst-invalid = The Typst pin is not valid

toolchains-title = Typst versions
toolchains-installed = Installed
toolchains-available = Available on GitHub
toolchains-built-in = Built in
toolchains-in-use = In use
toolchains-use = Use for this project
toolchains-remove = Remove
toolchains-install = Install
toolchains-installing = Installing { $percent }%
toolchains-loading = Presswork gets the list of versions.
toolchains-load-error = Presswork cannot get the list of versions. { $error }
toolchains-retry = Try again
toolchains-all-installed = All available versions are installed.
toolchains-size = { $megabytes } MB
toolchains-project-none = Open a project to select its Typst version.
toolchains-project-built-in = This project uses the built-in Typst { $version }.
toolchains-project-managed = This project uses Typst { $version } for exports. The preview uses the built-in Typst { $embedded }.
toolchains-project-missing = This project pins Typst { $pin }. Install a version that agrees with the pin.
toolchains-project-invalid = The Typst pin "{ $pin }" in presswork.toml is not valid.
toolchains-install-error = Typst { $version } is not installed. { $error }
toolchains-remove-error = Typst { $version } is not removed. { $error }
toolchains-pin-error = Presswork cannot write the pin. { $error }
toolchains-no-folder = Presswork cannot find a folder for the Typst versions.
