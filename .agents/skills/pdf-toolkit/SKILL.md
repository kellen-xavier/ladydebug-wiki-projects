---
name: pdf-toolkit
description: >
  Executa operações com arquivos PDF: mesclar, dividir, girar páginas,
  extrair texto, extrair imagens, ver metadados, comprimir, proteger com
  senha (criptografar), remover senha (descriptografar) e aplicar marca
  d'água. Use esta skill sempre que o usuário mencionar um arquivo .pdf e
  pedir para juntar, unir, separar, girar/rotacionar, extrair texto/tabelas
  de, extrair imagens de, comprimir, proteger, descriptografar ou
  carimbar/marcar um PDF.
---

# PDF Toolkit

## Visão Geral

Kit de linha de comando para as operações mais comuns com PDF, baseado no
skill oficial [`pdf`](https://github.com/anthropics/skills/tree/main/skills/pdf)
da Anthropic, porém reimplementado em **Rust** para seguir o padrão deste
repositório (binário único, portável, sem runtime Python).

Assim como `docx_to_pdf.rs` e `juntar_pdfs.rb`, o programa **não reimplementa
o formato PDF**: ele orquestra ferramentas maduras e amplamente testadas —
`qpdf`, `poppler-utils` (`pdftotext`/`pdfimages`/`pdfinfo`) e `ghostscript` —
validando entradas e relatando o resultado.

## Script: `script/`

Programa Rust autocontido (**apenas a `std`**, sem crates externos), dividido
em módulos curtos e objetivos — um arquivo por subcomando — no mesmo espírito
dos scripts individuais do [skill de referência](https://github.com/anthropics/skills/tree/main/skills/pdf/scripts):

```
script/
  main.rs              # CLI: parseia o subcomando e despacha
  common.rs            # logging, detecção de ferramentas, execução de processos
  commands/
    mod.rs
    merge.rs            extract_text.rs      encrypt.rs
    split.rs             extract_images.rs    decrypt.rs
    rotate.rs            metadata.rs          watermark.rs
    compress.rs
```

Cada `commands/<nome>.rs` expõe `run_cmd(args)` e `print_help()` e não
ultrapassa ~125 linhas. `common.rs` reúne apenas o que é compartilhado entre
os subcomandos (logging, verificação de ferramentas externas no PATH,
execução de processos, helpers de caminho).

## Build (uma vez)

```bash
cargo build --release
# binário em: target/release/pdf_toolkit
```

## Comandos

| Comando          | Faz                                              | Ferramenta   |
|-------------------|---------------------------------------------------|--------------|
| `merge`           | Mescla vários PDFs em um só, na ordem informada   | `qpdf`       |
| `split`           | Divide um PDF (por página ou por intervalos)      | `qpdf`       |
| `rotate`          | Gira páginas em múltiplos de 90°                  | `qpdf`       |
| `extract-text`    | Extrai o texto do PDF (com ou sem layout)         | `pdftotext`  |
| `extract-images`  | Extrai as imagens embutidas no PDF                | `pdfimages`  |
| `metadata`        | Exibe título, autor, nº de páginas, criptografia…  | `pdfinfo`    |
| `compress`        | Reduz o tamanho comprimindo imagens embutidas     | `ghostscript`|
| `encrypt`         | Protege o PDF com senha (AES-256)                 | `qpdf`       |
| `decrypt`         | Remove a senha/criptografia de um PDF protegido   | `qpdf`       |
| `watermark`       | Sobrepõe (ou coloca atrás) as páginas de outro PDF| `qpdf`       |

## Uso

```bash
pdf_toolkit merge a.pdf b.pdf c.pdf -o merged.pdf

pdf_toolkit split relatorio.pdf                          # 1 arquivo por página
pdf_toolkit split relatorio.pdf --ranges "1-5,6-10"       # por intervalo

pdf_toolkit rotate scan.pdf --degrees 90 -o scan_girado.pdf
pdf_toolkit rotate scan.pdf --degrees -90 --pages 2,4-6 -o scan_girado.pdf

pdf_toolkit extract-text contrato.pdf -o contrato.txt
pdf_toolkit extract-text contrato.pdf --layout -o contrato.txt

pdf_toolkit extract-images laudo.pdf --output-dir laudo_imagens

pdf_toolkit metadata documento.pdf

pdf_toolkit compress grande.pdf -o grande_comprimido.pdf
pdf_toolkit compress grande.pdf --quality screen -o grande_web.pdf

pdf_toolkit encrypt documento.pdf --user-password 123 -o documento_protegido.pdf
pdf_toolkit encrypt documento.pdf --user-password 123 --no-print --no-copy -o documento_protegido.pdf

pdf_toolkit decrypt documento_protegido.pdf --password 123 -o documento.pdf

pdf_toolkit watermark contrato.pdf --stamp confidencial.pdf -o contrato_marcado.pdf

pdf_toolkit --help
pdf_toolkit <comando> --help   # opções detalhadas de cada comando
```

## Pré-requisitos

```bash
sudo apt install qpdf poppler-utils ghostscript   # Debian / Ubuntu
brew install qpdf poppler ghostscript             # macOS
choco install qpdf poppler ghostscript            # Windows
```

| Ferramenta      | Função                                                        |
|-----------------|-----------------------------------------------------------------|
| `qpdf`          | Mesclar, dividir, girar, criptografar/descriptografar, marca d'água |
| `poppler-utils` | Extrair texto (`pdftotext`), imagens (`pdfimages`) e metadados (`pdfinfo`) |
| `ghostscript`   | Comprimir PDFs reduzindo a resolução das imagens embutidas    |
| Rust stdlib     | Nenhum crate externo necessário                                |

Se uma ferramenta faltar, o programa informa exatamente qual e como instalar
antes de abortar — nenhum subcomando falha silenciosamente.

## Comportamentos Importantes

- **Não-destrutivo por construção**: cada comando lê a entrada e escreve uma
  saída nova; o arquivo original nunca é sobrescrito por padrão.
- **`split` sem `--ranges`**: gera um PDF por página, numerado
  (`<nome>_p1.pdf`, `<nome>_p2.pdf`, ...), dentro de `<nome>_split/`.
- **`rotate` sem `--pages`**: aplica a rotação a todas as páginas.
- **`encrypt`**: usa AES-256 (`qpdf --encrypt ... 256`); se apenas
  `--user-password` for informado, a senha de dono usa o mesmo valor.
- **`watermark`**: por padrão sobrepõe (`--overlay`); use `--underlay` para
  desenhar atrás do conteúdo original. O PDF de carimbo é repetido em todas
  as páginas de saída (`--repeat=1-z`).

## Limitações

Em relação ao skill original (que usa `pypdf`/`pdfplumber`/`reportlab`/`pytesseract`
em Python), este toolkit cobre as operações de manipulação de arquivo mas
**não** cobre, por não terem uma ferramenta de linha de comando equivalente
confiável sem trazer dependências pesadas:

- **Extração estruturada de tabelas** (equivalente a `pdfplumber.extract_tables`):
  use `extract-text --layout`, que preserva colunas/espaçamento como texto simples.
- **Criação de PDF do zero** (equivalente a `reportlab`): gere o conteúdo em
  Markdown/DOCX e use a skill `docx-to-pdf` para produzir o PDF.
- **OCR de PDFs escaneados** (equivalente a `pytesseract`/`pdf2image`): requer
  `tesseract` + `ocrmypdf`; não incluído por não estarem entre as ferramentas
  já usadas neste repositório. Se necessário, peça para adicionar um
  subcomando `ocr` que os invoque.
- **Preenchimento de formulários PDF** (equivalente ao `FORMS.md` do skill
  original): `qpdf` não edita campos de formulário; requer `pdftk` ou uma
  biblioteca dedicada. Peça se isso vira necessário.
