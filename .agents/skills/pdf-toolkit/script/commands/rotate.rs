//! Subcomando `rotate`: gira paginas de um PDF em multiplos de 90 graus, via qpdf.

use std::path::PathBuf;

use crate::common::{error, log, ok, require_file, require_tool, run, usage_err};

pub fn run_cmd(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut degrees: Option<String> = None;
    let mut pages: String = "1-z".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).cloned();
            }
            "-d" | "--degrees" => {
                i += 1;
                degrees = args.get(i).cloned();
            }
            "-p" | "--pages" => {
                i += 1;
                match args.get(i) {
                    Some(v) => pages = v.clone(),
                    None => return usage_err("rotate", "--pages exige um intervalo."),
                }
            }
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            other if input.is_none() => input = Some(other.to_string()),
            other => return usage_err("rotate", &format!("Argumento inesperado: {other}")),
        }
        i += 1;
    }

    let input = match input {
        Some(p) => p,
        None => return usage_err("rotate", "Informe o arquivo PDF de entrada."),
    };
    let degrees = match degrees {
        Some(d) => d,
        None => return usage_err("rotate", "Informe --degrees (90, 180, 270, -90, ...)."),
    };
    let output = match output {
        Some(o) => o,
        None => return usage_err("rotate", "Informe o arquivo de saida com -o/--output."),
    };

    // qpdf exige sinal explicito para rotacao relativa (+90/-90/180).
    let degrees = if degrees.starts_with('+') || degrees.starts_with('-') {
        degrees
    } else {
        format!("+{degrees}")
    };

    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("qpdf") {
        return 1;
    }

    log(&format!("Girando paginas [{pages}] em {degrees} graus"));
    let qargs = vec![
        input.clone(),
        output.clone(),
        format!("--rotate={degrees}:{pages}"),
    ];
    let (success, _stdout, stderr) = run("qpdf", &qargs);
    if success {
        ok(&format!("Gerado: {output}"));
        0
    } else {
        error(&format!("qpdf falhou: {}", stderr.trim()));
        1
    }
}

pub fn print_help() {
    println!(
        "Uso: pdf_toolkit rotate arquivo.pdf --degrees 90 -o saida.pdf [opcoes]\n\n\
         Gira paginas do PDF em multiplos de 90 graus.\n\n\
         Opcoes:\n\
         \x20 -d, --degrees N     90, 180, 270, -90, ... (obrigatorio)\n\
         \x20 -p, --pages SPEC    Paginas a girar (padrao: todas — \"1-z\")\n\
         \x20 -o, --output PATH   Arquivo de saida (obrigatorio)\n\
         \x20 -h, --help          Exibe esta ajuda"
    );
}
