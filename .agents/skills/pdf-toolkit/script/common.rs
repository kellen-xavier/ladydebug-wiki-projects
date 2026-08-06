//! Utilidades compartilhadas por todos os subcomandos: plataforma, logging,
//! deteccao de ferramentas externas e execucao de processos.

use std::env;
use std::path::Path;
use std::process::Command;

// ─── Plataforma ────────────────────────────────────────────────────────────

pub const IS_WINDOWS: bool = cfg!(target_os = "windows");
pub const IS_MACOS: bool = cfg!(target_os = "macos");

// ─── Logging (ASCII, seguro em qualquer console) ───────────────────────────

pub fn log(msg: &str) {
    println!("[INFO] {msg}");
}
pub fn ok(msg: &str) {
    println!("[ OK ] {msg}");
}
pub fn warn(msg: &str) {
    eprintln!("[WARN] {msg}");
}
pub fn error(msg: &str) {
    eprintln!("[ERR ] {msg}");
}

/// Erro de uso de um subcomando: reporta a mensagem, aponta para o --help
/// correspondente e devolve o codigo de saida padrao (2) para o chamador.
pub fn usage_err(subcommand: &str, msg: &str) -> i32 {
    error(msg);
    error(&format!("Veja: pdf_toolkit {subcommand} --help"));
    2
}

pub fn install_hint() -> &'static str {
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
pub fn tool_available(name: &str) -> bool {
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
pub fn require_tool(name: &str) -> bool {
    if tool_available(name) {
        return true;
    }
    error(&format!("Ferramenta nao encontrada: {name}"));
    error(install_hint());
    false
}

/// Executa um comando e devolve (sucesso, stdout, stderr).
pub fn run(prog: &str, args: &[String]) -> (bool, String, String) {
    match Command::new(prog).args(args).output() {
        Ok(out) => (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ),
        Err(e) => (false, String::new(), e.to_string()),
    }
}

pub fn require_file(path: &Path) -> bool {
    if !path.is_file() {
        error(&format!("Arquivo nao encontrado: {}", path.display()));
        return false;
    }
    true
}

pub fn stem_of(path: &Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
