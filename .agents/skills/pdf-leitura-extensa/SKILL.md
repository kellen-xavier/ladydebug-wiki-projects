---
name: pdf-leitura-extensa
description: >
  Le e resume PDFs muito extensos (1.000+ paginas) que nao cabem em uma
  unica leitura de contexto, dividindo o texto em blocos por pagina com
  ancora de pagina, montando um sumario por topicos e escrevendo o
  resultado final. Use esta skill sempre que o usuario pedir para ler,
  resumir, mapear ou "achar onde fala de X" num PDF muito grande (autos
  processuais completos, tomos, processos com milhares de paginas). Nao
  cobre mesclar, dividir, girar, comprimir ou proteger PDF — isso e a
  skill `pdf-toolkit`.
---

# Leitura de PDFs Extensos

## Índice

- [Visão Geral](#visão-geral)
- [100% local, e o que consome contexto](#100-local-e-o-que-consome-contexto)
- [Pipeline de pastas](#pipeline-de-pastas)
- [Fluxo obrigatório](#fluxo-obrigatório)
- [Passo 1 — inventory](#passo-1--inventory)
- [Passo 2 — read-large](#passo-2--read-large)
- [Passo 3 — ler manifest.md e outline.json](#passo-3--ler-manifestmd-e-outlinejson)
- [Passo 4 — buscar e ler os blocos](#passo-4--buscar-e-ler-os-blocos)
- [Passo 5 — escrever a extração estruturada](#passo-5--escrever-a-extração-estruturada)
- [Passo 6 — curadoria](#passo-6--curadoria)
- [Referências](#referências)

## Visão Geral

Para PDFs com milhares de páginas, o texto completo não cabe em uma única
leitura de contexto — e não deve ser lido de uma vez de propósito. Esta
skill resolve a parte mecânica — dividir em blocos, extrair o outline
nativo, decidir se o PDF precisa de OCR — via subcomandos do binário
`pdf_toolkit` (mesmo binário da skill `pdf-toolkit`, que cobre a
*manipulação* de PDF; aqui é só *leitura*). Montar os tópicos, extrair
fatos e escrever o resumo é trabalho de compreensão de conteúdo, feito por
quem executa a skill ao ler os blocos — não pelo script.

Build do binário (uma vez, se ainda não existir):

```bash
cargo build --release
# binário em: target/release/pdf_toolkit
```

## 100% local, e o que consome contexto

Todo o binário `pdf_toolkit` roda **só na sua máquina**: é um executável
Rust compilado localmente, chamando ferramentas de linha de comando já
instaladas (`qpdf`, `poppler-utils`) — nenhuma chamada de rede, nenhum
serviço externo, nem o PDF nem o texto extraído saem da máquina.

Dentro do fluxo, nem todo passo tem o mesmo custo de contexto:

| Passo | O que faz | Consome contexto do agente? |
|---|---|---|
| 1–2 (`inventory`, `read-large`) | Script lê o PDF e escreve arquivos no disco | **Não** — o agente nunca vê o PDF nem o texto bruto nesses dois passos |
| 3 (`manifest.md`/`outline.json`) | Agente lê um índice pequeno | Pouco — poucas linhas |
| 4 (buscar/ler os blocos) | Agente busca ou lê os `.md` gerados | **Sim** — é aqui que o PDF "entra" no contexto, por isso a busca (`rg`) vem antes da leitura sequencial |
| 5–6 (extração, curadoria) | Agente escreve fatos com citação de página | Sim, mas pequeno — texto estruturado, não o PDF inteiro de novo |

Ou seja: **não peça para eu "ler o PDF"** — peça para rodar `inventory` +
`read-large` primeiro (isso não toca o contexto), e só depois me diga o
que procurar, para eu buscar com `rg` em vez de ler tudo em sequência.

## Pipeline de pastas

Convenção sugerida (adapte os nomes à estrutura do seu projeto — o que
importa é a separação entre bruto, derivado, extraído e curado):

| Pasta | Conteúdo | Quem escreve | Regra |
|---|---|---|---|
| `arquivos-pdfs-raw/` | PDFs originais (autos, extratos, tomos) | já existe antes desta skill rodar | nunca abrir o PDF bruto diretamente no contexto — passe sempre pelo pipeline abaixo |
| `arquivos-texto/` | Blocos `.md` com âncora de página, saída do `read-large --to` | script (`pdf_toolkit`), mecânico | derivado — se o projeto versiona texto extraído, avalie manter fora do controle de versão |
| `arquivos-extracted/` | Fatos extraídos dos blocos, **com página/citação** — Passo 5 | agente, lendo `arquivos-texto/` | sem hipótese: só o que está no texto, com referência |
| `arquivos-curated/` | Versão revisada/aprovada da extração — Passo 6 | agente + revisão humana | só o que está aqui pode alimentar um documento final |

## Fluxo obrigatório

1. `inventory` — decide a estratégia (texto extraível vs. escaneado).
2. `read-large --to arquivos-texto/` — gera os blocos `.md` com âncora de
   página, só se o PDF for realmente grande. Mecânico, sem custo de
   contexto.
3. Ler `manifest.md` e `outline.json` primeiro.
4. Buscar com `rg` (preferencial) ou ler os blocos em ordem (só se não
   souber o que procurar).
5. Escrever a extração estruturada, com página, em `arquivos-extracted/`.
6. Depois de revisada, mover para `arquivos-curated/` — daí em diante,
   aplicar o conteúdo curado num documento final é trabalho de outra
   etapa do seu projeto (esta skill cobre só leitura/extração).

## Passo 1 — inventory

```bash
pdf_toolkit inventory arquivo.pdf
```

Roda `pdfinfo` + `pdffonts` + `pdfimages -list` + `pdfdetach -list` e
imprime um veredito:

- **Camada de texto presente** → siga para o Passo 2 (`read-large`), ou
  direto para `pdf_toolkit extract-text` se o PDF tiver menos de
  `--min-pages`.
- **Sem fontes embutidas (PDF escaneado)** → texto não é extraível; veja
  [`reference/pdf-escaneado.md`](reference/pdf-escaneado.md) antes de
  prosseguir (rasterizar + OCR).

Mais critérios de decisão (orçamento de tokens, quando vale a pena dividir
por outline vs. por página fixa) em
[`reference/estrategia-leitura.md`](reference/estrategia-leitura.md).

## Passo 2 — read-large

```bash
pdf_toolkit read-large "arquivos-pdfs-raw/processo.pdf" --to "arquivos-texto/"
```

Só executa acima do limiar de páginas (`--min-pages`, padrão 1000) — é
deliberadamente exclusivo para PDFs muito grandes. `--to <dir>` é o modo
**recomendado** para trabalho de caso: escreve `manifest.md`,
`outline.json` e os blocos direto na pasta `arquivos-texto/` (sem
subpasta `chunks/`), casando com o [Pipeline de pastas](#pipeline-de-pastas)
acima — busque sempre os `.md` gerados ali via `rg`, nunca o PDF bruto.
Sem `--to`, usa `--output-dir` (padrão `<nome>_leitura/`, com subpasta
`chunks/`) — só para uso ad hoc, fora de uma estrutura de pastas fixa. As
duas opções são mutuamente exclusivas. Gera:

- `manifest.md` — índice dos blocos, na ordem de leitura, com o intervalo
  de páginas de cada um;
- `outline.json` — marcadores/sumário nativo do PDF, se existir (saída
  bruta de `qpdf --json --json-key=outlines`, para ler diretamente);
- blocos `<nome>_pNNNNN-NNNNN.md` — texto de cada intervalo de páginas
  (via `pdftotext -layout`), com uma âncora `<!-- pagina: N -->` antes do
  texto de cada página.

```bash
pdf_toolkit read-large "arquivos-pdfs-raw/processo.pdf" --to "arquivos-texto/" --chunk-pages 30   # blocos menores
pdf_toolkit read-large "arquivos-pdfs-raw/processo.pdf" --to "arquivos-texto/" --min-pages 800    # ajusta o limiar
```

Este passo é 100% mecânico — nenhum texto do PDF passa pelo contexto do
agente aqui (ver [100% local, e o que consome contexto](#100-local-e-o-que-consome-contexto)).

## Passo 3 — ler manifest.md e outline.json

Se o PDF já tem marcadores nativos, eles dão a estrutura de tópicos de
graça — leia `outline.json` antes de abrir qualquer bloco de texto.

## Passo 4 — buscar e ler os blocos

É aqui que o conteúdo do PDF passa a ocupar contexto — por isso, nesta
ordem de preferência:

1. **Se já se sabe o que procurar** (nome de parte, cláusula, valor,
   data): busque direto com `rg`, sem abrir os blocos em ordem. Com as
   âncoras de página, um `rg` puro já entrega o número da página junto do
   trecho:

   ```bash
   rg -n -B1 "termo de busca" arquivos-texto/*.md   # -B1 pega a ancora "<!-- pagina: N -->" acima
   ```

2. **Só se não se sabe o que procurar** (primeira passada de
   reconhecimento): leia os blocos em ordem, um de cada vez. Como o
   conteúdo total não cabe numa janela só, mantenha durante a leitura uma
   lista corrente e compacta de tópicos/subtópicos (título + intervalo de
   páginas + 1-2 frases), sem guardar o texto bruto de blocos já
   processados.

Para um PDF de milhares de páginas, prefira sempre (1): várias buscas
`rg` direcionadas custam muito menos contexto do que uma leitura
sequencial completa, mesmo em blocos.

Casos de texto truncado, fontes não embutidas ou mojibake:
[`reference/solucao-problemas.md`](reference/solucao-problemas.md).

## Passo 5 — escrever a extração estruturada

Depois de buscar/ler o necessário, escreva os fatos encontrados em
`arquivos-extracted/EXTRACAO-<nome-do-pdf>.md` (ver
[Pipeline de pastas](#pipeline-de-pastas)), com pelo menos:

- título, arquivo de origem (`arquivos-pdfs-raw/...`), total de páginas;
- um **sumário** dos tópicos encontrados, com o intervalo de páginas de
  cada um;
- uma **seção por tópico/fato**, cada uma citando a página de origem
  (`<!-- pagina: N -->` do bloco correspondente) — sem página citada, o
  fato não entra.

Regra desta skill: **sem hipótese**. Só escreva aqui o que está
literalmente no texto extraído; se um trecho estiver ambíguo, truncado ou
ilegível, marque como pendente (ver `reference/solucao-problemas.md`) em
vez de completar por dedução.

## Passo 6 — curadoria

`arquivos-extracted/` é o que o agente escreveu ao ler; ainda precisa de
revisão antes de virar conteúdo de um documento entregável. Depois de
revisado (por você ou numa segunda passada), mova/copie o conteúdo
aprovado para `arquivos-curated/`.

**Esta skill para aqui.** Aplicar o conteúdo de `arquivos-curated/` num
documento final (relatório, parecer, `.docx`, wiki etc.) é trabalho de
outra skill/pipeline específico do seu projeto — esta skill cobre só
leitura e extração de PDFs extensos.

## Referências

- [`reference/estrategia-leitura.md`](reference/estrategia-leitura.md) —
  tipo de documento → método de leitura; orçamento de tokens.
- [`reference/pdf-escaneado.md`](reference/pdf-escaneado.md) — PDF sem
  camada de texto: rasterizar com `pdf_toolkit rasterize`, OCR externo.
- [`reference/solucao-problemas.md`](reference/solucao-problemas.md) —
  texto truncado, fontes não embutidas, mojibake.
