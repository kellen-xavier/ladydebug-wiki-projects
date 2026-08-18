# Solução de problemas — leitura de PDF extenso

Referência de apoio à [`SKILL.md`](../SKILL.md). Não linka outro arquivo em
`reference/` — para os outros dois pontos de apoio, volte à SKILL.md.

## Texto truncado ou ausente num bloco

- Confirme primeiro o veredito do `inventory` (ver `pdf-escaneado.md`): um
  bloco "vazio" costuma ser página escaneada, não bug do `read-large`.
- Se só algumas páginas do bloco vieram vazias, o PDF pode misturar páginas
  de texto com páginas de imagem (comum em autos digitalizados por partes).
  Rode `pdf_toolkit inventory` de novo restringindo mentalmente ao
  intervalo problemático, ou rasterize só esse intervalo com
  `rasterize -f -l` para inspecionar visualmente.
- `pdftotext -layout` (usado pelo `read-large`) preserva colunas/tabelas
  como espaçamento; texto que parece "embaralhado" às vezes é uma tabela
  larga — não é erro de extração.

## Fontes não embutidas

`pdffonts` marcando uma fonte como não-embutida não impede a extração de
texto (o glifo ainda mapeia para um caractere Unicode na maioria dos
casos), mas é um sinal de risco de mojibake (próxima seção) — a fonte pode
usar uma codificação customizada sem tabela Unicode correta.

## Mojibake (caracteres corrompidos)

- Sintoma: acentos/cedilha viram símbolos (`Ã§`, `Ã£`, `�`) nos blocos
  `.md` gerados.
- Causa mais comum: fonte com codificação customizada (ver seção
  anterior) ou PDF gerado por scanner/OCR de baixa qualidade que já
  embutiu texto corrompido no PDF original — nesse caso o problema está
  na fonte de dados, não no `pdftotext`.
- `read-large` já roda `pdftotext -layout`, que lida bem com UTF-8 padrão;
  não há flag de codificação alternativa a tentar no toolkit atual. Se o
  mojibake for sistemático em todo o documento, trate como PDF sem camada
  de texto confiável e siga `pdf-escaneado.md` (rasterizar + OCR).

## `read-large` recusou rodar

Mensagem `read-large e exclusivo para PDFs muito grandes (>= N paginas)`:
o PDF tem menos páginas que `--min-pages` (padrão 1000). Use
`pdf_toolkit extract-text --layout` diretamente, ou baixe o limiar com
`--min-pages` se o caso realmente pedir leitura em blocos mesmo abaixo de
1000 páginas.

## `qpdf --show-npages` ou `pdftotext` falham

Geralmente PDF corrompido, protegido por senha, ou com estrutura inválida.
Rode `pdf_toolkit metadata arquivo.pdf` (skill `pdf-toolkit`) para
confirmar se o arquivo abre normalmente antes de insistir em `read-large`.
