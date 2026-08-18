//! Subcomando `inventory`: levanta o inventario de conteudo de um PDF antes
//! de decidir a estrategia de leitura. Roda pdfinfo + pdffonts +
//! pdfimages -list + pdfdetach -list e emite um veredito de estrategia:
//! camada de texto presente -> extract-text/read-large; sem fontes
//! embutidas -> PDF escaneado, rasterize + OCR. Sequencia fixa, sem opcoes
//! alem do arquivo — baixo grau de liberdade por design. Veja o fluxo
//! completo na skill `pdf-leitura-extensa`.

use std::path::PathBuf;

use crate::common::{error, ok, require_file, require_tool, run, usage_err, warn};

pub fn run_cmd(args: &[String]) -> i32 {
    if matches!(args.first().map(String::as_str), Some("-h") | Some("--help")) {
        print_help();
        return 0;
    }
    let input = match args.first() {
        Some(p) => p.clone(),
        None => return usage_err("inventory", "Informe o arquivo PDF de entrada."),
    };
    let input_path = PathBuf::from(&input);
    if !require_file(&input_path) {
        return 1;
    }
    for tool in ["pdfinfo", "pdffonts", "pdfimages", "pdfdetach"] {
        if !require_tool(tool) {
            return 1;
        }
    }

    println!("== pdfinfo ==");
    let (info_ok, info_out, info_err) = run("pdfinfo", &[input.clone()]);
    if info_ok {
        print!("{info_out}");
    } else {
        error(&format!("pdfinfo falhou: {}", info_err.trim()));
    }

    println!("\n== pdffonts ==");
    let (fonts_ok, fonts_out, fonts_err) = run("pdffonts", &[input.clone()]);
    if fonts_ok {
        print!("{fonts_out}");
    } else {
        error(&format!("pdffonts falhou: {}", fonts_err.trim()));
    }

    println!("\n== pdfimages -list ==");
    let (images_ok, images_out, images_err) = run("pdfimages", &["-list".into(), input.clone()]);
    if images_ok {
        print!("{images_out}");
    } else {
        error(&format!("pdfimages falhou: {}", images_err.trim()));
    }

    println!("\n== pdfdetach -list ==");
    let (detach_ok, detach_out, detach_err) = run("pdfdetach", &["-list".into(), input.clone()]);
    if detach_ok {
        print!("{detach_out}");
    } else {
        error(&format!("pdfdetach falhou: {}", detach_err.trim()));
    }

    // Veredito: pdffonts imprime duas linhas de cabecalho fixas seguidas de
    // uma linha por fonte embutida. Sem linhas de fonte, o PDF nao tem
    // camada de texto extraivel (escaneado).
    let font_lines = fonts_out.lines().skip(2).filter(|l| !l.trim().is_empty()).count();

    println!("\n== Veredito ==");
    if !fonts_ok {
        warn("Nao foi possivel determinar (pdffonts falhou). Inspecione manualmente.");
    } else if font_lines > 0 {
        ok("Camada de texto presente -> use 'extract-text' (ou 'read-large' se o PDF for muito grande).");
    } else {
        ok("Sem fontes embutidas -> PDF provavelmente escaneado. Use 'rasterize' e processe as imagens com OCR externo.");
    }

    0
}

pub fn print_help() {
    println!(
        "Uso: pdf_toolkit inventory arquivo.pdf\n\n\
         Levanta o inventario de conteudo do PDF (pdfinfo + pdffonts +\n\
         pdfimages -list + pdfdetach -list) e emite um veredito de estrategia\n\
         de leitura: camada de texto presente -> extract-text/read-large;\n\
         sem fontes embutidas -> PDF escaneado, use rasterize + OCR externo."
    );
}
