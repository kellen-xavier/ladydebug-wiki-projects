//! Subcomando `extract-images`: extrai as imagens embutidas no PDF, via pdfimages.

use std::fs;
use std::path::PathBuf;

use crate::common::{error, log, ok, require_file, require_tool, run, stem_of, usage_err};

pub fn run_cmd(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output_dir: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output-dir" => {
                i += 1;
                output_dir = args.get(i).cloned();
            }
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            other if input.is_none() => input = Some(other.to_string()),
            other => return usage_err("extract-images", &format!("Argumento inesperado: {other}")),
        }
        i += 1;
    }

    let input = match input {
        Some(p) => p,
        None => return usage_err("extract-images", "Informe o arquivo PDF de entrada."),
    };
    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("pdfimages") {
        return 1;
    }

    let out_dir = PathBuf::from(
        output_dir.unwrap_or_else(|| format!("{}_imagens", stem_of(&input_path))),
    );
    if let Err(e) = fs::create_dir_all(&out_dir) {
        error(&format!("Nao foi possivel criar {}: {e}", out_dir.display()));
        return 1;
    }
    let prefix = out_dir.join(stem_of(&input_path));

    log(&format!("Extraindo imagens de {input}"));
    let pargs = vec!["-j".to_string(), input.clone(), prefix.to_string_lossy().into_owned()];
    let (success, _stdout, stderr) = run("pdfimages", &pargs);
    if !success {
        error(&format!("pdfimages falhou: {}", stderr.trim()));
        return 1;
    }

    let count = fs::read_dir(&out_dir).map(|it| it.count()).unwrap_or(0);
    ok(&format!("{count} imagem(ns) extraida(s) em {}", out_dir.display()));
    0
}

pub fn print_help() {
    println!(
        "Uso: pdf_toolkit extract-images arquivo.pdf [opcoes]\n\n\
         Extrai todas as imagens embutidas no PDF via pdfimages (poppler-utils).\n\n\
         Opcoes:\n\
         \x20 -o, --output-dir DIR  Diretorio de saida (padrao: <nome>_imagens/)\n\
         \x20 -h, --help            Exibe esta ajuda"
    );
}
