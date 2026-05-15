//! Hand-written PDF writer. No PDF crate dependency.
//! Mirrors `python-reference/vigb_max2pdf.py:write_pdf` (lines 992-1057).

use std::io::Write;
use std::path::Path;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::decoder::{Page, Preview};
use crate::error::Result;

/// Configuration for PDF generation.
#[derive(Debug, Clone)]
pub struct PdfOptions {
    /// Embed each page's preview thumbnail as a second page after the main page.
    /// When false, only main-image pages are written.
    pub include_previews: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            include_previews: true,
        }
    }
}

/// Write `pages` to `path` as a single PDF. Convenience wrapper for
/// [`write_pdf_bytes`].
pub fn write_pdf(pages: &[Page], path: &Path) -> Result<()> {
    let bytes = write_pdf_bytes(pages, &PdfOptions::default());
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Build a PDF as a `Vec<u8>`.
pub fn write_pdf_bytes(pages: &[Page], options: &PdfOptions) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = vec![Vec::new()]; // 1-based indexing
    let palette = [0xFFu8, 0x00]; // /Indexed [0=white, 1=black]
    let mut page_ids: Vec<usize> = Vec::new();

    for p in pages {
        page_ids.push(emit_page_for_bitmap(
            &p.bitmap,
            p.width,
            p.height,
            p.dpi_x,
            p.dpi_y,
            p.row_bytes,
            &palette,
            &mut objects,
        ));
        if options.include_previews {
            if let Some(prev) = &p.preview {
                page_ids.push(emit_page_for_preview(
                    prev,
                    p.dpi_x,
                    p.dpi_y,
                    &palette,
                    &mut objects,
                ));
            }
        }
    }

    // /Pages object
    let mut pages_obj = Vec::new();
    write!(
        pages_obj,
        "<< /Type /Pages /Count {} /Kids [",
        page_ids.len()
    )
    .unwrap();
    for (i, pid) in page_ids.iter().enumerate() {
        if i > 0 {
            pages_obj.push(b' ');
        }
        write!(pages_obj, "{pid} 0 R").unwrap();
    }
    pages_obj.extend_from_slice(b"] >>");
    let pages_id = emit(pages_obj, &mut objects);

    // Patch each page's /Parent reference
    for &pid in &page_ids {
        let placeholder = b"/Parent 0 0 R";
        let replacement = format!("/Parent {pages_id} 0 R");
        if let Some(start) = find_subslice(&objects[pid], placeholder) {
            objects[pid].splice(start..start + placeholder.len(), replacement.bytes());
        }
    }

    // /Catalog
    let catalog = format!("<< /Type /Catalog /Pages {pages_id} 0 R >>").into_bytes();
    let catalog_id = emit(catalog, &mut objects);

    // Assemble the PDF
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");
    let mut offsets: Vec<usize> = vec![0; objects.len()];
    for i in 1..objects.len() {
        offsets[i] = buf.len();
        writeln!(buf, "{i} 0 obj").unwrap();
        buf.extend_from_slice(&objects[i]);
        buf.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = buf.len();
    write!(buf, "xref\n0 {}\n0000000000 65535 f \n", objects.len()).unwrap();
    for &off in &offsets[1..] {
        writeln!(buf, "{off:010} 00000 n ").unwrap();
    }
    write!(
        buf,
        "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
        objects.len(),
        catalog_id,
        xref_pos
    )
    .unwrap();
    buf
}

fn emit(obj: Vec<u8>, objects: &mut Vec<Vec<u8>>) -> usize {
    objects.push(obj);
    objects.len() - 1
}

#[allow(clippy::too_many_arguments)]
fn emit_page_for_bitmap(
    raw: &[u8],
    width: u32,
    height: u32,
    dpi_x: u32,
    dpi_y: u32,
    row_bytes: u32,
    palette: &[u8; 2],
    objects: &mut Vec<Vec<u8>>,
) -> usize {
    let stored_width = row_bytes * 8;
    let compressed = zlib_compress(raw);

    let mut img_dict = Vec::new();
    let pal_hex: String = palette.iter().map(|b| format!("{b:02X}")).collect();
    write!(
        img_dict,
        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /BitsPerComponent 1 \
         /ColorSpace [/Indexed /DeviceGray 1 <{}>] /Filter /FlateDecode /Length {} >>\nstream\n",
        stored_width,
        height,
        pal_hex,
        compressed.len()
    )
    .unwrap();
    img_dict.extend_from_slice(&compressed);
    img_dict.extend_from_slice(b"\nendstream");
    let img_id = emit(img_dict, objects);

    let page_w = width as f64 * 72.0 / dpi_x as f64;
    let page_h = height as f64 * 72.0 / dpi_y as f64;
    let scale_x = stored_width as f64 * 72.0 / dpi_x as f64;
    let scale_y = page_h;
    let content_str = format!("q\n{scale_x:.4} 0 0 {scale_y:.4} 0 0 cm\n/Im0 Do\nQ\n");
    let mut content = Vec::new();
    write!(content, "<< /Length {} >>\nstream\n", content_str.len()).unwrap();
    content.extend_from_slice(content_str.as_bytes());
    content.extend_from_slice(b"endstream");
    let content_id = emit(content, objects);

    let page_obj = format!(
        "<< /Type /Page /Parent 0 0 R /MediaBox [0 0 {page_w:.4} {page_h:.4}] \
         /Contents {content_id} 0 R \
         /Resources << /XObject << /Im0 {img_id} 0 R >> /ProcSet [/PDF /ImageB] >> >>"
    )
    .into_bytes();
    emit(page_obj, objects)
}

fn emit_page_for_preview(
    prev: &Preview,
    dpi_x: u32,
    dpi_y: u32,
    palette: &[u8; 2],
    objects: &mut Vec<Vec<u8>>,
) -> usize {
    emit_page_for_bitmap(
        &prev.bitmap,
        prev.width,
        prev.height,
        dpi_x,
        dpi_y,
        prev.row_bytes,
        palette,
        objects,
    )
}

fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::DecodeStats;

    fn make_test_page(width: u32, height: u32) -> Page {
        let row_bytes = ((width as usize).div_ceil(8) + 3) & !3;
        let bitmap = vec![0u8; row_bytes * height as usize];
        Page {
            width,
            height,
            dpi_x: 300,
            dpi_y: 300,
            row_bytes: row_bytes as u32,
            bitmap,
            preview: None,
            stats: DecodeStats::default(),
        }
    }

    #[test]
    fn pdf_has_valid_header_and_trailer() {
        let pages = vec![make_test_page(100, 100)];
        let bytes = write_pdf_bytes(&pages, &PdfOptions::default());
        assert!(bytes.starts_with(b"%PDF-1.4\n"));
        assert!(bytes.windows(7).any(|w| w == b"trailer"));
        assert!(bytes.windows(9).any(|w| w == b"startxref"));
        assert!(bytes.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn pdf_xref_is_well_formed() {
        let pages = vec![make_test_page(50, 50)];
        let bytes = write_pdf_bytes(&pages, &PdfOptions::default());
        let xref_pos = bytes.windows(5).position(|w| w == b"xref\n").unwrap();
        let xref_section = &bytes[xref_pos..];
        assert!(xref_section
            .windows(20)
            .any(|w| w == b"0000000000 65535 f \n"));
    }

    #[test]
    fn write_pdf_to_disk_round_trips() {
        let pages = vec![make_test_page(20, 20)];
        let tmp = std::env::temp_dir().join("vigb_decoder_test.pdf");
        write_pdf(&pages, &tmp).expect("write PDF");
        let bytes = std::fs::read(&tmp).expect("read PDF back");
        assert!(bytes.starts_with(b"%PDF-1.4\n"));
        std::fs::remove_file(&tmp).ok();
    }
}
