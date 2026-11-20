severity-error = error
severity-warning = warning
hint = hint: { $hint }

project-not-found = The folder { $folder } is not a Presswork project. A project folder contains a presswork.toml file.
project-invalid = The presswork.toml file is not correct: { $problem }
project-main-outside = The main file { $file } must be in the project folder.
project-main-missing = The main file { $file } does not exist.
project-read = Presswork cannot read { $file }. { $error }

pin-invalid = The Typst pin { $pin } is not valid.
pin-not-built-in = The project needs Typst { $pin }. The command-line tool has only Typst { $version }, so it cannot compile the project.

profile-unknown = The project has no export profile with the name { $name }.

problem-resolution = Page { $page }: The image has { $ppi } ppi. The profile { $profile } needs { $minimum } ppi.
problem-alt = Page { $page }: The image has no alternative text.

export-outside = The profile { $profile } writes { $path }. This path is outside the project folder.
export-failed = Presswork cannot export the profile { $profile }. { $error }
export-write-failed = Presswork cannot write { $path }. { $error }
