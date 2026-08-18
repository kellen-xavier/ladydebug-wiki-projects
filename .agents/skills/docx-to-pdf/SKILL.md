---
name: docx-to-pdf
description: >
  Converte arquivos .docx em PDF preservando o layout original (estilos, tabelas,
  imagens e fontes), sem alterar os arquivos de origem. Usa o LibreOffice como
  motor de renderizacao. Use esta skill sempre que o usuario quiser transformar,
  exportar ou gerar PDF a partir de um ou varios .docx — individualmente ou em
  lote via curinga (`*.docx`). Tambem se aplica a expressoes como
  "converter Word para PDF", "docx para pdf", "exportar .docx em PDF" ou
  "gerar o PDF do documento". Nao interpreta nem edita o conteudo: apenas renderiza.
---

# DOCX para PDF

## Índice

- [Visão Geral](#visão-geral)
- [Script: `script/docx_to_pdf.rs`](#script-scriptdocx_to_pdfrs)
- [Build (uma vez)](#build-uma-vez)
- [Uso](#uso)
- [Pré-requisitos](#pré-requisitos)
- [Comportamentos Importantes](#comportamentos-importantes)
- [Limitações](#limitações)

## Visão Geral

Converte `.docx` em PDF com **alta fidelidade de layout**, delegando a
renderização ao LibreOffice (o mesmo motor que abre o documento no editor).

1. Localiza o LibreOffice automaticamente (Linux, macOS e Windows)
2. Renderiza cada `.docx` para PDF **sem modificar o original**
3. Grava o `.pdf` no diretório de saída (padrão: ao lado do `.docx`)
4. Roda **mesmo com o LibreOffice aberto** no desktop, via perfil isolado

**Não-destrutivo por construção**: o programa apenas **lê** o `.docx` e **escreve**
um `.pdf` novo. A origem nunca é movida, renomeada ou regravada.

## Script: `script/docx_to_pdf.rs`

Programa Rust autocontido, **sem dependências externas** (apenas a `std`). Um
único binário, portável entre sistemas. Mesmo padrão do `md2-to-docx.rs`.

## Build (uma vez)

Via Cargo, se o projeto tiver um `Cargo.toml` registrando este binário
(ver exemplo em `[[bin]]` de um `Cargo.toml` na raiz do repositório):

```bash
cargo build --release
# binário em: target/release/docx_to_pdf
```

Como este conversor não tem dependências, também pode ser compilado direto,
sem Cargo (informe a edition usada pelo projeto):

```bash
rustc -O --edition 2021 script/docx_to_pdf.rs -o docx_to_pdf
```

## Uso

```bash
# Um arquivo — PDF ao lado do .docx
docx_to_pdf documento.docx

# Lote, com pasta de saída
docx_to_pdf *.docx --output ./pdfs

# LibreOffice em local não padrão (ex.: Windows)
docx_to_pdf documento.docx --soffice "C:\Program Files\LibreOffice\program\soffice.exe"

# Ajuda
docx_to_pdf --help
```

| Opção            | Função                                                        |
|------------------|---------------------------------------------------------------|
| `-o, --output`   | Diretório de saída (padrão: mesmo do arquivo de entrada)      |
| `-s, --soffice`  | Caminho explícito do executável `soffice`/`libreoffice`      |
| `-t, --timeout`  | Timeout por arquivo em segundos (padrão: 120)                |
| `-h, --help`     | Exibe a ajuda                                                  |

## Pré-requisitos

Apenas o **LibreOffice** instalado (fornece o executável `soffice`):

```bash
sudo apt install libreoffice                       # Debian / Ubuntu
brew install --cask libreoffice                    # macOS
winget install TheDocumentFoundation.LibreOffice   # Windows
```

| Componente     | Função                                             |
|----------------|----------------------------------------------------|
| LibreOffice    | Renderiza o `.docx` e exporta o PDF (fidelidade)   |
| Rust stdlib    | Nenhum crate externo necessário                    |

## Comportamentos Importantes

- **Origem intacta**: o `.docx` nunca é alterado (comprovável por checksum)
- **Descoberta multiplataforma**: varre o `PATH` e os locais de instalação
  conhecidos de cada SO; aceita `--soffice` como override
- **Perfil isolado por conversão**: usa `-env:UserInstallation` num diretório
  temporário, então roda mesmo com o LibreOffice aberto e sem colisão em lote
- **Timeout com kill**: encerra o processo se a conversão travar (evita pipeline pendurado)
- **Curinga próprio**: expande `*.docx` mesmo em shells que não expandem (Windows)
- **Tolerante a falhas**: um arquivo com erro não interrompe os demais; o código
  de saída é `1` se houve qualquer falha, `0` caso contrário

## Limitações

- **Fidelidade = LibreOffice**: recursos muito específicos do Word (alguns campos,
  numeração complexa) podem ter pequenas diferenças de renderização
- **Curinga apenas no nome do arquivo** (ex.: `pasta/*.docx`); `**` recursivo não é suportado
- Converte **apenas** `.docx` → PDF. Para juntar os PDFs resultantes, use a skill `juntar-pdfs`
