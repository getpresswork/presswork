app-title = Presswork

window-minimize = Minimize
window-maximize = Maximize
window-restore = Restore
window-close = Close

menu-new-project = New project…
menu-open-folder = Open folder…
menu-connect = Connect to a server…
menu-save = Save
menu-typst = Typst versions
menu-logs = Logs
menu-quit = Quit
shortcut = Ctrl+{ $key }
shortcut-mac = Cmd+{ $key }
shortcut-shift = Ctrl+Shift+{ $key }
shortcut-shift-mac = Shift+Cmd+{ $key }

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
new-project-title = Select or make a folder for the new project

project-error-no-manifest = This folder is not a Presswork project. A project folder contains a presswork.toml file.
project-error-invalid-manifest = The presswork.toml file is not correct: { $problem }
project-error-main-outside = The main file must be in the project folder.
project-error-main-missing = The main file { $file } does not exist.
project-error-read = Presswork cannot read { $file }.
save-error = Presswork cannot save { $file }.
suggestions-moved = The suggestions of { $file } do not agree with the file, because the file changed outside Presswork. Presswork moved them to { $path }.
suggestions-read-error = Presswork cannot read the suggestions of { $file }. { $error }
suggest-toggle = Suggest
suggest-toggle-tip = Record your changes as suggestions ({ $shortcut })
suggestion-accept = Accept the suggestion
suggestion-reject = Reject the suggestion
suggestion-accept-all = Accept all suggestions
suggestion-reject-all = Reject all suggestions
suggestion-accept-detail = Presswork keeps the inserted text and removes the deleted text.
suggestion-reject-detail = Presswork removes the inserted text and keeps the deleted text.
suggestion-insertion-tip = Insertion from { $author } · { $time }
suggestion-deletion-tip = Deletion from { $author } · { $time }
suggestion-unknown-author = an unknown author
status-position = Line { $line }, column { $column }
status-compiled = Compiled in { $milliseconds } ms

preview-empty = The preview shows the pages after the first compilation.
preview-error = Line { $line }: { $message }
preview-error-without-line = { $message }
preview-needs-typst = This project needs Typst { $pin }. Install it from the Typst item in the status bar.
preview-invalid-pin = The Typst pin in presswork.toml is not valid. Correct the pin to see the preview.
preview-program-error = Presswork cannot run the Typst program. { $error }
preview-jump-other-file = The text is in the file { $file }. Open the project to edit it.
preview-jump-needs-engine = A click on the pages goes to the text only with the built-in Typst engine. This project uses a different Typst version.
definition-other-file = The definition is in the file { $file }. Open the project to edit it.
definition-in-library = { $name } is part of the Typst library.
preview-window-title = Preview · { $name }
preview-window-title-scratch = Preview

label-kind-heading = Heading
label-kind-figure = Figure
label-kind-table = Table
label-kind-code = Code
label-kind-equation = Equation
label-kind-other = Labeled element
completion-label-page = { $kind }, page { $page }
completion-label-text = { $place }: { $text }

find-placeholder = Find
replace-placeholder = Replace
find-count = { $current } of { $total }
find-total = { $total ->
    [one] 1 result
   *[other] { $total } results
}
find-none = No results
find-invalid = The pattern is not valid
find-match-case = Match case
find-whole-word = Whole word
find-pattern = Regular expression
find-previous = Previous result (Shift+Enter)
find-next = Next result (Enter)
find-close = Close (Escape)
find-toggle-replace = Show or hide Replace
replace-one = Replace
replace-all = Replace all

files-title = Files
files-toggle = Files
files-not-text = Presswork cannot open { $file }, because it is not a text file.
files-read-error = Presswork cannot read { $file }.

outline-title = Outline
outline-toggle = Outline
outline-empty = The document has no headings.
outline-needs-engine = The outline is available only with the built-in Typst engine.

