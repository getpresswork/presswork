app-title = Presswork

window-minimize = Minimize
window-maximize = Maximize
window-restore = Restore
window-close = Close

menu-open-folder = Open folder…
menu-connect = Connect to a server…
menu-save = Save
menu-typst = Typst versions
menu-logs = Logs
menu-quit = Quit
shortcut = Ctrl+{ $key }
shortcut-mac = Cmd+{ $key }

server-title = Connect to a server
server-address = Server address
server-address-placeholder = https://press.example.com
server-email = Email address
server-password = Password
server-sign-in = Sign in
server-signing-in = Signing in…
server-cancel = Cancel
server-close = Close
server-browse-title = Open a server document
server-loading = Presswork gets the projects.
server-no-projects = Your organizations have no projects. An administrator makes the projects.
server-project = { $organization } · { $project }
server-make-main = Make main.typ
server-problem-address = The server address must start with http:// or https://.
server-problem-unreachable = Presswork cannot connect to the server. Make sure that the address is correct.
server-problem-status = The server answered with the status { $status }.

tasks-title = Tasks · { $project }
tasks-loading = Presswork gets the tasks.
tasks-empty = This project has no tasks. Add the first task.
tasks-placeholder = New task
tasks-add = Add
tasks-in-progress = In progress

library-title = Library · { $organization }
library-add = Add PDF…
library-search-placeholder = Find words in the library
library-search = Find
library-loading = Presswork gets the works of the library.
library-empty = The library has no works. Add the first PDF file.
library-no-hits = No page has all these words.
library-hit = { $title } · p. { $page }
library-cite = Cite
library-add-edition = Add edition…
library-other-edition = Not the standard edition
library-standard = Standard edition
library-make-standard = Make standard
library-translators = Translated by { $names }
library-page-numbers = Page numbers…
library-page-map-title = Page numbers of { $title }
library-page-map-empty = The page numbers come from the PDF file.
library-correction = PDF page { $page } is printed page { $label }
library-correction-page = PDF page
library-correction-start = Printed page
library-style-arabic = 1, 2, 3
library-style-lower-roman = i, ii, iii
library-style-upper-roman = I, II, III
library-correction-add = Add correction
library-correction-remove = Remove
library-page-map-save = Save
library-pages = { $pages ->
    [one] 1 page
   *[other] { $pages } pages
}, { $text } with text
library-page-count = { $pages ->
    [one] 1 page
   *[other] { $pages } pages
}
library-reading = The server reads the text of the pages.
library-recognizing = The server finds the text of the scanned pages with OCR.
library-failed = The server cannot read the text of the file.
library-no-file = No file
library-form-title = Title
library-form-creators = Creators
library-form-creators-example = Tolstoy, Leo; Maude, Louise
library-form-year = Year
library-form-publisher = Publisher
library-form-language = Language
library-form-translators = Translators
library-form-translators-example = Maude, Louise; Maude, Aylmer
library-form-new-edition = New edition of { $title }: { $file }
library-form-key = Citation key
library-form-add = Add
library-form-cancel = Cancel
library-uploading = Presswork uploads the file and reads its pages.
library-too-large = The file is larger than 64 MiB. Select a smaller file.

chat-toggle = Chat
chat-title = Chat · { $organization }
chat-empty = No messages yet. Write the first message.
chat-placeholder = Write a message
chat-send = Send

unsaved-title = Save the changes to { $file }?
unsaved-body = If you do not save, the changes are lost.
unsaved-save = Save
unsaved-discard = Do not save
unsaved-cancel = Cancel

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
status-live-connecting = { $path } · Connecting
status-live = { $path } · Live
status-live-offline = { $path } · Offline

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
preview-needs-typst = This project needs Typst { $pin }. Install it from the Typst item in the status bar.
preview-invalid-pin = The Typst pin in presswork.toml is not valid. Correct the pin to see the preview.
preview-program-error = Presswork cannot run the Typst program. { $error }
preview-jump-other-file = The text is in the file { $file }. The editor shows only the main file.
preview-jump-needs-engine = A click on the pages goes to the text only with the built-in Typst engine. This project uses a different Typst version.
definition-other-file = The definition is in the file { $file }. The editor shows only the main file.
definition-in-library = { $name } is part of the Typst library.
preview-window-title = Preview · { $name }
preview-window-title-scratch = Preview

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
toolchains-project-managed = This project uses Typst { $version } for the preview and the exports. Completion and other editor help need the built-in Typst { $embedded }.
toolchains-project-missing = This project pins Typst { $pin }. Install a version that agrees with the pin.
toolchains-project-invalid = The Typst pin "{ $pin }" in presswork.toml is not valid.
toolchains-install-error = Typst { $version } is not installed. { $error }
toolchains-remove-error = Typst { $version } is not removed. { $error }
toolchains-pin-error = Presswork cannot write the pin. { $error }
toolchains-no-folder = Presswork cannot find a folder for the Typst versions.
