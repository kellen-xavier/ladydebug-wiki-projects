//! Subcomando `read-large`: prepara PDFs muito grandes (milhares de paginas)
//! para leitura organizada por topicos.
//!
//! Divide o texto em blocos por intervalo de paginas (via pdftotext) e
//! extrai o sumario/outline nativo do PDF, quando existir (via qpdf --json).
//! Este comando NAO escreve o resumo em si: montar topicos e sumario exige
//! compreensao do conteudo, que e trabalho de quem le os blocos gerados (o
//! agente), nao deste script. Ele so prepara os insumos, na ordem correta,
//! para que essa leitura caiba em blocos gerenciaveis. Veja o fluxo completo
//! na skill `pdf-leitura-extensa`.

use std::fs;
use std::path::PathBuf;

use crate::common::{error, log, ok, require_file, require_tool, run, stem_of, usage_err, warn};

const DEFAULT_MIN_PAGES: u32 = 1000;
const DEFAULT_CHUNK_PAGES: u32 = 50;

pub fn run_cmd(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output_dir: Option<String> = None;
    let mut to_dir: Option<String> = None;
    let mut min_pages = DEFAULT_MIN_PAGES;
    let mut chunk_pages = DEFAULT_CHUNK_PAGES;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output-dir" => {
                i += 1;
                output_dir = args.get(i).cloned();
            }
            "--to" => {
                i += 1;
                to_dir = args.get(i).cloned();
            }
            "--min-pages" => {
                i += 1;
                match args.get(i).and_then(|v| v.parse::<u32>().ok()) {
                    Some(v) => min_pages = v,
                    None => return usage_err("read-large", "--min-pages exige um numero inteiro."),
                }
            }
            "--chunk-pages" => {
                i += 1;
                match args.get(i).and_then(|v| v.parse::<u32>().ok()) {
                    Some(v) if v > 0 => chunk_pages = v,
                    _ => {
                        return usage_err(
                            "read-large",
                            "--chunk-pages exige um numero inteiro maior que zero.",
                        )
                    }
                }
            }
            "-h" | "--help" => {
                print_help();
                return 0;
            }
            other if input.is_none() => input = Some(other.to_string()),
            other => return usage_err("read-large", &format!("Argumento inesperado: {other}")),
        }
        i += 1;
    }

    if output_dir.is_some() && to_dir.is_some() {
        return usage_err("read-large", "Use apenas uma opcao: --output-dir ou --to.");
    }

    let input = match input {
        Some(p) => p,
        None => return usage_err("read-large", "Informe o arquivo PDF de entrada."),
    };
    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("qpdf") || !require_tool("pdftotext") {
        return 1;
    }

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

    // Guarda de escopo: este comando existe para o caso especifico de PDFs
    // enormes, onde o texto extraido nao cabe em uma unica leitura. Para
    // arquivos menores, `extract-text` ja resolve em um unico passo.
    if npages < min_pages {
        error(&format!(
            "read-large e exclusivo para PDFs muito grandes (>= {min_pages} paginas); {} tem {npages}.",
            input_path.display()
        ));
        error("Para PDFs menores, use: pdf_toolkit extract-text");
        error("(ou ajuste o limiar com --min-pages, se nao se aplicar ao seu caso)");
        return 2;
    }

    let stem = stem_of(&input_path);
    // Com --to, os blocos vao direto para o diretorio informado (ex.:
    // src/dominios/<nome-projeto>/arquivos-texto/), sem subpasta "chunks/",
    // para casar com a regra do AGENTS.md de ler os .md de arquivos-texto
    // via ripgrep. Sem --to, mantem a estrutura padrao <nome>_leitura/chunks/.
    let (out_dir, chunks_dir) = match to_dir {
        Some(dir) => {
            let d = PathBuf::from(dir);
            (d.clone(), d)
        }
        None => {
            let out_dir = PathBuf::from(output_dir.unwrap_or_else(|| format!("{stem}_leitura")));
            let chunks_dir = out_dir.join("chunks");
            (out_dir, chunks_dir)
        }
    };
    if let Err(e) = fs::create_dir_all(&chunks_dir) {
        error(&format!("Nao foi possivel criar {}: {e}", chunks_dir.display()));
        return 1;
    }

    log(&format!("{npages} paginas, {chunk_pages} por bloco"));

    // Sumario nativo do PDF (marcadores/outline), se existir. Nao e parseado
    // aqui: o agente le o JSON diretamente ao montar os topicos, evitando
    // reimplementar um parser JSON so para isso.
    let (outline_ok, outline_json, _stderr) = run(
        "qpdf",
        &["--json".into(), "--json-key=outlines".into(), input.clone()],
    );
    let has_outline = outline_ok && outline_json.contains("\"title\"");
    if outline_ok {
        let outline_path = out_dir.join("outline.json");
        if let Err(e) = fs::write(&outline_path, &outline_json) {
            warn(&format!("Nao foi possivel salvar {}: {e}", outline_path.display()));
        }
    }

    let mut chunks: Vec<(u32, u32, String)> = Vec::new();
    let mut start = 1u32;
    while start <= npages {
        let end = (start + chunk_pages - 1).min(npages);
        let file_name = format!("{stem}_p{start:05}-{end:05}.md");
        let chunk_file = chunks_dir.join(&file_name);
        // "-" como arquivo de saida manda o pdftotext escrever no stdout, em
        // vez de num .txt: assim da para inserir a ancora de pagina
        // (`<!-- pagina: N -->`) entre as paginas antes de gravar o .md.
        let pargs = vec![
            "-layout".to_string(),
            "-f".to_string(),
            start.to_string(),
            "-l".to_string(),
            end.to_string(),
            input.clone(),
            "-".to_string(),
        ];
        let (success, page_stdout, stderr) = run("pdftotext", &pargs);
        if success {
            // pdftotext separa paginas por form-feed (0x0C); uma ancora por
            // pagina deixa um hit de ripgrep carregar o numero da pagina.
            let mut md = String::new();
            for (offset, page_text) in page_stdout.split('\u{0C}').enumerate() {
                let page_num = start + offset as u32;
                if page_num > end {
                    break;
                }
                md.push_str(&format!("<!-- pagina: {page_num} -->\n\n"));
                md.push_str(page_text.trim_end());
                md.push_str("\n\n");
            }
            if let Err(e) = fs::write(&chunk_file, md) {
                warn(&format!("  falha ao escrever {}: {e}", chunk_file.display()));
            } else {
                chunks.push((start, end, file_name));
            }
        } else {
            warn(&format!("  falha nas paginas {start}-{end}: {}", stderr.trim()));
        }
        start = end + 1;
    }

    if chunks.is_empty() {
        error("Nenhum bloco de texto foi gerado.");
        return 1;
    }

    // manifest.md: indice dos blocos, na ordem em que devem ser lidos.
    let mut manifest = String::new();
    manifest.push_str(&format!("# Leitura: {}\n\n", input_path.display()));
    manifest.push_str(&format!("- Paginas totais: {npages}\n"));
    manifest.push_str(&format!("- Paginas por bloco: {chunk_pages}\n"));
    manifest.push_str(&format!("- Blocos gerados: {}\n", chunks.len()));
    manifest.push_str(&format!(
        "- Sumario nativo (marcadores do PDF): {}\n\n",
        if has_outline { "ver outline.json" } else { "nao encontrado" }
    ));
    manifest.push_str("## Blocos, em ordem de leitura\n\n");
    manifest.push_str("| # | Paginas | Arquivo |\n|---|---------|---------|\n");
    let chunks_rel = if chunks_dir == out_dir { "" } else { "chunks/" };
    for (idx, (s, e, name)) in chunks.iter().enumerate() {
        manifest.push_str(&format!("| {} | {s}-{e} | `{chunks_rel}{name}` |\n", idx + 1));
    }

    let manifest_path = out_dir.join("manifest.md");
    if let Err(e) = fs::write(&manifest_path, manifest) {
        error(&format!("Nao foi possivel escrever {}: {e}", manifest_path.display()));
        return 1;
    }

    ok(&format!(
        "{} bloco(s) gerado(s) em {} — leia manifest.md para continuar",
        chunks.len(),
        out_dir.display()
    ));
    0
}

