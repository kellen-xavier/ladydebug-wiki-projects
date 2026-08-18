//! Subcomando `watermark`: sobrepoe (ou coloca atras de) as paginas de um PDF
//! sobre outro, via qpdf --overlay/--underlay.

use std::path::Path;

use crate::common::{error, log, ok, require_file, require_tool, run, usage_err};

pub fn run_cmd(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut stamp: Option<String> = None;
    let mut underlay = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).cloned();
            }
            "-s" | "--stamp" => {
                i += 1;
                stamp = args.get(i).cloned();
            }
            "--underlay" => underlay = true,
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            other if input.is_none() => input = Some(other.to_string()),
            other => return usage_err("watermark", &format!("Argumento inesperado: {other}")),
        }
        i += 1;
    }

    let input = match input {
        Some(p) => p,
        None => return usage_err("watermark", "Informe o arquivo PDF de entrada."),
    };
    let stamp = match stamp {
        Some(s) => s,
        None => return usage_err("watermark", "Informe o PDF da marca d'agua com -s/--stamp."),
    };
    let output = match output {
        Some(o) => o,
        None => return usage_err("watermark", "Informe o arquivo de saida com -o/--output."),
    };

    if !require_file(Path::new(&input)) || !require_file(Path::new(&stamp)) {
        return 1;
    }
    if !require_tool("qpdf") {
        return 1;
    }

    let flag = if underlay { "--underlay" } else { "--overlay" };
    log(&format!("Aplicando marca d'agua ({})", if underlay { "underlay" } else { "overlay" }));
    let qargs = vec![
        input.clone(),
        output.clone(),
        flag.to_string(),
        stamp.clone(),
        "--repeat=1-z".to_string(),
        "--".to_string(),
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
        "Uso: pdf_toolkit watermark arquivo.pdf --stamp marca.pdf -o saida.pdf [opcoes]\n\n\
         Sobrepoe as paginas de um PDF de marca d'agua sobre (ou sob) cada\n\
         pagina do documento de entrada.\n\n\
         Opcoes:\n\
         \x20 -s, --stamp PATH   PDF de uma pagina com a marca d'agua (obrigatorio)\n\
         \x20     --underlay     Desenha a marca atras do conteudo (padrao: sobre)\n\
         \x20 -o, --output PATH  Arquivo de saida (obrigatorio)\n\
         \x20 -h, --help         Exibe esta ajuda"
    );
}
