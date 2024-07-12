# Rust XLSB writer

## About project

This project is designed to implement reading and, first of all, writing MS XLSB (Excel Binary WorkBook) files.

The program is based on the [`MS-XLSB`](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-xlsb/acc8aa92-1f02-4167-99f5-84f9f676b95a) standard revision `2024-05-21`. Older revisions of this standard are not supported yet.

The Rust language is mainly used for development.

The goal of the project is to implement data recording “cell by cell” and data recording in batches in the Apache Arrow format with support for styles and formats in columns and individual cells, minimal load on RAM and ensuring high quality of recorded data (maintaining precision in floating point numbers.

The ability to write files in this format also requires the ability to read them back for testing, so the ability to read files “cell by cell” and in batches into the Apache Arrow format will also be implemented.

## Why XLSB

The XLSB format was designed to optimize work with Excel workbooks, and has significant advantages over the XLSX format, which is currently widely used by the huge community of data engineers and analysts for automated reporting.

These benefits include, but are not limited to:

- A significant reduction in the size of the final file on disk (in most cases - up to 10 times compared to XLSX)
- Optimization of read and write operations of such files due to direct binary compatibility of the stored data
- Providing multi-threaded read and write capabilities through the use of a special binary index.

XLSB files can be opened in almost any dedicated office application, just like XLSX, be it Microsoft Excel, OpenOffice Calc, LibreOffice Calc, Google Sheets, or others.

Even packaging an XLSX file using archivers usually does not allow creating final files of such a compact size compared to an XLSB file of the same content.

Compared to CSV files, XLSB files can be well-typed internally and well-formatted. However, uncompressed XLSB files tend to take up less disk space than uncompressed CSV files - especially for large files that contain many string cells with the same content.

Also, all numeric cells in XLSB occupy exactly 8 bytes, according to the IEEE 754 standard, while in XLSX and CSV files, numbers occupy exactly as many bytes as they contain decimal places (plus one byte for a period), which leads, firstly, to waste places to store them when there are more than eight digits, as well as the need to convert them from string format to numeric format and vice versa when reading and writing.

Date, when stored in XLSB format, occupy exactly four bytes, while maintaining day-precision, and timestamp take up 8 bytes, while maintaining millisecond precision. At the same time, CSV files are forced to store dates and timestamps in full string representation (usually in ISO 8601 format, for example: `"2022-01-01"` and `"2022-01-01T12:00:00.000"`), which maintaining the same level of accuracy, takes 10 and 24 bytes, respectively (without quotes).

## Project milestones

The development of this project has several key milestones:

- Full support for basic low-level read and write
- Basic support for cell styling, support for merging cells
- Support for recording in batches in Apache Arrow format
- Support for multi-threading when recording in batches
- Full support for cell styling
- Ability to insert images onto a sheet
- Support for cells with formulas

As the project progresses, these milestones may be revised and new ones may be added.

The condition for the release of version 1.0.0-alpha will be the recognition of this project by the open source community and the inclusion of participants in the OpenOffice or LibreOffice projects in the development, as well as integrations into other widely used software libraries and products.

## Contributing

You are more than welcome to contribute to this project, or sponsor it at [Boosty.to](https://boosty.to/sael.dev/donate).

The project is in active development, but at the moment there is only one developer working on it, Peter ([SaelKimberly](https://github.com/SaelKimberly)). Your Issues and Pull Requests are welcome!

## License

This project is licensed under Apache License, Version 2.0 ([`LICENSE`](https://github.com/SaelKimberly/rust-xlsb-writer/blob/main/LICENSE) or [`https://www.apache.org/licenses/LICENSE-2.0`](https://www.apache.org/licenses/LICENSE-2.0))
