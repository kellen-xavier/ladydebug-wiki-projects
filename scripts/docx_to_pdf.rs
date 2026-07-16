//! docx_to_pdf
//!
//! Converte arquivos .docx para PDF usando o LibreOffice (ferramenta externa).
//! Portavel: Linux, macOS e Windows. Sem dependencias externas (apenas a std).
//!
//! Uso:
//!   docx_to_pdf arquivo.docx
//!   docx_to_pdf *.docx --output ./pdfs
//!   docx_to_pdf a.docx b.docx --output "C:\\saida"
//!   docx_to_pdf arquivo.docx --soffice "/caminho/para/soffice"
//!   docx_to_pdf --help
//!
//! Requisito: LibreOffice instalado (fornece o executavel `soffice`/`libreoffice`).

use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

// ─── Localizacao do LibreOffice ────────────────────────────────────────────

/// Procura um executavel no PATH sem depender de `which`/`where`.
fn find_in_path(names: &[&str]) -> Option<PathBuf> {
    let exts: Vec<String> = if IS_WINDOWS {
        env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.BAT;.CMD".to_string())
            .split(';')
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![String::new()]
    };

    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        for name in names {
            for ext in &exts {
                let fname = if name.contains('.') {
                    name.to_string()
                } else {
                    format!("{name}{ext}")
                };
                let cand = dir.join(&fname);
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
    }
    None
}

/// Locais de instalacao conhecidos por sistema operacional.
fn known_locations() -> Vec<PathBuf> {
    if IS_WINDOWS {
        let pf = env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
        let pfx =
            env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".to_string());
        vec![
            PathBuf::from(pf).join(r"LibreOffice\program\soffice.exe"),
            PathBuf::from(pfx).join(r"LibreOffice\program\soffice.exe"),
        ]
    } else if IS_MACOS {
        let mut v = vec![PathBuf::from(
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
        )];
        if let Ok(home) = env::var("HOME") {
            v.push(PathBuf::from(home).join("Applications/LibreOffice.app/Contents/MacOS/soffice"));
        }
        v
    } else {
        let mut v: Vec<PathBuf> = [
            "/usr/bin/soffice",
            "/usr/local/bin/soffice",
            "/snap/bin/libreoffice",
            "/usr/bin/libreoffice",
            "/opt/libreoffice/program/soffice",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        // Cobre instalacoes versionadas: /opt/libreoffice7.6/program/soffice
        if let Ok(entries) = fs::read_dir("/opt") {
            for e in entries.flatten() {
                let name = e.file_name();
                if name.to_string_lossy().starts_with("libreoffice") {
                    v.push(e.path().join("program/soffice"));
                }
            }
        }
        v
    }
}

fn find_soffice(override_path: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = override_path {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    if let Some(p) = find_in_path(&["soffice", "libreoffice"]) {
        return Some(p);
    }
    known_locations().into_iter().find(|p| p.exists())
}

fn install_hint() -> &'static str {
    if IS_WINDOWS {
        "Baixe em https://www.libreoffice.org ou: winget install TheDocumentFoundation.LibreOffice"
    } else if IS_MACOS {
        "Baixe em https://www.libreoffice.org ou: brew install --cask libreoffice"
    } else {
        "Instale com: sudo apt install libreoffice   (ou o gerenciador da sua distro)"
    }
}

// ─── Conversao ─────────────────────────────────────────────────────────────

/// Diretorio temporario unico (a std nao tem mktmpdir).
fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!("{prefix}{}_{}", std::process::id(), nanos))
}

/// Converte um caminho absoluto em URI file:// valida (inclusive no Windows).
fn path_to_file_uri(p: &Path) -> String {
    let abs = fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let mut s = abs.to_string_lossy().replace('\\', "/");
    // canonicalize no Windows retorna prefixo de caminho estendido \\?\C:\...
    if let Some(rest) = s.strip_prefix("//?/") {
        s = rest.to_string();
    }
    if !s.starts_with('/') {
        s = format!("/{s}");
    }
    format!("file://{}", s.replace(' ', "%20"))
}

enum Outcome {
    Converted(PathBuf),
    Skipped,
    Failed(PathBuf),
}

