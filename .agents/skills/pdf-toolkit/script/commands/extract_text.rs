//! Subcomando `extract-text`: extrai o texto do PDF, via pdftotext.

use std::path::PathBuf;

use crate::common::{error, log, ok, require_file, require_tool, run, stem_of, usage_err};

pub fn run_cmd(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut layout = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).cloned();
            }
            "--layout" => layout = true,
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            other if input.is_none() => input = Some(other.to_string()),
            other => return usage_err("extract-text", &format!("Argumento inesperado: {other}")),
        }
        i += 1;
    }

    let input = match input {
        Some(p) => p,
        None => return usage_err("extract-text", "Informe o arquivo PDF de entrada."),
    };
    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("pdftotext") {
        return 1;
    }

    let output = output.unwrap_or_else(|| format!("{}.txt", stem_of(&input_path)));

    let mut pargs: Vec<String> = Vec::new();
    if layout {
        pargs.push("-layout".into());
    }
    pargs.push(input.clone());
    pargs.push(output.clone());

    log(&format!("Extraindo texto de {input}"));
    let (success, _stdout, stderr) = run("pdftotext", &pargs);
    if success {
        ok(&format!("Gerado: {output}"));
        0
    } else {
        error(&format!("pdftotext falhou: {}", stderr.trim()));
        1
    }
}

pub fn print_help() {
    println!(
        "Uso: pdf_toolkit extract-text arquivo.pdf [opcoes]\n\n\
         Extrai o texto do PDF via pdftotext (poppler-utils).\n\n\
         Opcoes:\n\
         \x20 -o, --output PATH   Arquivo .txt de saida (padrao: <nome>.txt)\n\
         \x20     --layout        Preserva o layout original (colunas, espacamento)\n\
         \x20 -h, --help          Exibe esta ajuda"
    );
}
