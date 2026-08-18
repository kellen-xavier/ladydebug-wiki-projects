//! Subcomando `metadata`: exibe metadados e informacoes do PDF, via pdfinfo.

use std::path::PathBuf;

use crate::common::{error, require_file, require_tool, run, usage_err};

pub fn run_cmd(args: &[String]) -> i32 {
    if matches!(args.first().map(String::as_str), Some("-h") | Some("--help")) {
        print_help();
        return 0;
    }
    let input = match args.first() {
        Some(p) => p.clone(),
        None => return usage_err("metadata", "Informe o arquivo PDF de entrada."),
    };
    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("pdfinfo") {
        return 1;
    }

    let (success, stdout, stderr) = run("pdfinfo", &[input.clone()]);
    if success {
        print!("{stdout}");
        0
    } else {
        error(&format!("pdfinfo falhou: {}", stderr.trim()));
        1
    }
}

pub fn print_help() {
    println!(
        "Uso: pdf_toolkit metadata arquivo.pdf\n\n\
         Exibe metadados e informacoes do PDF via pdfinfo (poppler-utils)."
    );
}
