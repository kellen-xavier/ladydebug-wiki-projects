//! Subcomando `compress`: reduz o tamanho do PDF comprimindo imagens, via ghostscript.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{error, log, ok, require_file, require_tool, run, stem_of, usage_err};

pub fn run_cmd(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut quality = "printer".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).cloned();
            }
            "-q" | "--quality" => {
                i += 1;
                match args.get(i) {
                    Some(v) => quality = v.clone(),
                    None => return usage_err("compress", "--quality exige printer|ebook|screen."),
                }
            }
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            other if input.is_none() => input = Some(other.to_string()),
            other => return usage_err("compress", &format!("Argumento inesperado: {other}")),
        }
        i += 1;
    }

    if !["printer", "ebook", "screen"].contains(&quality.as_str()) {
        return usage_err("compress", "--quality deve ser printer, ebook ou screen.");
    }

    let input = match input {
        Some(p) => p,
        None => return usage_err("compress", "Informe o arquivo PDF de entrada."),
    };
    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("gs") {
        return 1;
    }

    let output = output.unwrap_or_else(|| format!("{}_comprimido.pdf", stem_of(&input_path)));

    let gs_args = vec![
        "-sDEVICE=pdfwrite".to_string(),
        "-dCompatibilityLevel=1.5".to_string(),
        format!("-dPDFSETTINGS=/{quality}"),
        "-dNOPAUSE".to_string(),
        "-dQUIET".to_string(),
        "-dBATCH".to_string(),
        "-dColorImageResolution=150".to_string(),
        "-dGrayImageResolution=150".to_string(),
        "-dMonoImageResolution=300".to_string(),
        format!("-sOutputFile={output}"),
        input.clone(),
    ];

    log(&format!("Comprimindo (qualidade: {quality})"));
    let (success, _stdout, stderr) = run("gs", &gs_args);
    if !success || !Path::new(&output).is_file() {
        error(&format!("ghostscript falhou: {}", stderr.trim()));
        return 1;
    }

    let original_kb = fs::metadata(&input_path).map(|m| m.len()).unwrap_or(0) as f64 / 1024.0;
    let compressed_kb = fs::metadata(&output).map(|m| m.len()).unwrap_or(0) as f64 / 1024.0;
    let savings = if original_kb > 0.0 {
        (1.0 - compressed_kb / original_kb) * 100.0
    } else {
        0.0
    };
    ok(&format!(
        "Gerado: {output} ({original_kb:.1} KB -> {compressed_kb:.1} KB, {savings:.1}% menor)"
    ));
    0
}

pub fn print_help() {
    println!(
        "Uso: pdf_toolkit compress arquivo.pdf [opcoes]\n\n\
         Comprime um PDF via ghostscript, reduzindo a resolucao das imagens.\n\n\
         Opcoes:\n\
         \x20 -o, --output PATH    Arquivo de saida (padrao: <nome>_comprimido.pdf)\n\
         \x20 -q, --quality NIVEL  printer (padrao, alta), ebook (media) ou screen (baixa)\n\
         \x20 -h, --help           Exibe esta ajuda"
    );
}
