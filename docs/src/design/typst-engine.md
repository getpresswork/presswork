# Typst engine

This chapter tells how Presswork compiles, renders, and exports Typst documents.

## Two engines

Presswork uses Typst in two ways:

| Engine | Use | Typst version |
| --- | --- | --- |
| Embedded engine | Live preview, exports, diagnostics, completion, tooltips, the outline, and jumps between source and pages | The version in this Presswork release, now 0.15 |
| Managed toolchain | Live preview, exports, and diagnostics of projects that pin a different Typst version | Each version that the toolchain manager installs |

The embedded engine links the Typst crates into Presswork. It gives access to the document model. Thus the interface can connect each element on a page to its source.

The managed toolchain runs the official Typst command-line tool. It gives the exact output of the pinned version. Refer to [Toolchains](toolchains.md).

## Version rules

1. A project can pin a Typst version in its manifest, for example `typst = "0.14"`.
2. If the project has no pin, Presswork uses the embedded engine for all functions.
3. If the pin accepts the embedded version, Presswork uses the embedded engine for all functions.
4. If the pin does not accept the embedded version, Presswork uses the managed toolchain for the preview and the exports. Thus you proof the same output that goes to print. The preview is slower. The editor functions that need the document model, such as completion, tooltips, the outline, and jumps from the pages to the source, are off, and a note tells why.
5. If no installed version agrees with the pin, Presswork shows no preview. The Toolchains panel shows the versions that you can install.

The preview and the exports of a project always use the same Typst version. Refer to D100 in [Decisions](decisions.md).

The managed toolchain reads the text of the main file from the standard input, so the preview also shows changes that are not saved. It reads the other project files from the project folder. It writes one PNG image for each page, at 144 pixels for each inch, which is the scale of the embedded preview. Its diagnostics give a line and a column, so the editor marks the word at that position.

Presswork stops the managed toolchain in these conditions:

- The program runs for more than two minutes. The preview then shows a problem.
- You open another project, or the project changes its Typst version. The result of the program is then not necessary.

Presswork does not stop the program when you type. One compilation always finishes, and then Presswork compiles the newest text. Thus the preview also updates while you type without a pause.

All engine functions are behind the `Engine` trait. A later release can add worker processes that link other Typst versions. Then all functions are available for each version.

## The Typst world

The Typst compiler reads all files, packages, and fonts through a `World`. The Presswork world looks for a file in this sequence:

1. **Overlays.** The open documents from the editor and from Loro.
2. **Project files.** The files in the project folder.
3. **Packages.** The packages in the package cache. The toolchain manager downloads the packages that are not in the cache.

The world gives fonts from these sources: project fonts, team fonts, system fonts, and the fonts that Typst supplies.

The world gives access only to the files in the project root and to packages. Thus a document cannot read other files on the computer.

The caller supplies the clock of the world. Tests use a fixed date. An export can use a fixed date from the manifest, so that the output is reproducible.

## Compilation

- The engine compiles on a dedicated thread.
- The Typst compiler cannot stop a compilation that is in progress. Thus the engine lets each compilation finish.
- If the sources change during a compilation, the engine does one more compilation with the new sources.
- Typst keeps the results of each step in a cache (comemo). After each compilation, the engine removes old cache entries, so that memory use stays low.
- The result of a compilation contains the document, the diagnostics, and the duration.

## Diagnostics

The engine changes each Typst diagnostic into a Presswork diagnostic with these parts:

- The severity: error or warning.
- The file and the text range, in lines and columns.
- The message and the hints.
- The trace: the sequence of calls that caused the diagnostic.

The editor underlines the range. The diagnostics panel shows a list of all diagnostics. A click on a diagnostic moves the cursor to its range.

In milestone 2, a diagnostic has the file of its source and the byte range in that file. The editor marks the diagnostics of the file that it shows (D204). The command-line tool shows the file, the line, and the column of each diagnostic.

## Page images

- The engine renders pages to images with `typst-render`. The scale is the zoom level multiplied by the scale factor of the screen.
- Each page frame has a hash. The engine renders a page again only if its hash changes.
- The preview asks for the visible pages first.
- At high zoom levels, the engine renders tiles of a page, not the full page.
- While the guides of the preview are on, the engine renders the bleed around each page and tells the preview the size of the bleed (D181).

In milestone 1, the engine has these limits:

- The world gives only the fonts that Typst supplies: Libertinus Serif, New Computer Modern, and DejaVu Sans Mono. Thus the output is the same on all computers.
- The world reads the files of the open project folder. It does not read packages yet.
- Exports are not available yet. The preview of a project with a different pin uses the managed toolchain.
- The engine renders all pages after each compilation, at 2 pixels for each point.

In milestone 2, the engine does these things:

