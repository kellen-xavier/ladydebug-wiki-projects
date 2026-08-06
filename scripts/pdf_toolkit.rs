//! pdf_toolkit
//!
//! Kit de operacoes com PDF (mesclar, dividir, girar, extrair texto/imagens,
//! metadados, comprimir, criptografar/descriptografar, marca d'agua).
//! Portavel: Linux, macOS e Windows. Sem dependencias externas (apenas a std).
//!
//! Delega o trabalho binario a ferramentas maduras e ja testadas (mesmo
//! padrao de `docx_to_pdf.rs` e `juntar_pdfs.rb`): qpdf, poppler-utils
//! (pdftotext/pdfimages/pdfinfo) e ghostscript. O programa apenas orquestra
//! esses processos, valida entradas e reporta resultado.
//!
//! Uso:
//!   pdf_toolkit merge a.pdf b.pdf c.pdf -o merged.pdf
//!   pdf_toolkit split relatorio.pdf
//!   pdf_toolkit split relatorio.pdf --ranges "1-5,6-10"
//!   pdf_toolkit rotate scan.pdf --degrees 90 -o scan_girado.pdf
//!   pdf_toolkit extract-text contrato.pdf -o contrato.txt
//!   pdf_toolkit extract-images laudo.pdf --output-dir laudo_imagens
//!   pdf_toolkit metadata documento.pdf
//!   pdf_toolkit compress grande.pdf -o grande_comprimido.pdf
//!   pdf_toolkit encrypt documento.pdf --user-password 123 -o documento_protegido.pdf
//!   pdf_toolkit decrypt documento_protegido.pdf --password 123 -o documento.pdf
//!   pdf_toolkit watermark contrato.pdf --stamp confidencial.pdf -o contrato_marcado.pdf
//!   pdf_toolkit --help
//!
//! Requisitos: qpdf, poppler-utils (pdftotext/pdfimages/pdfinfo) e ghostscript.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ─── Plataforma ────────────────────────────────────────────────────────────

const IS_WINDOWS: bool = cfg!(target_os = "windows");
const IS_MACOS: bool = cfg!(target_os = "macos");

// ─── Logging (ASCII, seguro em qualquer console) ───────────────────────────

fn log(msg: &str) {
    println!("[INFO] {msg}");
}
fn ok(msg: &str) {
    println!("[ OK ] {msg}");
}
fn warn(msg: &str) {
    eprintln!("[WARN] {msg}");
}
fn error(msg: &str) {
    eprintln!("[ERR ] {msg}");
}

fn install_hint() -> &'static str {
    if IS_WINDOWS {
        "Instale via choco: choco install qpdf poppler ghostscript"
    } else if IS_MACOS {
        "Instale via Homebrew: brew install qpdf poppler ghostscript"
    } else {
        "Instale com: sudo apt install qpdf poppler-utils ghostscript"
    }
}

/// Verifica se um executavel existe no PATH. Nao o invoca: as ferramentas
/// aqui usadas (qpdf, gs, pdftotext, pdfimages, pdfinfo) nao compartilham uma
/// flag de verificacao comum (`--version` derruba os utilitarios poppler,
/// por exemplo, tratando-a como nome de arquivo), entao a checagem e feita
/// varrendo o PATH, como em `docx_to_pdf.rs`.
fn tool_available(name: &str) -> bool {
    let exts: Vec<String> = if IS_WINDOWS {
        env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.BAT;.CMD".to_string())
            .split(';')
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![String::new()]
    };
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    for dir in env::split_paths(&path) {
        for ext in &exts {
            if dir.join(format!("{name}{ext}")).is_file() {
                return true;
            }
        }
    }
    false
}

/// Garante que a ferramenta existe; se nao, imprime dica de instalacao e
/// devolve false (o chamador deve abortar o subcomando com codigo de erro).
fn require_tool(name: &str) -> bool {
    if tool_available(name) {
        return true;
    }
    error(&format!("Ferramenta nao encontrada: {name}"));
    error(install_hint());
    false
}

