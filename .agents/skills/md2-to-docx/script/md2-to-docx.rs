//! md2docx — converte um arquivo Markdown (.md) em .docx preservando
//! 100% do conteudo e da indentacao do arquivo original.
//!
//! A conversao e VERBATIM: cada linha do .md vira um paragrafo em fonte
//! monoespacada, com todos os espacos preservados (xml:space="preserve").
//! Nada da sintaxe Markdown e interpretado — `#`, `-`, cercas de codigo,
//! indentacao de listas e blocos permanecem exatamente como no fonte.
//!
//! Uso:
//!     md2docx <entrada.md> [saida.docx]
//!
//! Sem dependencias pesadas: um .docx e apenas um ZIP com alguns XMLs,
//! e este programa os gera diretamente.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "md2docx — Markdown -> DOCX (verbatim: preserva conteudo e indentacao)\n\n\
             Uso:\n    {} <entrada.md> [saida.docx]\n\n\
             Se a saida for omitida, usa a entrada trocando a extensao para .docx.",
            args.first().map(String::as_str).unwrap_or("md2docx")
        );
        return ExitCode::from(2);
    }

    let input = PathBuf::from(&args[1]);
    let output = match args.get(2) {
        Some(o) => PathBuf::from(o),
        None => input.with_extension("docx"),
    };

    match run(&input, &output) {
        Ok(n) => {
            println!("OK: {} -> {} ({n} linhas)", input.display(), output.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("ERRO: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(input: &Path, output: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(input)
        .map_err(|e| format!("nao consegui ler '{}': {e}", input.display()))?;

    // Remove BOM UTF-8 se presente (nao e conteudo visivel).
    let content = raw.strip_prefix('\u{feff}').unwrap_or(&raw);

    // Quebra em linhas logicas. O '\r' de finais CRLF e removido: e um
    // controle de fim de linha, nao indentacao nem conteudo textual.
    let mut lines: Vec<&str> = content
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();

    // Remove o paragrafo vazio "fantasma" quando o arquivo termina em \n.
    if lines.len() > 1 && lines.last() == Some(&"") {
        lines.pop();
    }

    let document_xml = build_document_xml(&lines);
    write_docx(output, &document_xml)?;
    Ok(lines.len())
}

/// Monta o corpo do word/document.xml, um <w:p> por linha do arquivo.
fn build_document_xml(lines: &[&str]) -> String {
    let mut body = String::with_capacity(lines.len() * 96 + 512);

    body.push_str(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#,
    );

    // Espacamento simples e sem antes/depois -> visual proximo do fonte.
    const PPR: &str =
        r#"<w:pPr><w:spacing w:before="0" w:after="0" w:line="240" w:lineRule="auto"/></w:pPr>"#;
    const RPR: &str = r#"<w:rPr><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas" w:cs="Consolas"/><w:sz w:val="20"/><w:szCs w:val="20"/></w:rPr>"#;

    for &line in lines {
        body.push_str("<w:p>");
        body.push_str(PPR);
        if !line.is_empty() {
            body.push_str("<w:r>");
            body.push_str(RPR);
            body.push_str(r#"<w:t xml:space="preserve">"#);
            xml_escape_into(line, &mut body);
            body.push_str("</w:t></w:r>");
        }
        body.push_str("</w:p>");
    }

    // Secao A4, margens de 2,5cm (1440 twips = 1 polegada; 1417 ~= 2,5cm).
    body.push_str(
        r#"<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1417" w:right="1417" w:bottom="1417" w:left="1417" w:header="708" w:footer="708" w:gutter="0"/></w:sectPr>"#,
    );
    body.push_str("</w:body></w:document>");
    body
}

/// Escapa &, <, > e descarta caracteres invalidos em XML 1.0.
/// Tabs sao preservados como caractere literal (indentacao intacta).
fn xml_escape_into(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\t' => out.push('\t'),
            c if is_valid_xml_char(c) => out.push(c),
            _ => {} // controle invalido em XML: descartado (nao ocorre em .md normal)
        }
    }
}

fn is_valid_xml_char(c: char) -> bool {
    let u = c as u32;
    u == 0x9 || u == 0xA || u == 0xD || (0x20..=0xD7FF).contains(&u)
        || (0xE000..=0xFFFD).contains(&u)
        || (0x10000..=0x10FFFF).contains(&u)
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

/// Empacota o .docx (ZIP com os XMLs obrigatorios).
fn write_docx(output: &Path, document_xml: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(output)
        .map_err(|e| format!("nao consegui criar '{}': {e}", output.display()))?;
    let mut zip = ZipWriter::new(file);
    let opts = FileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", opts)?;
    zip.write_all(CONTENT_TYPES.as_bytes())?;

    zip.start_file("_rels/.rels", opts)?;
    zip.write_all(RELS.as_bytes())?;

    zip.start_file("word/document.xml", opts)?;
    zip.write_all(document_xml.as_bytes())?;

    zip.finish()?;
    Ok(())
}