/// Executa o soffice com timeout, matando o processo se estourar.
/// Saida do soffice vai para `log_file`. Retorna Ok(concluiu_no_prazo).
fn run_with_timeout(
    soffice: &Path,
    args: &[String],
    log_file: File,
    timeout: Duration,
) -> std::io::Result<bool> {
    let err_log = log_file.try_clone()?;
    let mut child = Command::new(soffice)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(err_log))
        .spawn()?;

    let start = Instant::now();
    loop {
        match child.try_wait()? {
            Some(_status) => return Ok(true),
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok(false);
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn convert(soffice: &Path, input: &Path, output_dir: &Path, timeout: Duration) -> Outcome {
    let input = match fs::canonicalize(input) {
        Ok(p) => p,
        Err(_) => {
            warn(&format!("Arquivo nao encontrado: {}", input.display()));
            return Outcome::Skipped;
        }
    };

    if !input.is_file() {
        warn(&format!("Arquivo nao encontrado: {}", input.display()));
        return Outcome::Skipped;
    }
    let is_docx = input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("docx"))
        .unwrap_or(false);
    if !is_docx {
        warn(&format!("Ignorando (nao e .docx): {}", input.display()));
        return Outcome::Skipped;
    }

    if let Err(e) = fs::create_dir_all(output_dir) {
        error(&format!("Nao foi possivel criar {}: {e}", output_dir.display()));
        return Outcome::Failed(input);
    }

    // Perfil de usuario isolado por conversao: roda mesmo com o LibreOffice
    // aberto no desktop e evita conflitos em execucoes em lote.
    let profile = unique_temp_dir("lo_profile_");
    if fs::create_dir_all(&profile).is_err() {
        error("Nao foi possivel criar o perfil temporario do LibreOffice.");
        return Outcome::Failed(input);
    }
    let log_path = profile.join("soffice.log");

    let args: Vec<String> = vec![
        "--headless".into(),
        "--norestore".into(),
        "--nolockcheck".into(),
        format!("-env:UserInstallation={}", path_to_file_uri(&profile)),
        "--convert-to".into(),
        "pdf".into(),
        "--outdir".into(),
        output_dir.to_string_lossy().into_owned(),
        input.to_string_lossy().into_owned(),
    ];

    log(&format!(
        "Convertendo: {}",
        input.file_name().unwrap_or_default().to_string_lossy()
    ));

    let finished = match File::create(&log_path) {
        Ok(f) => run_with_timeout(soffice, &args, f, timeout).unwrap_or(false),
        Err(_) => false,
    };

    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let pdf = output_dir.join(format!("{stem}.pdf"));
    let produced = pdf.is_file();

    let outcome = if produced {
        ok(&format!("Gerado: {}", pdf.display()));
        Outcome::Converted(pdf)
    } else {
        if !finished {
            error(&format!("Timeout ao converter: {}", input.display()));
        } else {
            error(&format!("Falha ao converter: {}", input.display()));
        }
        if let Ok(text) = fs::read_to_string(&log_path) {
            let text = text.trim();
            if !text.is_empty() {
                eprintln!("       soffice: {}", text.lines().last().unwrap_or(""));
            }
        }
        Outcome::Failed(input)
    };

    let _ = fs::remove_dir_all(&profile);
    outcome
}

// ─── Glob minimo (std nao expande curingas) ────────────────────────────────

/// Casamento de curinga estilo shell: `*` (qualquer sequencia) e `?` (1 char).
fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut match_i) = (None::<usize>, 0usize);

    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            match_i = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            match_i += 1;
            ti = match_i;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Expande um padrao com curinga no nome do arquivo (ex.: `dir/*.docx`).
/// Sem curinga, devolve o proprio caminho. Curinga apenas no ultimo componente.
fn expand_glob(pattern: &str) -> Vec<PathBuf> {
    if !pattern.contains('*') && !pattern.contains('?') {
        return vec![PathBuf::from(pattern)];
    }
    let path = Path::new(pattern);
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let file_pat = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for e in entries.flatten() {
            if let Some(name) = e.file_name().to_str() {
                if wildcard_match(&file_pat, name) {
                    out.push(e.path());
                }
            }
        }
    }
    out.sort();
    if out.is_empty() {
        vec![PathBuf::from(pattern)] // deixa o aviso de "nao encontrado" acontecer
    } else {
        out
    }
}

