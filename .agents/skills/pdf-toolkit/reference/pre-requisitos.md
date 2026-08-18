# Pré-requisitos por SO e limitações

Referência de apoio à [`SKILL.md`](../SKILL.md). Não linka outro arquivo em
`reference/` — para as flags de cada comando, volte à SKILL.md e siga para
`comandos.md`.

## Instalação

```bash
sudo apt install qpdf poppler-utils ghostscript   # Debian / Ubuntu
brew install qpdf poppler ghostscript             # macOS
choco install qpdf poppler ghostscript            # Windows
```

| Ferramenta | Função | Usada por |
|---|---|---|
| `qpdf` | Mesclar, dividir, girar, criptografar/descriptografar, marca d'água | `merge`, `split`, `rotate`, `encrypt`, `decrypt`, `watermark` |
| `poppler-utils` | Extrair texto (`pdftotext`), imagens (`pdfimages`) e metadados (`pdfinfo`) | `extract-text`, `extract-images`, `metadata` |
| `ghostscript` | Comprimir PDFs reduzindo a resolução das imagens embutidas | `compress` |
| Rust stdlib | Nenhum crate externo necessário | todos |

Se uma ferramenta faltar, o programa informa exatamente qual e como
instalar antes de abortar (`require_tool` em `common.rs`) — nenhum
subcomando falha silenciosamente.

## Limitações

Em relação ao skill original da Anthropic (que usa
`pypdf`/`pdfplumber`/`reportlab`/`pytesseract` em Python), este toolkit
cobre as operações de manipulação de arquivo mas **não** cobre, por não
terem uma ferramenta de linha de comando equivalente confiável sem trazer
dependências pesadas:

- **Extração estruturada de tabelas** (equivalente a
  `pdfplumber.extract_tables`): use `extract-text --layout`, que preserva
  colunas/espaçamento como texto simples.
- **Criação de PDF do zero** (equivalente a `reportlab`): gere o conteúdo
  em Markdown/DOCX e use a skill `docx-to-pdf` para produzir o PDF.
- **OCR de PDFs escaneados** (equivalente a `pytesseract`/`pdf2image`):
  requer `tesseract` + `ocrmypdf`; não incluído por não estarem entre as
  ferramentas já usadas neste repositório. A skill `pdf-leitura-extensa`
  cobre a etapa de rasterizar (`rasterize`) até esse ponto; o OCR em si
  fica para uma ferramenta externa.
- **Preenchimento de formulários PDF** (equivalente ao `FORMS.md` do skill
  original): `qpdf` não edita campos de formulário; requer `pdftk` ou uma
  biblioteca dedicada. Peça se isso vira necessário.
