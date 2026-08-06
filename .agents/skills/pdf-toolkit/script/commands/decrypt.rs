//! Subcomando `decrypt`: remove a senha/criptografia de um PDF, via qpdf.

use std::path::PathBuf;

use crate::common::{error, log, ok, require_file, require_tool, run, usage_err};

pub fn run_cmd(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut password = String::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).cloned();
            }
            "-p" | "--password" => {
                i += 1;
                match args.get(i) {
                    Some(v) => password = v.clone(),
                    None => return usage_err("decrypt", "--password exige um valor."),
                }
            }
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            other if input.is_none() => input = Some(other.to_string()),
            other => return usage_err("decrypt", &format!("Argumento inesperado: {other}")),
        }
        i += 1;
    }

    let input = match input {
        Some(p) => p,
        None => return usage_err("decrypt", "Informe o arquivo PDF de entrada."),
    };
    let output = match output {
        Some(o) => o,
        None => return usage_err("decrypt", "Informe o arquivo de saida com -o/--output."),
    };

    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("qpdf") {
        return 1;
    }

    log("Removendo senha do PDF");
    let qargs = vec![
        format!("--password={password}"),
        "--decrypt".to_string(),
        input.clone(),
        output.clone(),
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
        "Uso: pdf_toolkit decrypt arquivo.pdf --password SENHA -o saida.pdf\n\n\
         Remove a senha/criptografia de um PDF protegido."
    );
}
