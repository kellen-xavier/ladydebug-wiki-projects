//! Subcomando `rasterize`: converte paginas de um PDF em imagens raster
//! (PNG/JPEG) via pdftoppm, para o caso de PDF escaneado (sem camada de
//! texto) onde a leitura exige OCR externo. Imprime os nomes de arquivo
//! gerados: o zero-padding do pdftoppm varia com o total de paginas
//! processadas, e adivinhar o nome e um erro comum.

use std::fs;
use std::path::PathBuf;

use crate::common::{error, ok, require_file, require_tool, run, stem_of, usage_err};

const DEFAULT_DPI: u32 = 150;

pub fn run_cmd(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output_dir: Option<String> = None;
    let mut first: Option<u32> = None;
    let mut last: Option<u32> = None;
    let mut dpi = DEFAULT_DPI;
    let mut format = "png".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output-dir" => {
                i += 1;
                output_dir = args.get(i).cloned();
            }
            "-f" | "--first" => {
                i += 1;
                match args.get(i).and_then(|v| v.parse::<u32>().ok()) {
                    Some(v) => first = Some(v),
                    None => return usage_err("rasterize", "--first exige um numero inteiro."),
                }
            }
            "-l" | "--last" => {
                i += 1;
                match args.get(i).and_then(|v| v.parse::<u32>().ok()) {
                    Some(v) => last = Some(v),
                    None => return usage_err("rasterize", "--last exige um numero inteiro."),
                }
            }
            "--dpi" => {
                i += 1;
                match args.get(i).and_then(|v| v.parse::<u32>().ok()) {
                    Some(v) if v > 0 => dpi = v,
                    _ => return usage_err("rasterize", "--dpi exige um numero inteiro maior que zero."),
                }
            }
            "--format" => {
                i += 1;
                match args.get(i).map(String::as_str) {
                    Some("png") | Some("jpeg") => format = args[i].clone(),
                    _ => return usage_err("rasterize", "--format aceita apenas 'png' ou 'jpeg'."),
                }
            }
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            other if input.is_none() => input = Some(other.to_string()),
            other => return usage_err("rasterize", &format!("Argumento inesperado: {other}")),
        }
        i += 1;
    }

    let input = match input {
        Some(p) => p,
        None => return usage_err("rasterize", "Informe o arquivo PDF de entrada."),
    };
    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("pdftoppm") {
        return 1;
    }

    let stem = stem_of(&input_path);
    let out_dir = PathBuf::from(output_dir.unwrap_or_else(|| format!("{stem}_rasterizado")));
    if let Err(e) = fs::create_dir_all(&out_dir) {
        error(&format!("Nao foi possivel criar {}: {e}", out_dir.display()));
        return 1;
    }
    let prefix = out_dir.join(&stem);

    let mut pargs: Vec<String> = vec!["-r".into(), dpi.to_string()];
    if let Some(f) = first {
        pargs.push("-f".into());
        pargs.push(f.to_string());
    }
    if let Some(l) = last {
        pargs.push("-l".into());
        pargs.push(l.to_string());
    }
    pargs.push(format!("-{format}"));
    pargs.push(input.clone());
    pargs.push(prefix.to_string_lossy().into_owned());

    let (success, _stdout, stderr) = run("pdftoppm", &pargs);
    if !success {
        error(&format!("pdftoppm falhou: {}", stderr.trim()));
        return 1;
    }

    // pdftoppm nao imprime os nomes gerados; listamos o diretorio de saida
    // filtrando pelo prefixo, em vez de tentar adivinhar o zero-padding.
    let ext = if format == "jpeg" { "jpg" } else { "png" };
    let mut generated: Vec<String> = match fs::read_dir(&out_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(&stem) && name.ends_with(&format!(".{ext}")))
            .collect(),
        Err(e) => {
            error(&format!("Nao foi possivel listar {}: {e}", out_dir.display()));
            return 1;
        }
    };
    generated.sort();

    if generated.is_empty() {
        error("Nenhuma imagem foi gerada.");
        return 1;
    }

    for name in &generated {
        println!("  {}", out_dir.join(name).display());
    }
    ok(&format!("{} imagem(ns) gerada(s) em {}", generated.len(), out_dir.display()));
    0
}

pub fn print_help() {
    println!(
        "Uso: pdf_toolkit rasterize arquivo.pdf [opcoes]\n\n\
         Converte paginas do PDF em imagens raster (PNG ou JPEG) via pdftoppm.\n\
         Imprime os nomes de arquivo gerados (o zero-padding varia com o total\n\
         de paginas processadas).\n\n\
         Opcoes:\n\
         \x20 -o, --output-dir DIR   Diretorio de saida (padrao: <nome>_rasterizado/)\n\
         \x20 -f, --first N          Primeira pagina (padrao: 1)\n\
         \x20 -l, --last N           Ultima pagina (padrao: ultima do PDF)\n\
         \x20     --dpi N            Resolucao em DPI (padrao: {DEFAULT_DPI})\n\
         \x20     --format png|jpeg  Formato de saida (padrao: png)\n\
         \x20 -h, --help             Exibe esta ajuda"
    );
}