pub fn print_help() {
    println!(
        "Uso: pdf_toolkit read-large arquivo.pdf [opcoes]\n\n\
         Prepara um PDF muito grande para leitura organizada por topicos:\n\
         divide o texto em blocos .md por pagina (com ancora '<!-- pagina: N -->'\n\
         antes do texto de cada pagina) e extrai o outline nativo do PDF (se\n\
         existir). Nao gera o resumo — apenas os insumos para que a leitura\n\
         seja feita em blocos. Exclusivo para PDFs com pelo menos --min-pages\n\
         paginas (padrao: {DEFAULT_MIN_PAGES}); para PDFs menores, use\n\
         'pdf_toolkit extract-text'.\n\n\
         Opcoes:\n\
         \x20 -o, --output-dir DIR   Diretorio de saida (padrao: <nome>_leitura/)\n\
         \x20     --to DIR           Escreve manifest/outline/blocos direto em DIR,\n\
         \x20                        sem subpasta chunks/ (ex.: arquivos-texto/ do\n\
         \x20                        projeto). Exclusivo com --output-dir.\n\
         \x20     --chunk-pages N    Paginas por bloco de texto (padrao: {DEFAULT_CHUNK_PAGES})\n\
         \x20     --min-pages N      Paginas minimas exigidas (padrao: {DEFAULT_MIN_PAGES})\n\
         \x20 -h, --help             Exibe esta ajuda"
    );
}
