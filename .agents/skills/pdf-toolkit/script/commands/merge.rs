//! Subcomando `merge`: mescla varios PDFs em um so, via qpdf.

use std::path::Path;

use crate::common::{error, log, ok, require_file, require_tool, run, usage_err};

pub fn run_cmd(args: &[String]) -> i32 {
    let mut inputs: Vec<String> = Vec::new();
    let mut output: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                match args.get(i) {
                    Some(v) => output = Some(v.clone()),
                    None => return usage_err("merge", "--output exige um caminho."),
                }
            }
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            other => inputs.push(other.to_string()),
        }
        i += 1;
    }

    if inputs.len() < 2 {
        return usage_err("merge", "Informe ao menos dois arquivos PDF para mesclar.");
    }
    let output = match output {
        Some(o) => o,
        None => return usage_err("merge", "Informe o arquivo de saida com -o/--output."),
    };

    for f in &inputs {
        if !require_file(Path::new(f)) {
            return 1;
        }
    }
    if !require_tool("qpdf") {
        return 1;
    }

    log(&format!("Mesclando {} arquivo(s) em: {output}", inputs.len()));
    let mut qargs: Vec<String> = vec!["--empty".into(), "--pages".into()];
    qargs.extend(inputs.iter().cloned());
    qargs.push("--".into());
    qargs.push(output.clone());

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
        "Uso: pdf_toolkit merge arquivo1.pdf arquivo2.pdf [...] -o saida.pdf\n\n\
         Mescla dois ou mais PDFs em um unico arquivo, na ordem informada."
    );
}