- The world reads packages from the package folders of Typst (D203): first the data folder, for example `%APPDATA%\typst\packages` on Windows, then the cache folder, for example `%LOCALAPPDATA%\typst\packages`. The toolchain manager does not download packages yet. Thus a package must be in one of these folders before Presswork can use it, for example after a compilation with the Typst command-line tool.
- The compilation makes no page images. The preview draws only the pages near its visible part, on a separate thread, at 2 pixels for each point (D202).
- The preview keeps each page image by a hash of the page and of the bleed setting. After a change, it draws again only the pages with a different hash. It keeps up to 32 images.
- The QA report, the baseline grid, and the preflight checks run on a separate thread after each compilation. The results of an older compilation are not used.
- The ignored test `time_a_large_book` times a large book. Set `PRESSWORK_BENCH_PROJECT` to its folder.

## IDE functions

The engine uses `typst-ide` for these functions:

| Function | Result |
| --- | --- |
| Completion | A list of completions at the cursor. |
| Tooltip | Information about the item below the mouse pointer. |
| Definition | The location of the definition of a function, a variable, or a label. |
| Jump from pages | The source location of a point that you click in the preview. |
| Jump from source | The position of the cursor in the pages. |
| Outline | The headings of the document with their numbers and pages, for the outline panel. |
| Labels | All labels with the kind, the text, and the page of their elements, for the completion of references. |

These functions use the result of the last compilation. Each compilation keeps a copy of the world and of the document. Thus the functions do not wait for a compilation that runs (D169).

In milestone 2, all these functions are available. Tooltips and completions use a copy of the world with the text of the editor, because the text can be newer than the compilation. They run on a separate thread, because `typst-ide` can compile the document again to find a value.

The outline and the labels come from the introspector of the document. Typst keeps the number of each heading in the heading element, so the outline needs no other compilation. The page of an element is its printed page number if the page has a numbering pattern. Otherwise, it is the position of the page in the document (D174, D175).

The jump from the source has its own search for the glyph at the cursor. The search of `typst-ide` gives only the first glyph of a text node, and in Typst 0.15 a text node contains a full line of text with its spaces. If a paragraph continues on the next page, the search gives only the page of the cursor.

## Export

Each project has export profiles in its manifest. A profile has these parts:

- A name, for example `print` or `screen`.
- A format: PDF, PNG, SVG, or HTML.
- For PDF, one or more PDF standards, for example PDF/A-2b or PDF/UA-1.
- Inputs: the values that the document reads from `sys.inputs`.
- An output path.

HTML export is experimental in Typst. Presswork shows it only if you turn on experimental functions.

In milestone 2, a profile has a name, a format (PDF, PNG, or SVG), PDF standards, an output path, the resolution of PNG files, and the lowest resolution of images. Inputs and HTML export come later. The keys of the manifest are in [Interface](interface.md).

An export runs on its own thread. It compiles the text of the editor without page images, does the preflight checks, and writes the files. `typst-pdf` makes the PDF files, `typst-render` the PNG files, and `typst-svg` the SVG files. A PDF/A file must have a creation date, so the export gives the current time in UTC (D182). Then the export writes a build report next to the files (D184).

## Preflight

Preflight finds problems before an export. The engine examines the compiled document for these problems:

- Images with a resolution below the limit of the profile, for example 300 ppi for print.
- Images without alternative text.
- Fonts that are not available.
- Colors in a color space that the profile does not allow, for example RGB colors in a CMYK print profile.
- Pages with a size that is different from the profile.

In milestone 2, preflight finds all these problems, with the rules of the export profile (D184):

- The resolution of a raster image is its number of pixels for each inch of its size on the page. Scales and other transformations of the image count. If the resolutions across and down are different, preflight uses the lower one. Vector images have no resolution.
- A font family is not available when Typst gives the warning "unknown font family". Preflight shows one problem for each family. The text then uses a different font.
- A glyph is missing when no font has the character. Typst then shows an empty box (glyph 0). Preflight shows one problem for each page, with the first ten characters.
- The page size is the trim size of the page, without the bleed. It must agree with `page-size` of the profile to 0.1 mm.
- Preflight examines the fill and the stroke of text and shapes, and the content of tilings. With `colors = "cmyk"`, the profile accepts CMYK, gray (`luma`), and spot colors. With `colors = "gray"`, it accepts only gray colors. Preflight shows one problem for each color, at its first use. A gradient must have an accepted color space, because Typst converts its stops into that space. Preflight does not examine the colors in images yet.

Each problem has its page, if it has one, and if possible its source. Problems do not stop an export.

A problem on a page also has its area: the image, the shape or the text with the color, or the first missing glyph. After each compilation with pages, the preview puts a box in the error color around the preflight problems of all export profiles of the project (D191). The tooltip of a problem of a profile, for example a color, starts with the name of the profile. A problem of the document, for example an image without alternative text, shows one time.

After an export, veraPDF can validate PDF/A and PDF/UA files. Refer to [Toolchains](toolchains.md).

Checklists can use preflight results as automatic items. Refer to [Domain model](domain-model.md).
