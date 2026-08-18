# Estratégia de leitura por tipo de documento

Referência de apoio ao Passo 1 (`inventory`) da
[`SKILL.md`](../SKILL.md#passo-1--inventory). Não linka outro arquivo em
`reference/` — para os outros dois pontos de apoio, volte à SKILL.md.

## Tipo de documento → método

| Situação (veredito do `inventory`) | Método |
|---|---|
| Texto extraível, PDF pequeno/médio (< `--min-pages`) | `pdf_toolkit extract-text --layout`, leitura direta |
| Texto extraível, PDF muito grande (>= `--min-pages`) | `pdf_toolkit read-large`, leitura em blocos |
| Sem fontes embutidas (escaneado) | Ver `pdf-escaneado.md` — rasterizar + OCR antes de qualquer coisa |
| PDF com outline/marcadores nativos | Ler `outline.json` primeiro; ele já dá a estrutura de tópicos |
| PDF sem outline nativo | A estrutura de tópicos precisa ser inferida lendo os blocos em ordem |

## Orçamento de tokens

- Cada bloco de `chunks/` cobre `--chunk-pages` páginas (padrão 50). Antes
  de ler todos os blocos de um PDF muito grande, estime: páginas totais ÷
  páginas por bloco = número de blocos a ler.
- Não guarde o texto bruto de blocos já lidos — mantenha só a lista
  corrente de tópicos (título + intervalo de páginas + 1-2 frases), como
  descrito no Passo 4 da SKILL.md. Ler tudo de novo no fim para "confirmar"
  desperdiça o orçamento que a divisão em blocos existe para preservar.
- Se já se sabe o termo/cláusula/data procurada, prefira `rg` sobre os
  `chunks/*.md` a ler os blocos em ordem — a âncora `<!-- pagina: N -->`
  entrega a página do hit sem precisar abrir o bloco inteiro.
- Para PDFs perto do limiar (`--min-pages`), considere aumentar
  `--chunk-pages` para reduzir o número de blocos, se o conteúdo por página
  for esparso (ex.: autos com muitas páginas de assinatura/carimbo).
