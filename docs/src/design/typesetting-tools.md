# Typesetting tools

This chapter tells which tools Presswork gives to typesetters and designers. Each tool writes or examines normal Typst code. Thus a project stays a normal Typst project.

## Style panel

The style panel has forms for the page size, the margins, the columns, the running heads, the page numbers, the headings, the paragraphs, and the lists.

- The panel writes `set` rules and `show` rules into a style file of the project.
- The code stays visible. If you edit the code, the panel reads it again.
- If the panel cannot read a rule, it shows the rule as code.

## Builders

| Builder | Function |
| --- | --- |
| Table builder | A grid editor for Typst tables, with spans, header rows, and alignment. |
| Math builder | An equation editor with a symbol palette and a live preview. It writes Typst math. |
| Chart builder | A chart from a data file, with the chart type, the axes, and the series. It writes the Typst code for a chart package from Typst Universe. |

Each builder writes normal Typst code. You can open the code again in the builder.

## Lift from PDF

The lift function rebuilds content from an area of a PDF page as Typst code. The PDF file can be a library file, an asset, or an imported file.

| Content | Method | Result |
| --- | --- | --- |
| Chart | Presswork reads the vector graphics: the axes, the ticks, the labels, and the marks. It finds the chart type and calculates the data values from the positions and the axis scales. | A data file and a Typst chart that uses it. The chart opens in the chart builder. |
| Chart, as a fallback | If Presswork cannot find the chart type, it copies the vector paths and the text. | Typst code for the shapes and the text, which you can edit. |
| Chart in an image | AI assistance reads the image and proposes the data values. A person checks each value. | A draft data file and a Typst chart. |
| Table | Presswork finds the cells, the rows, the columns, the spans, and the header rows from the text positions. For scanned pages, it uses the OCR result. | A Typst table. It opens in the table builder. |
| Equation | Presswork uses the glyphs and their positions in vector PDF files. For images, it uses AI assistance. | Typst math. It opens in the math builder. |

Presswork shows the original area and the result side by side. The result is a suggestion.

If the PDF file is from a different rights holder, Presswork links the result to its source page and makes a clearance with the status "necessary". Refer to [Production](production.md).

## Brand kits

- A brand kit is a team package with the fonts, the colors, the page designs, and the series designs of the organization.
- The template gallery shows the brand kits and the templates of the team.
- A new project can start from a template in the gallery.

## Typesetting QA report

The typesetting QA report examines the compiled layout. It finds these problems:

- Widows: the last line of a paragraph at the top of a page.
- Orphans: the first line of a paragraph at the bottom of a page.
- Stacked hyphens: more consecutive hyphenated lines than the limit.
- Loose lines: lines with word spaces wider than the limit.
- Short last lines: last lines with fewer characters than the limit.
- Text outside of the margins.

The preview marks each problem on its page. The QA panel lists the problems, and each item links to its source. AI assistance can propose small text edits that correct a break.

In milestone 2, the report finds the first five problems (D185):

- The report finds the paragraphs from the tags of the Typst output. Thus headings, table cells, and other text outside of paragraphs do not count. The lines of a paragraph come from the positions of its text.
- A widow or an orphan is a line alone on a different page or in a different column. A two-line paragraph across a break has both.
- A line is hyphenated when it ends with a soft hyphen that Typst added, or with a hyphen. More than three hyphenated lines in sequence are one problem.
- A line is loose when one of its word spaces is wider than 150 % of the normal width of the space. The last line of a paragraph is never loose.
- A last line is short when it has fewer than five characters, without spaces. Paragraphs with one line do not count.
- The limits are fixed now. Text outside of the margins needs the margins, and Typst does not give them (D181).

The Typeset workspace shows the report of the last compilation. A click on a problem opens the Write workspace and selects the source of the line. The preview then marks the place. The report needs the built-in Typst engine. `presswork-cli check` also lists these problems, but they do not change its exit code.

## Baseline grid check

The baseline grid check finds lines of body text that are not on the baseline grid of the page design. The grid comes from the style file of the project.

## Soft proof

The soft proof shows the printed colors in the preview. Presswork converts the colors with the ICC profile of the export profile. It also marks colors that are outside of the color range of the profile.

## Hyphenation exceptions

- An organization keeps lists of words with their correct hyphenation, for each language.
- The Presswork helper package applies the lists with Typst `show` rules. Thus projects still compile with the Typst command-line tool.

## Copyfitting

- Before typesetting, the copyfitting assistant estimates the page count from the word count and the page design.
- After typesetting, it proposes small changes to reach a target page count, for example in the leading, the tracking, or the space between paragraphs.
- Each proposal is a suggestion that shows the new page count.
- The print plan gives the target page count. Refer to [Production](production.md).

## Version migration

The version migration moves a project to a new Typst version in these steps:

1. Install the new Typst version.
2. Compile the project with the old version and with the new version.
3. Compare the pages of the two results visually.
4. List the deprecation warnings and the changed pages.
5. Make suggestions that correct the known syntax changes automatically.
6. Change the pin in the manifest when the team accepts the result.

## Compile profiler

Typst records the time of each step of a compilation. The compile profiler shows these times as a flame chart. Each bar links to its source location.

## Packages and templates

- A package project has a `typst.toml` manifest.
- Presswork tests a package with example documents and page snapshots.
- Presswork sets the version and publishes the package to the team server, or prepares a submission to Typst Universe.

## Scratchpad

The scratchpad is a Typst document outside of projects, with an instant preview. You can test an idea there and then move the content into a project.
