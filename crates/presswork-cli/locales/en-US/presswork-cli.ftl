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

problem-on-page = Page { $page }: { $problem }
problem-resolution = The image has { $ppi } ppi. The profile { $profile } needs { $minimum } ppi.
problem-alt = The image has no alternative text.
problem-font = The font family { $family } is not available. Typst uses a different font.
problem-glyphs = No font has these characters: { $characters }
problem-page-size = The page is { $width } × { $height } { $unit }. The profile { $profile } needs { $needed_width } × { $needed_height } { $unit }.
problem-color = The document uses the color { $color }. The profile { $profile } permits only { $permitted }.
problem-gradient = A gradient uses the color space { $space }. The profile { $profile } permits only { $permitted }.
problem-widow = The last line of a paragraph is alone at the top of a page or column.
problem-orphan = The first line of a paragraph is alone at the bottom of a page or column.
problem-hyphens = { $lines } lines in sequence end with a hyphen.
problem-short-line = The last line of a paragraph has only { $characters ->
    [one] 1 character
   *[other] { $characters } characters
}.
problem-loose-line = The word spaces in this line are { $spacing }% of their normal width.
problem-spelling = The word “{ $word }” is not in the dictionary for { $language }.
spelling-missing = Presswork has no dictionary for { $language }, so it did not check the spelling of this text.
colors-cmyk = CMYK, gray, and spot colors
colors-gray = gray colors

export-outside = The profile { $profile } writes { $path }. This path is outside the project folder.
export-failed = Presswork cannot export the profile { $profile }. { $error }
export-write-failed = Presswork cannot write { $path }. { $error }