/// Executa um comando e devolve (sucesso, stdout, stderr).
fn run(prog: &str, args: &[String]) -> (bool, String, String) {
    match Command::new(prog).args(args).output() {
        Ok(out) => (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ),
        Err(e) => (false, String::new(), e.to_string()),
    }
}

fn require_file(path: &Path) -> bool {
    if !path.is_file() {
        error(&format!("Arquivo nao encontrado: {}", path.display()));
        return false;
    }
    true
}

fn stem_of(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

// ─── merge ─────────────────────────────────────────────────────────────────

fn cmd_merge(args: &[String]) -> i32 {
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
                print_merge_help();
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

fn print_merge_help() {
    println!(
        "Uso: pdf_toolkit merge arquivo1.pdf arquivo2.pdf [...] -o saida.pdf\n\n\
         Mescla dois ou mais PDFs em um unico arquivo, na ordem informada."
    );
}

// ─── split ─────────────────────────────────────────────────────────────────

fn cmd_split(args: &[String]) -> i32 {
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
                print_split_help();
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

fn print_split_help() {
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

// ─── rotate ────────────────────────────────────────────────────────────────

fn cmd_rotate(args: &[String]) -> i32 {
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
                print_rotate_help();
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

fn print_rotate_help() {
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

// ─── extract-text ───────────────────────────────────────────────────────────

fn cmd_extract_text(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut layout = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).cloned();
            }
            "--layout" => layout = true,
            "-h" | "--help" => {
                print_extract_text_help();
                return 0;
            }
            other if input.is_none() => input = Some(other.to_string()),
            other => return usage_err("extract-text", &format!("Argumento inesperado: {other}")),
        }
        i += 1;
    }

    let input = match input {
        Some(p) => p,
        None => return usage_err("extract-text", "Informe o arquivo PDF de entrada."),
    };
    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("pdftotext") {
        return 1;
    }

    let output = output.unwrap_or_else(|| format!("{}.txt", stem_of(&input_path)));

    let mut pargs: Vec<String> = Vec::new();
    if layout {
        pargs.push("-layout".into());
    }
    pargs.push(input.clone());
    pargs.push(output.clone());

    log(&format!("Extraindo texto de {input}"));
    let (success, _stdout, stderr) = run("pdftotext", &pargs);
    if success {
        ok(&format!("Gerado: {output}"));
        0
    } else {
        error(&format!("pdftotext falhou: {}", stderr.trim()));
        1
    }
}

fn print_extract_text_help() {
    println!(
        "Uso: pdf_toolkit extract-text arquivo.pdf [opcoes]\n\n\
         Extrai o texto do PDF via pdftotext (poppler-utils).\n\n\
         Opcoes:\n\
         \x20 -o, --output PATH   Arquivo .txt de saida (padrao: <nome>.txt)\n\
         \x20     --layout        Preserva o layout original (colunas, espacamento)\n\
         \x20 -h, --help          Exibe esta ajuda"
    );
}

// ─── extract-images ──────────────────────────────────────────────────────

fn cmd_extract_images(args: &[String]) -> i32 {
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
                print_extract_images_help();
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

fn print_extract_images_help() {
    println!(
        "Uso: pdf_toolkit extract-images arquivo.pdf [opcoes]\n\n\
         Extrai todas as imagens embutidas no PDF via pdfimages (poppler-utils).\n\n\
         Opcoes:\n\
         \x20 -o, --output-dir DIR  Diretorio de saida (padrao: <nome>_imagens/)\n\
         \x20 -h, --help            Exibe esta ajuda"
    );
}

// ─── metadata ──────────────────────────────────────────────────────────────

fn cmd_metadata(args: &[String]) -> i32 {
    if args.first().map(String::as_str) == Some("-h") || args.first().map(String::as_str) == Some("--help") {
        println!("Uso: pdf_toolkit metadata arquivo.pdf\n\nExibe metadados e informacoes do PDF via pdfinfo (poppler-utils).");
        return 0;
    }
    let input = match args.first() {
        Some(p) => p.clone(),
        None => return usage_err("metadata", "Informe o arquivo PDF de entrada."),
    };
    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    if !require_tool("pdfinfo") {
        return 1;
    }

    let (success, stdout, stderr) = run("pdfinfo", &[input.clone()]);
    if success {
        print!("{stdout}");
        0
    } else {
        error(&format!("pdfinfo falhou: {}", stderr.trim()));
        1
    }
}

// ─── compress ──────────────────────────────────────────────────────────────

fn cmd_compress(args: &[String]) -> i32 {
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
                print_compress_help();
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

fn print_compress_help() {
    println!(
        "Uso: pdf_toolkit compress arquivo.pdf [opcoes]\n\n\
         Comprime um PDF via ghostscript, reduzindo a resolucao das imagens.\n\n\
         Opcoes:\n\
         \x20 -o, --output PATH    Arquivo de saida (padrao: <nome>_comprimido.pdf)\n\
         \x20 -q, --quality NIVEL  printer (padrao, alta), ebook (media) ou screen (baixa)\n\
         \x20 -h, --help           Exibe esta ajuda"
    );
}

// ─── encrypt / decrypt ──────────────────────────────────────────────────────

fn cmd_encrypt(args: &[String]) -> i32 {
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
                print_encrypt_help();
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

fn print_encrypt_help() {
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

fn cmd_decrypt(args: &[String]) -> i32 {
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
                print_decrypt_help();
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

fn print_decrypt_help() {
    println!(
        "Uso: pdf_toolkit decrypt arquivo.pdf --password SENHA -o saida.pdf\n\n\
         Remove a senha/criptografia de um PDF protegido."
    );
}

// ─── watermark ───────────────────────────────────────────────────────────

fn cmd_watermark(args: &[String]) -> i32 {
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
                print_watermark_help();
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

fn print_watermark_help() {
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

// ─── CLI raiz ────────────────────────────────────────────────────────────

fn usage_err(subcommand: &str, msg: &str) -> i32 {
    error(msg);
    error(&format!("Veja: pdf_toolkit {subcommand} --help"));
    2
}

fn print_help() {
    println!(
        "Uso: pdf_toolkit <comando> [opcoes]\n\n\
         Comandos:\n\
         \x20 merge           Mescla varios PDFs em um so\n\
         \x20 split           Divide um PDF em varios arquivos\n\
         \x20 rotate          Gira paginas de um PDF\n\
         \x20 extract-text    Extrai o texto do PDF\n\
         \x20 extract-images  Extrai as imagens embutidas no PDF\n\
         \x20 metadata        Exibe metadados e informacoes do PDF\n\
         \x20 compress        Comprime um PDF\n\
         \x20 encrypt         Protege um PDF com senha\n\
         \x20 decrypt         Remove a senha de um PDF\n\
         \x20 watermark       Aplica marca d'agua em um PDF\n\n\
         Use 'pdf_toolkit <comando> --help' para detalhes de cada comando."
    );
}

fn main() {
    let argv: Vec<String> = env::args().collect();
    let args = &argv[1..];

    let Some(subcommand) = args.first() else {
        print_help();
        std::process::exit(1);
    };

    let rest = &args[1..];
    let code = match subcommand.as_str() {
        "merge" => cmd_merge(rest),
        "split" => cmd_split(rest),
        "rotate" => cmd_rotate(rest),
        "extract-text" => cmd_extract_text(rest),
        "extract-images" => cmd_extract_images(rest),
        "metadata" => cmd_metadata(rest),
        "compress" => cmd_compress(rest),
        "encrypt" => cmd_encrypt(rest),
        "decrypt" => cmd_decrypt(rest),
        "watermark" => cmd_watermark(rest),
        "-h" | "--help" => {
            print_help();
            0
        }
        other => {
            error(&format!("Comando desconhecido: {other}"));
            print_help();
            2
        }
    };
    std::process::exit(code);
}
