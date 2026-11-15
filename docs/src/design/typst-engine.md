# Typst engine

This chapter tells how Presswork compiles, renders, and exports Typst documents.

## Two engines

Presswork uses Typst in two ways:

| Engine | Use | Typst version |
| --- | --- | --- |
| Embedded engine | Live preview, diagnostics, completion, tooltips, and jumps between source and pages | The version in this Presswork release, now 0.15 |
| Managed toolchain | Exports of projects that pin a different Typst version | Each version that the toolchain manager installs |

The embedded engine links the Typst crates into Presswork. It gives access to the document model. Thus the interface can connect each element on a page to its source.

The managed toolchain runs the official Typst command-line tool. It gives the exact output of the pinned version. Refer to [Toolchains](toolchains.md).

## Version rules

1. A project can pin a Typst version in its manifest, for example `typst = "0.14"`.
2. If the project has no pin, Presswork uses the embedded engine for all work.
3. If the pin accepts the embedded version, Presswork uses the embedded engine for all work.
4. If the pin does not accept the embedded version, Presswork uses the managed toolchain for exports. The preview shows a message with the two versions.
5. In this condition, the user can also select a pinned preview. The pinned preview uses the managed toolchain. It is slower, but it shows the exact output of the pinned version.

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

These functions use the result of the last compilation.

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
