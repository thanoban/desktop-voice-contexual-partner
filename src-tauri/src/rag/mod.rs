use anyhow::{anyhow, Result};
use std::path::Path;

// ── Text extraction ──────────────────────────────────────────────────────────

/// Extracts plain text from TXT, MD, PDF, or DOCX files.
pub fn extract_text(path: &Path) -> Result<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "txt" | "md" | "markdown" | "rst" => extract_plain(path),
        "pdf" => extract_pdf(path),
        "docx" => extract_docx(path),
        other => Err(anyhow!(
            "Unsupported file type '.{}'. Supported: txt, md, pdf, docx",
            other
        )),
    }
}

fn extract_plain(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| anyhow!("Cannot read file: {}", e))
}

fn extract_pdf(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|e| anyhow!("Cannot read PDF: {}", e))?;
    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| anyhow!("PDF extraction failed: {}", e))
}

fn extract_docx(path: &Path) -> Result<String> {
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|e| anyhow!("Cannot open DOCX: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| anyhow!("Invalid DOCX archive: {}", e))?;

    let mut xml_content = String::new();
    {
        let mut doc_xml = archive
            .by_name("word/document.xml")
            .map_err(|_| anyhow!("word/document.xml not found — is this a valid .docx file?"))?;
        doc_xml
            .read_to_string(&mut xml_content)
            .map_err(|e| anyhow!("Failed to read document.xml: {}", e))?;
    }

    Ok(xml_to_text(&xml_content))
}

fn xml_to_text(xml: &str) -> String {
    // Insert line breaks at paragraph and run boundaries before stripping tags
    let spaced = xml
        .replace("<w:p ", "\n")
        .replace("<w:p>", "\n")
        .replace("</w:p>", "\n")
        .replace("<w:br/>", " ")
        .replace("<w:tab/>", "\t");

    // Strip all remaining XML tags
    let mut text = String::with_capacity(spaced.len());
    let mut in_tag = false;
    for c in spaced.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }

    // Normalise whitespace: collapse runs of blanks, trim lines
    text.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Chunking ─────────────────────────────────────────────────────────────────

/// Splits text into overlapping character-level chunks.
/// `chunk_size` — target chunk size in characters (~500 ≈ 80-100 words).
/// `overlap`    — characters of overlap between adjacent chunks (~100).
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }
    if text.len() <= chunk_size {
        return vec![text.to_string()];
    }

    // Work on char boundaries to avoid splitting multi-byte sequences
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < total {
        let end = (start + chunk_size).min(total);

        // Try to break at a sentence or word boundary
        let break_at = if end < total {
            // Look back up to 60 chars for '. ', '\n', or ' '
            let search_start = end.saturating_sub(60);
            let window = &chars[search_start..end];
            let break_pos = window
                .windows(2)
                .rposition(|w| w[0] == '.' || w[0] == '\n')
                .map(|i| search_start + i + 1)
                .or_else(|| {
                    window
                        .iter()
                        .rposition(|&c| c == ' ')
                        .map(|i| search_start + i + 1)
                })
                .unwrap_or(end);
            break_pos
        } else {
            end
        };

        let chunk: String = chars[start..break_at].iter().collect();
        let trimmed = chunk.trim().to_string();
        if !trimmed.is_empty() {
            chunks.push(trimmed);
        }

        if break_at >= total {
            break;
        }
        start = break_at.saturating_sub(overlap);
    }

    chunks
}
