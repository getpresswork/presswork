# Production

This chapter tells how Presswork makes the final output, and how it controls assets, clearances, and identifiers.

## Export profiles

Each export uses an export profile. Refer to [Typst engine](typst-engine.md) for export profiles and preflight.

## EPUB

Presswork makes EPUB 3 files from the HTML output of Typst:

1. Typst exports each chapter as HTML.
2. Presswork adds the styles, the images, the navigation, and the package metadata.
3. Presswork adds the accessibility metadata, for example the access modes and the hazards.
4. EPUBCheck validates the file. The toolchain manager installs EPUBCheck. It uses the same Java runtime as veraPDF.

The HTML export of Typst is experimental. Thus EPUB export is also experimental until the HTML export of Typst is stable.

## Covers

The cover tool makes print covers and ebook covers:

- It calculates the spine width from the page count and the paper thickness.
- It makes a Typst cover template with the back cover, the spine, the front cover, and the bleed.
- It makes an EAN-13 barcode from the ISBN, with an optional price code.
- It exports a print cover PDF and an ebook cover image.

## Identifiers and metadata

- The ISBN pool keeps the ISBNs of the publisher. Presswork assigns an ISBN to each format of a publication. It checks each check digit.
- Each publication has metadata: the contributors and their roles, the subjects in Thema and BISAC, the languages, the rights territories, and the prices.
- Presswork exports the metadata as ONIX 3.0 for distributors.
- For academic publications, Presswork makes Crossref deposit files and records the DOIs.

## JATS and ORCID

- For journals, Presswork exports articles as JATS XML for repositories and indexing services.
- Contributors can have ORCID identifiers. Presswork adds them to the metadata, to the JATS files, and to the Crossref deposits.

## Print plans

- A print plan divides the page count into signatures, for example of 16 or 32 pages. It shows the blank pages that are necessary at the end of the book.
- Printer price lists give cost estimates for each format, paper, binding, and quantity.
- Cost estimates go into the budget as planned costs.
- The copyfitting assistant helps to reach the target page count. Refer to [Typesetting tools](typesetting-tools.md).

## Print production

- Typst documents can use CMYK colors. Export profiles can contain ICC profiles and an output intent.
- Presswork can add crop marks, bleed marks, and color bars.
- Presswork can impose pages, for example as a saddle-stitched booklet or as signatures for a bound book. Presswork does the imposition with a Typst document that places the pages of the exported PDF file.
- Presswork can make PDF/X files for printers.

Open question: the method for PDF/X output. Typst does not export PDF/X at this time. The two options are a future Typst function or a conversion tool in the toolchain manager. The export profile hides the method. Thus the method can change without an effect on projects.

## Asset library

The asset library keeps images and graphics:

- Each asset has versions and renditions, for example a thumbnail and a CMYK version.
- Each asset has a caption, alternative text, a credit line, and its rights data.
- Presswork checks the resolution of each asset at each location where a document uses it.
- Each asset shows the pages that use it. Presswork finds these pages from the compiled documents.
- If you drag an asset into the editor, Presswork inserts an `#image` call with the caption and the credit.

## Clearances

A clearance records the agreement of a rights holder to the use of a text or an image. It contains:

- The rights holder and the contact data.
- The status: necessary, requested, granted, or refused.
- The terms: the formats, the territories, the print run, and the fee.
- The expiry date.
- Links to the assets, the quotations, and the publications that use it.

Presswork shows a warning if a publication uses an asset or a long quotation without a granted clearance. A checklist item can make this check necessary before print. Presswork also shows the clearances that expire soon.