publish-title = Export
publish-default-note = The project has no export profiles. Presswork uses one PDF profile. To add profiles, add [[export]] tables to presswork.toml.
publish-no-project = Open a project to export it.
publish-export = Export
publish-exporting = Exporting…
publish-exported = { $count ->
    [one] Presswork wrote { $file } and the build report { $report }.
   *[other] Presswork wrote { $count } files in { $folder } and the build report { $report }.
}
publish-project-folder = the project folder
publish-needs-engine = The export is available only with the built-in Typst engine.
publish-no-pages = The document has errors. Correct the errors, then export again.
publish-stopped = The export stopped before the end. Export again.
publish-outside = The file { $path } is outside the project folder. Presswork writes export files only in the project folder.
publish-failed = Presswork cannot export the document. { $error }
publish-write-failed = Presswork cannot write { $path }. { $error }
publish-format-pdf = PDF
publish-format-png = PNG, { $ppi } ppi
publish-format-svg = SVG
typeset-title = Typesetting QA
typeset-empty = Presswork found no problems in the layout.
typeset-needs-engine = The QA report is available only with the built-in Typst engine.
typeset-source = Show the source in the Write workspace
typeset-count = { $count ->
    [one] 1 problem in the layout
   *[other] { $count } problems in the layout
}

problem-on-page = Page { $page }: { $problem }
problem-in-profile = Export profile “{ $profile }”: { $problem }
problem-resolution = The image has { $ppi } ppi. The profile needs { $minimum } ppi.
problem-alt = The image has no alternative text.
problem-font = The font family { $family } is not available. Typst uses a different font.
problem-glyphs = No font has these characters: { $characters }
problem-page-size = The page is { $width } × { $height } { $unit }. The profile needs { $needed_width } × { $needed_height } { $unit }.
problem-color = The document uses the color { $color }. The profile permits only { $permitted }.
problem-gradient = A gradient uses the color space { $space }. The profile permits only { $permitted }.
problem-widow = The last line of a paragraph is alone at the top of a page or column.
problem-orphan = The first line of a paragraph is alone at the bottom of a page or column.
problem-hyphens = { $lines } lines in sequence end with a hyphen.
problem-short-line = The last line of a paragraph has only { $characters ->
    [one] 1 character
   *[other] { $characters } characters
}.
problem-loose-line = The word spaces in this line are { $spacing }% of their normal width.
problem-grid = The baseline of this line is { $offset } { $unit } { $direction ->
    [above] above
   *[below] below
} the baseline grid.
colors-cmyk = CMYK, gray, and spot colors
colors-gray = gray colors

proofreading-spelling = The word “{ $word }” is not in the dictionary for { $language }.
proofreading-style = The house style does not permit “{ $text }”.
proofreading-replacement = Push { $shortcut } to change the text to “{ $replacement }”.
proofreading-suggestions = Push { $shortcut } to show the corrections.
proofreading-no-corrections = Presswork has no corrections for the text at the cursor.
proofreading-add-word = Add “{ $word }” to the word list
proofreading-add-word-detail = Presswork adds the word to the file presswork-words.txt in the project folder.
proofreading-add-word-error = Presswork cannot write presswork-words.txt. { $error }
proofreading-add-personal = Add “{ $word }” to the personal dictionary
proofreading-add-personal-detail = Presswork adds the word to your personal dictionary. The word is then correct in all your projects.
proofreading-add-personal-error = Presswork cannot write your personal dictionary. { $error }
style-error-file = The house style file { $file } is not valid. { $problem }
style-error-missing = Rule { $rule } in { $file } has no text and no pattern.
style-error-both = Rule { $rule } in { $file } has a text and a pattern. Use only one of them.
style-error-empty = Rule { $rule } in { $file } has an empty text or pattern.
style-error-pattern = The pattern of rule { $rule } in { $file } is not valid: { $problem }.

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

dictionaries-title = Dictionaries
dictionaries-install = Download
dictionaries-remove = Remove
dictionaries-install-error = Presswork cannot download the { $language } dictionary. { $error }
dictionaries-remove-error = Presswork cannot remove the { $language } dictionary. { $error }
dictionary-en-US = English (United States)
dictionary-en-GB = English (United Kingdom)
dictionary-de-DE = German (Germany)
dictionary-de-AT = German (Austria)
dictionary-de-CH = German (Switzerland)
dictionary-fr-FR = French (France)
dictionary-es-ES = Spanish (Spain)
dictionary-it-IT = Italian (Italy)
dictionary-nl-NL = Dutch (Netherlands)
dictionary-pt-BR = Portuguese (Brazil)
dictionary-pt-PT = Portuguese (Portugal)
dictionary-pl-PL = Polish (Poland)
dictionary-ru-RU = Russian (Russia)
dictionary-da-DK = Danish (Denmark)
