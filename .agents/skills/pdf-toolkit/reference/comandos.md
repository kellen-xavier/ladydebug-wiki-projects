# Comandos — flags detalhadas

Referência de apoio à [`SKILL.md`](../SKILL.md). Não linka outro arquivo em
`reference/` — para pré-requisitos, volte à SKILL.md e siga para
`pre-requisitos.md`.

## merge

```bash
pdf_toolkit merge a.pdf b.pdf c.pdf -o merged.pdf
```

Mescla, na ordem dos argumentos informados, via `qpdf`. `-o`/`--output`
obrigatório.

## split

```bash
pdf_toolkit split relatorio.pdf                          # 1 arquivo por pagina
pdf_toolkit split relatorio.pdf --ranges "1-5,6-10"       # por intervalo
```

- `-o`, `--output-dir DIR` — padrão `<nome>_split/`.
- `-r`, `--ranges LISTA` — intervalos separados por vírgula. Sem esta
  opção, gera um arquivo por página.

## rotate

```bash
pdf_toolkit rotate scan.pdf --degrees 90 -o scan_girado.pdf
pdf_toolkit rotate scan.pdf --degrees -90 --pages 2,4-6 -o scan_girado.pdf
```

`--degrees` em múltiplos de 90 (aceita negativo). `--pages` restringe a
páginas específicas; sem ela, gira todas.

## extract-text

```bash
pdf_toolkit extract-text contrato.pdf -o contrato.txt
pdf_toolkit extract-text contrato.pdf --layout -o contrato.txt
```

`--layout` preserva colunas/espaçamento como texto simples (equivalente a
`pdfplumber.extract_tables` para tabelas simples).

## extract-images

```bash
pdf_toolkit extract-images laudo.pdf --output-dir laudo_imagens
```

Extrai as imagens embutidas via `pdfimages`.

## metadata

```bash
pdf_toolkit metadata documento.pdf
```

Exibe título, autor, nº de páginas, criptografia etc. via `pdfinfo`.

## compress

```bash
pdf_toolkit compress grande.pdf -o grande_comprimido.pdf
pdf_toolkit compress grande.pdf --quality screen -o grande_web.pdf
```

`--quality` aceita `screen`/`ebook`/`printer` (trade-off tamanho ×
qualidade de imagem, via `ghostscript`).

## encrypt / decrypt

```bash
pdf_toolkit encrypt documento.pdf --user-password 123 -o documento_protegido.pdf
pdf_toolkit encrypt documento.pdf --user-password 123 --no-print --no-copy -o documento_protegido.pdf
pdf_toolkit decrypt documento_protegido.pdf --password 123 -o documento.pdf
```

`encrypt` usa AES-256 (`qpdf --encrypt ... 256`); se só `--user-password`
for informado, a senha de dono usa o mesmo valor. `--no-print`/`--no-copy`
restringem permissões do PDF resultante.

## watermark

```bash
pdf_toolkit watermark contrato.pdf --stamp confidencial.pdf -o contrato_marcado.pdf
```

Por padrão sobrepõe (`--overlay`); use `--underlay` para desenhar atrás do
conteúdo original. O PDF de carimbo é repetido em todas as páginas de
saída (`--repeat=1-z`).

## Comportamentos comuns a todos os comandos

- **Não-destrutivo por construção**: cada comando lê a entrada e escreve
  uma saída nova; o arquivo original nunca é sobrescrito por padrão.
- Se uma ferramenta externa faltar, o programa informa exatamente qual e
  como instalar antes de abortar — nenhum subcomando falha silenciosamente.
