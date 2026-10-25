//! The first sheet of an xlsx workbook as rows of text: a zip directory reader, raw deflate
//! (miniz_oxide) and the SpreadsheetML cells (shared strings, inline strings, numbers).

use std::collections::BTreeMap;

/// One sheet row: its sheet row number and the column → text of its nonempty cells.
pub type Row = (usize, BTreeMap<String, String>);

fn u16le(b: &[u8], i: usize) -> usize {
    u16::from_le_bytes([b[i], b[i + 1]]) as usize
}

fn u32le(b: &[u8], i: usize) -> usize {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize
}

/// The member `name` of a zip archive, decompressed.
pub fn member(zip: &[u8], name: &str) -> Result<Vec<u8>, String> {
    let eocd = (0..zip.len().saturating_sub(21)).rev()
        .find(|&i| zip[i..i + 4] == [0x50, 0x4b, 0x05, 0x06])
        .ok_or("not a zip archive")?;
    let (count, mut at) = (u16le(zip, eocd + 10), u32le(zip, eocd + 16));
    for _ in 0..count {
        if zip[at..at + 4] != [0x50, 0x4b, 0x01, 0x02] {
            return Err("corrupt zip directory".into());
        }
        let (method, size) = (u16le(zip, at + 10), u32le(zip, at + 20));
        let (n, x, c) = (u16le(zip, at + 28), u16le(zip, at + 30), u16le(zip, at + 32));
        let local = u32le(zip, at + 42);
        if &zip[at + 46..at + 46 + n] == name.as_bytes() {
            let start = local + 30 + u16le(zip, local + 26) + u16le(zip, local + 28);
            let raw = &zip[start..start + size];
            return match method {
                0 => Ok(raw.to_vec()),
                8 => miniz_oxide::inflate::decompress_to_vec(raw)
                    .map_err(|e| format!("{name}: inflate failed: {e:?}")),
                m => Err(format!("{name}: unsupported compression method {m}")),
            };
        }
        at += 46 + n + x + c;
    }
    Err(format!("{name} not in the workbook"))
}

/// Concatenated text of every <t> below a node (ElementTree `iter('t')`).
fn text(n: roxmltree::Node) -> String {
    n.descendants().filter(|d| d.has_tag_name((NS, "t"))).filter_map(|t| t.text()).collect()
}

const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// Rows of the first sheet in sheet order; a numeric cell drops a trailing ".0".
pub fn sheet(zip: &[u8]) -> Result<Vec<Row>, String> {
    let ss = String::from_utf8(member(zip, "xl/sharedStrings.xml")?).map_err(|e| e.to_string())?;
    let ss = roxmltree::Document::parse(&ss).map_err(|e| e.to_string())?;
    let shared: Vec<String> = ss.root_element().children().filter(|n| n.has_tag_name((NS, "si")))
        .map(text).collect();
    let ws = String::from_utf8(member(zip, "xl/worksheets/sheet1.xml")?).map_err(|e| e.to_string())?;
    let ws = roxmltree::Document::parse(&ws).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for r in ws.descendants().filter(|n| n.has_tag_name((NS, "row"))) {
        let mut cells = BTreeMap::new();
        for c in r.children().filter(|n| n.has_tag_name((NS, "c"))) {
            let at = c.attribute("r").ok_or("cell without reference")?;
            let col: String = at.chars().take_while(|ch| ch.is_ascii_uppercase()).collect();
            let v = c.children().find(|n| n.has_tag_name((NS, "v"))).map(|v| v.text().unwrap_or(""));
            let value = match c.attribute("t") {
                Some("s") => {
                    let i: usize = v.ok_or("shared string without value")?.parse().map_err(|_| "bad index")?;
                    Some(shared[i].clone())
                }
                Some("inlineStr") => Some(text(c)),
                _ => v.map(|v| v.strip_suffix(".0").unwrap_or(v).to_string()),
            };
            if let Some(value) = value {
                cells.insert(col, value);
            }
        }
        rows.push((r.attribute("r").ok_or("row without number")?.parse().map_err(|_| "bad row")?, cells));
    }
    Ok(rows)
}
