# PDF escaneado (sem camada de texto)

Referência de apoio ao Passo 1 (`inventory`) da
[`SKILL.md`](../SKILL.md#passo-1--inventory). Não linka outro arquivo em
`reference/` — para os outros dois pontos de apoio, volte à SKILL.md.

## Como identificar

`pdf_toolkit inventory arquivo.pdf` roda `pdffonts` e conclui "sem fontes
embutidas" quando não há nenhuma linha de fonte na saída — sinal de que as
páginas são imagem (scan), não texto vetorial. `pdftotext`/`read-large`
sobre um PDF assim produz blocos vazios ou com poucas linhas de ruído
(cabeçalho/rodapé que porventura seja texto real).

## O que fazer

1. **Rasterizar** as páginas relevantes:

   ```bash
   pdf_toolkit rasterize arquivo.pdf --dpi 300 --format png
   pdf_toolkit rasterize arquivo.pdf -f 10 -l 25 --dpi 300   # so um intervalo
   ```

   `rasterize` imprime os nomes de arquivo gerados — não adivinhe o
   zero-padding do `pdftoppm`, ele varia com o total de páginas
   processadas.

2. **OCR**: este toolkit não inclui OCR (ver limitação equivalente na
   skill `pdf-toolkit`). As imagens geradas por `rasterize` podem ser
   passadas para uma ferramenta de OCR externa (`tesseract`, ou leitura
   direta da imagem por um agente com visão) — combine com o usuário qual
   caminho usar antes de prosseguir; não presuma.

3. Depois do OCR, o texto resultante ainda deve levar a âncora de página
   (`<!-- pagina: N -->`) se for reincorporado a um fluxo de leitura em
   blocos, para manter a rastreabilidade página → trecho.

## DPI

300 DPI é um ponto de partida razoável para OCR de texto datilografado ou
impresso comum. Para documentos com letra pequena (tabelas densas, notas de
rodapé), considere DPI mais alto; isso aumenta proporcionalmente o tamanho
dos arquivos gerados.
