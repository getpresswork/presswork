# Typst engine

This chapter tells how Presswork compiles, renders, and exports Typst documents.

## Two engines

Presswork uses Typst in two ways:

| Engine | Use | Typst version |
| --- | --- | --- |
| Embedded engine | Live preview, exports, diagnostics, completion, tooltips, and jumps between source and pages | The version in this Presswork release, now 0.15 |
| Managed toolchain | Live preview, exports, and diagnostics of projects that pin a different Typst version | Each version that the toolchain manager installs |

The embedded engine links the Typst crates into Presswork. It gives access to the document model. Thus the interface can connect each element on a page to its source.

The managed toolchain runs the official Typst command-line tool. It gives the exact output of the pinned version. Refer to [Toolchains](toolchains.md).

## Version rules

1. A project can pin a Typst version in its manifest, for example `typst = "0.14"`.
2. If the project has no pin, Presswork uses the embedded engine for all functions.
3. If the pin accepts the embedded version, Presswork uses the embedded engine for all functions.
4. If the pin does not accept the embedded version, Presswork uses the managed toolchain for the preview and the exports. Thus you proof the same output that goes to print. The preview is slower. The editor functions that need the document model, such as completion, tooltips, and jumps from the pages to the source, are off, and a note tells why.
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

## Page images

- The engine renders pages to images with `typst-render`. The scale is the zoom level multiplied by the scale factor of the screen.
- Each page frame has a hash. The engine renders a page again only if its hash changes.
- The preview asks for the visible pages first.
- At high zoom levels, the engine renders tiles of a page, not the full page.

In milestone 1, the engine has these limits:

- The world gives only the fonts that Typst supplies: Libertinus Serif, New Computer Modern, and DejaVu Sans Mono. Thus the output is the same on all computers.
- The world reads the files of the open project folder. It does not read packages yet.
- Exports are not available yet. The preview of a project with a different pin uses the managed toolchain.
- The engine renders all pages after each compilation, at 2 pixels for each point.

## IDE functions

The engine uses `typst-ide` for these functions:

| Function | Result |
| --- | --- |
| Completion | A list of completions at the cursor. |
| Tooltip | Information about the item below the mouse pointer. |
| Definition | The location of the definition of a function, a variable, or a label. |
| Jump from pages | The source location of a point that you click in the preview. |
| Jump from source | The position of the cursor in the pages. |
| Outline | The headings of the document, for the outline panel. |
| Labels | All labels and their positions, for references. |

These functions use the result of the last compilation. Each compilation keeps a copy of the world and of the document. Thus the functions do not wait for a compilation that runs (D169).

The jump from the source has its own search for the glyph at the cursor. The search of `typst-ide` gives only the first glyph of a text node, and in Typst 0.15 a text node contains a full line of text with its spaces. If a paragraph continues on the next page, the search gives only the page of the cursor.

## Export

Each project has export profiles in its manifest. A profile has these parts:

- A name, for example `print` or `screen`.
- A format: PDF, PNG, SVG, or HTML.
- For PDF, one or more PDF standards, for example PDF/A-2b or PDF/UA-1.
- Inputs: the values that the document reads from `sys.inputs`.
- An output path.

HTML export is experimental in Typst. Presswork shows it only if you turn on experimental functions.

## Preflight

Preflight finds problems before an export. The engine examines the compiled document for these problems:

- Images with a resolution below the limit of the profile, for example 300 ppi for print.
- Images without alternative text.
- Fonts that are not available.
- Colors in a color space that the profile does not allow, for example RGB colors in a CMYK print profile.
- Pages with a size that is different from the profile.

After an export, veraPDF can validate PDF/A and PDF/UA files. Refer to [Toolchains](toolchains.md).

Checklists can use preflight results as automatic items. Refer to [Domain model](domain-model.md).
