//! Subcomando `encrypt`: protege o PDF com senha (AES-256), via qpdf.

use std::path::PathBuf;

use crate::common::{error, log, ok, require_file, require_tool, run, usage_err};

pub fn run_cmd(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut user_password = String::new();
    let mut owner_password = String::new();
    let mut no_print = false;
    let mut no_modify = false;
    let mut no_copy = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).cloned();
            }
            "--user-password" => {
                i += 1;
                match args.get(i) {
                    Some(v) => user_password = v.clone(),
                    None => return usage_err("encrypt", "--user-password exige um valor."),
                }
            }
            "--owner-password" => {
                i += 1;
                match args.get(i) {
                    Some(v) => owner_password = v.clone(),
                    None => return usage_err("encrypt", "--owner-password exige um valor."),
                }
            }
            "--no-print" => no_print = true,
            "--no-modify" => no_modify = true,
            "--no-copy" => no_copy = true,
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            other if input.is_none() => input = Some(other.to_string()),
            other => return usage_err("encrypt", &format!("Argumento inesperado: {other}")),
        }
        i += 1;
    }

    let input = match input {
        Some(p) => p,
        None => return usage_err("encrypt", "Informe o arquivo PDF de entrada."),
    };
    if user_password.is_empty() && owner_password.is_empty() {
        return usage_err("encrypt", "Informe --user-password e/ou --owner-password.");
    }
    let output = match output {
        Some(o) => o,
        None => return usage_err("encrypt", "Informe o arquivo de saida com -o/--output."),
    };
    // Sem senha de dono definida, usa a senha de usuario (qpdf exige as duas
    // posicionais na forma classica usada aqui).
    if owner_password.is_empty() {
        owner_password = user_password.clone();
    }

    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("qpdf") {
        return 1;
    }

    let yn = |restrict: bool| if restrict { "n" } else { "y" }.to_string();
    let qargs = vec![
        "--encrypt".to_string(),
        user_password,
        owner_password,
        "256".to_string(),
        format!("--print={}", if no_print { "none" } else { "full" }),
        format!("--modify={}", if no_modify { "none" } else { "all" }),
        format!("--extract={}", yn(no_copy)),
        "--".to_string(),
        input.clone(),
        output.clone(),
    ];

    log("Criptografando PDF (AES-256)");
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
        "Uso: pdf_toolkit encrypt arquivo.pdf -o saida.pdf [opcoes]\n\n\
         Protege o PDF com senha (AES-256 via qpdf).\n\n\
         Opcoes:\n\
         \x20     --user-password SENHA   Senha para abrir o arquivo\n\
         \x20     --owner-password SENHA  Senha para alterar permissoes (padrao: = user-password)\n\
         \x20     --no-print              Bloqueia impressao\n\
         \x20     --no-modify             Bloqueia edicao do documento\n\
         \x20     --no-copy               Bloqueia extracao de texto/imagem\n\
         \x20 -o, --output PATH           Arquivo de saida (obrigatorio)\n\
         \x20 -h, --help                  Exibe esta ajuda"
    );
}
