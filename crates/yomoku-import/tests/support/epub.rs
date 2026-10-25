//! Shared readable EPUB archive fixture for storage and real-analyzer tests.

use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, ZipWriter};

pub fn book(title: bool, second: &str) -> Vec<u8> {
    let mut package = include_str!("../fixtures/epub/book.opf").to_owned();
    if !title {
        package = package.replace("<dc:title> 日本語の本 &amp; EPUB </dc:title>", "");
    }
    let second =
        format!(r#"<html xmlns="http://www.w3.org/1999/xhtml"><body>{second}</body></html>"#);
    let files = [
        ("mimetype", b"application/epub+zip".as_slice()),
        (
            "META-INF/container.xml",
            include_bytes!("../fixtures/epub/container.xml").as_slice(),
        ),
        ("OPS/book.opf", package.as_bytes()),
        (
            "OPS/text/z-first.xhtml",
            include_bytes!("../fixtures/epub/first.xhtml").as_slice(),
        ),
        ("OPS/text/a-second.xhtml", second.as_bytes()),
        ("OPS/images/cover.png", b"fixture cover bytes".as_slice()),
        ("OPS/images/挿絵.png", b"fixture image bytes".as_slice()),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in files {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
