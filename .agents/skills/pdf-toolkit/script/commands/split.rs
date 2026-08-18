//! Subcomando `split`: divide um PDF em varios arquivos, via qpdf.

use std::fs;
use std::path::PathBuf;

use crate::common::{error, log, ok, require_file, require_tool, run, stem_of, usage_err, warn};

pub fn run_cmd(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output_dir: Option<String> = None;
    let mut ranges: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output-dir" => {
                i += 1;
                match args.get(i) {
                    Some(v) => output_dir = Some(v.clone()),
                    None => return usage_err("split", "--output-dir exige um diretorio."),
                }
            }
            "-r" | "--ranges" => {
                i += 1;
                match args.get(i) {
                    Some(v) => ranges = Some(v.clone()),
                    None => return usage_err("split", "--ranges exige uma lista de intervalos."),
                }
            }
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            other if input.is_none() => input = Some(other.to_string()),
            other => return usage_err("split", &format!("Argumento inesperado: {other}")),
        }
        i += 1;
    }

    let input = match input {
        Some(p) => p,
        None => return usage_err("split", "Informe o arquivo PDF de entrada."),
    };
    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("qpdf") {
        return 1;
    }

    let out_dir = PathBuf::from(
        output_dir.unwrap_or_else(|| format!("{}_split", stem_of(&input_path))),
    );
    if let Err(e) = fs::create_dir_all(&out_dir) {
        error(&format!("Nao foi possivel criar {}: {e}", out_dir.display()));
        return 1;
    }

    // Grupos de paginas a extrair: um por --ranges informado, ou uma pagina
    // por grupo (1, 2, 3, ...) quando --ranges nao e informado.
    let groups: Vec<String> = match ranges {
        Some(spec) => spec.split(',').map(|s| s.trim().to_string()).collect(),
        None => {
            let (success, stdout, stderr) = run("qpdf", &["--show-npages".into(), input.clone()]);
            if !success {
                error(&format!("Nao foi possivel obter o numero de paginas: {}", stderr.trim()));
                return 1;
            }
            let npages: u32 = match stdout.trim().parse() {
                Ok(n) => n,
                Err(_) => {
                    error("Saida inesperada de qpdf --show-npages.");
                    return 1;
                }
            };
            (1..=npages).map(|n| n.to_string()).collect()
        }
    };

    let stem = stem_of(&input_path);
    let mut produced: Vec<PathBuf> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for range in &groups {
        let safe_name = range.replace(['-', ','], "_");
        let out_file = out_dir.join(format!("{stem}_p{safe_name}.pdf"));
        let qargs = vec![
            input.clone(),
            "--pages".into(),
            ".".into(),
            range.clone(),
            "--".into(),
            out_file.to_string_lossy().into_owned(),
        ];
        let (success, _stdout, stderr) = run("qpdf", &qargs);
        if success {
            log(&format!("  paginas {range} -> {}", out_file.display()));
            produced.push(out_file);
        } else {
            warn(&format!("  falha nas paginas {range}: {}", stderr.trim()));
            failed.push(range.clone());
        }
    }

    ok(&format!("{} arquivo(s) gerado(s) em {}", produced.len(), out_dir.display()));
    if !failed.is_empty() {
        error(&format!("Intervalos com falha: {}", failed.join(", ")));
        return 1;
    }
    0
}

pub fn print_help() {
    println!(
        "Uso: pdf_toolkit split arquivo.pdf [opcoes]\n\n\
         Divide um PDF em varios arquivos.\n\n\
         Opcoes:\n\
         \x20 -o, --output-dir DIR  Diretorio de saida (padrao: <nome>_split/)\n\
         \x20 -r, --ranges LISTA    Intervalos separados por virgula (ex.: \"1-5,6-10\").\n\
         \x20                       Sem esta opcao, gera um arquivo por pagina.\n\
         \x20 -h, --help            Exibe esta ajuda"
    );
}
