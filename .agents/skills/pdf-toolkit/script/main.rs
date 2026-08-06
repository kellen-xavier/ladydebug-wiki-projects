//! pdf_toolkit
//!
//! Kit de operacoes com PDF (mesclar, dividir, girar, extrair texto/imagens,
//! metadados, comprimir, criptografar/descriptografar, marca d'agua).
//! Portavel: Linux, macOS e Windows. Sem dependencias externas (apenas a std).
//!
//! Delega o trabalho binario a ferramentas maduras e ja testadas (mesmo
//! padrao de `docx_to_pdf.rs` e `juntar_pdfs.rb`): qpdf, poppler-utils
//! (pdftotext/pdfimages/pdfinfo) e ghostscript. O programa apenas orquestra
//! esses processos, valida entradas e reporta resultado.
//!
//! Organizacao: cada subcomando vive em seu proprio modulo, curto e objetivo,
//! em `commands/` (mesmo espirito dos scripts individuais do skill de
//! referencia). `common.rs` reune apenas o que e compartilhado entre eles.
//!
//! Uso:
//!   pdf_toolkit merge a.pdf b.pdf c.pdf -o merged.pdf
//!   pdf_toolkit split relatorio.pdf
//!   pdf_toolkit split relatorio.pdf --ranges "1-5,6-10"
//!   pdf_toolkit rotate scan.pdf --degrees 90 -o scan_girado.pdf
//!   pdf_toolkit extract-text contrato.pdf -o contrato.txt
//!   pdf_toolkit extract-images laudo.pdf --output-dir laudo_imagens
//!   pdf_toolkit metadata documento.pdf
//!   pdf_toolkit compress grande.pdf -o grande_comprimido.pdf
//!   pdf_toolkit encrypt documento.pdf --user-password 123 -o documento_protegido.pdf
//!   pdf_toolkit decrypt documento_protegido.pdf --password 123 -o documento.pdf
//!   pdf_toolkit watermark contrato.pdf --stamp confidencial.pdf -o contrato_marcado.pdf
//!   pdf_toolkit --help
//!
//! Requisitos: qpdf, poppler-utils (pdftotext/pdfimages/pdfinfo) e ghostscript.

mod commands;
mod common;

use std::env;

use commands::{compress, decrypt, encrypt, extract_images, extract_text, merge, metadata, rotate, split, watermark};
use common::error;

fn print_help() {
    println!(
        "Uso: pdf_toolkit <comando> [opcoes]\n\n\
         Comandos:\n\
         \x20 merge           Mescla varios PDFs em um so\n\
         \x20 split           Divide um PDF em varios arquivos\n\
         \x20 rotate          Gira paginas de um PDF\n\
         \x20 extract-text    Extrai o texto do PDF\n\
         \x20 extract-images  Extrai as imagens embutidas no PDF\n\
         \x20 metadata        Exibe metadados e informacoes do PDF\n\
         \x20 compress        Comprime um PDF\n\
         \x20 encrypt         Protege um PDF com senha\n\
         \x20 decrypt         Remove a senha de um PDF\n\
         \x20 watermark       Aplica marca d'agua em um PDF\n\n\
         Use 'pdf_toolkit <comando> --help' para detalhes de cada comando."
    );
}

fn main() {
    let argv: Vec<String> = env::args().collect();
    let args = &argv[1..];

    let Some(subcommand) = args.first() else {
        print_help();
        std::process::exit(1);
    };

    let rest = &args[1..];
    let code = match subcommand.as_str() {
        "merge" => merge::run_cmd(rest),
        "split" => split::run_cmd(rest),
        "rotate" => rotate::run_cmd(rest),
        "extract-text" => extract_text::run_cmd(rest),
        "extract-images" => extract_images::run_cmd(rest),
        "metadata" => metadata::run_cmd(rest),
        "compress" => compress::run_cmd(rest),
        "encrypt" => encrypt::run_cmd(rest),
        "decrypt" => decrypt::run_cmd(rest),
        "watermark" => watermark::run_cmd(rest),
        "-h" | "--help" => {
            print_help();
            0
        }
        other => {
            error(&format!("Comando desconhecido: {other}"));
            print_help();
            2
        }
    };
    std::process::exit(code);
}