// ─── CLI ───────────────────────────────────────────────────────────────────

struct Opts {
    inputs: Vec<String>,
    output: Option<String>,
    soffice: Option<String>,
    timeout: u64,
}

fn print_help(program: &str) {
    println!(
        "Uso: {program} [opcoes] arquivo.docx [arquivo2.docx ...]\n\n\
         Converte arquivos .docx para PDF usando o LibreOffice (Linux, macOS e Windows).\n\n\
         Opcoes:\n\
         \x20 -o, --output DIR    Diretorio de saida (padrao: mesmo do arquivo de entrada)\n\
         \x20 -s, --soffice PATH  Caminho explicito do executavel soffice/libreoffice\n\
         \x20 -t, --timeout SEC   Timeout por arquivo em segundos (padrao: 120)\n\
         \x20 -h, --help          Exibe esta ajuda"
    );
}

/// Retorna Err(codigo) para encerrar cedo (ajuda ou erro de uso).
fn parse_args(argv: &[String], program: &str) -> Result<Opts, i32> {
    let mut opts = Opts {
        inputs: Vec::new(),
        output: None,
        soffice: None,
        timeout: 120,
    };
    let mut i = 0;
    while i < argv.len() {
        let arg = &argv[i];
        match arg.as_str() {
            "-h" | "--help" => {
                print_help(program);
                return Err(0);
            }
            "-o" | "--output" => {
                i += 1;
                match argv.get(i) {
                    Some(v) => opts.output = Some(v.clone()),
                    None => {
                        error("--output exige um diretorio.");
                        return Err(2);
                    }
                }
            }
            "-s" | "--soffice" => {
                i += 1;
                match argv.get(i) {
                    Some(v) => opts.soffice = Some(v.clone()),
                    None => {
                        error("--soffice exige um caminho.");
                        return Err(2);
                    }
                }
            }
            "-t" | "--timeout" => {
                i += 1;
                match argv.get(i).and_then(|v| v.parse::<u64>().ok()) {
                    Some(v) => opts.timeout = v,
                    None => {
                        error("--timeout exige um numero de segundos.");
                        return Err(2);
                    }
                }
            }
            other => opts.inputs.push(other.to_string()),
        }
        i += 1;
    }
    Ok(opts)
}

fn main() {
    let argv: Vec<String> = env::args().collect();
    let program = argv
        .first()
        .map(|p| {
            Path::new(p)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "docx_to_pdf".to_string());

    let opts = match parse_args(&argv[1..], &program) {
        Ok(o) => o,
        Err(code) => std::process::exit(code),
    };

    if opts.inputs.is_empty() {
        error("Nenhum arquivo especificado.");
        print_help(&program);
        std::process::exit(1);
    }

    let soffice = match find_soffice(opts.soffice.as_deref()) {
        Some(p) => p,
        None => {
            error("LibreOffice nao encontrado.");
            error(install_hint());
            error("Em local nao padrao, use: --soffice \"/caminho/para/soffice\"");
            std::process::exit(1);
        }
    };
    log(&format!("LibreOffice: {}", soffice.display()));

    let timeout = Duration::from_secs(opts.timeout);
    let mut converted: Vec<PathBuf> = Vec::new();
    let mut failed: Vec<PathBuf> = Vec::new();

    for pattern in &opts.inputs {
        for file in expand_glob(pattern) {
            let out_dir = match &opts.output {
                Some(d) => PathBuf::from(d),
                None => file
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(".")),
            };
            match convert(&soffice, &file, &out_dir, timeout) {
                Outcome::Converted(p) => converted.push(p),
                Outcome::Failed(p) => failed.push(p),
                Outcome::Skipped => {}
            }
        }
    }

    println!();
    println!("{}", "-".repeat(50));
    println!("Convertidos com sucesso : {}", converted.len());
    println!("Falhas                  : {}", failed.len());
    if !failed.is_empty() {
        println!("\nArquivos com falha:");
        for f in &failed {
            println!("  - {}", f.display());
        }
    }

    std::process::exit(if failed.is_empty() { 0 } else { 1 });
}
