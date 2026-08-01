---
title: "ADR 0002 - Organizacao do repositorio orientada a dominio"
version: "1.0.0"
status: "proposto"
owner: "time"
updated: "2026-07-29"
systems:
  - SISTEMA-A
  - SISTEMA-A-LEGADO
  - SISTEMA-B
  - SISTEMA-C
audience:
  - analista
  - dev
  - qa
tags:
  - adr
  - organizacao
  - dominio
  - wiki
---

# ADR 0002 - Organizacao do repositorio orientada a dominio

> Template de ADR. Os nomes de sistema (`sistema-a`, `sistema-b`, `sistema-c`...)
> e os numeros de inventario sao exemplos ilustrativos. Substitua pelos do seu
> projeto.

## Status

Aceito. Fases 1 a 4 executadas. Fases 5 e 6 pendentes.

## Contexto

### O problema real

O repositorio nao esta desorganizado por descuido. Ele tem **tres criterios de
organizacao competindo no mesmo nivel**:

| Criterio | Pastas que o seguem |
| --- | --- |
| Por **sistema** | `sistema-a/`, `sistema-b/`, `sistema-c/` |
| Por **tipo de artefato** | `database/`, `integracoes/`, `fluxos/`, `scripts/` |
| Por **dominio** | `dominio/` (contem apenas 1 sistema) |

`sistema-b/` e `sistema-c/` nao estao "soltas" por acidente: elas seguem o
criterio *por sistema*, enquanto suas vizinhas seguem o criterio *por tipo de
artefato*. Nenhum dos dois criterios esta errado isoladamente. O erro e ter os
dois no mesmo nivel — o leitor nao consegue prever onde procurar.

### Inventario medido

| Pasta | Arquivos | Observacao |
| --- | --- | --- |
| `database/` | 146 | `queries/` por subsistema + `tabelas/` (dicionario) |
| `dominio/sistema-a/` | 111 | `modulo-1/`, `modulo-2/`, `fluxos/`, `cadastro/`, `area-x/`, `area-y/` |
| `integracoes/` | 66 | subpasta por sistema |
| `sistema-c/` | 56 | 2 `.md` + 54 imagens |
| `sistema-a/` (raiz) | 33 | `cadastro/`, `area-x/`, `area-y/` — **duplica** o de `dominio/` |
| `fluxos/` | 26 | subpasta de um sistema + 5 fluxos soltos, todos do mesmo sistema |
| `sistema-b/` | 22 | 19 `.sql` + 3 `.md`, tudo plano |

### O problema mais grave nao foi o relatado

`sistema-a` existe em **dois lugares, com as mesmas subpastas**:

| Subpasta | Na raiz | Em `dominio/` |
| --- | --- | --- |
| `cadastro/` | 2 | 2 |
| `area-x/` | 28 | 3 |
| `area-y/` | 3 | 11 |

A divisao e arbitraria — nao e "antigo vs novo". Enquanto isso existir, nenhum
leitor sabe qual copia e a canonica. Isso precede qualquer outra mudanca.

### Restricao que define o desenho

O arquivo `.order` na raiz e a convencao de **Wiki do Azure DevOps**. A arvore de
pastas *e* a navegacao da wiki, e a ordem das paginas vem do `.order` de cada
pasta. Existe apenas **um** `.order` no repositorio, logo hoje a navegacao e
alfabetica — o oposto de didatica.

O `SISTEMAS.md` ja define o vocabulario de dominio dos sistemas. O
`frontmatter.json` ja define a taxonomia `systems` com `SISTEMA-A`,
`SISTEMA-A-LEGADO`, `SISTEMA-B`, `SISTEMA-C`.

## Decisao

### Principio 1 - A arvore de pastas e a projecao do SISTEMAS.md

A estrutura de `dominio/` espelha os sistemas declarados em `SISTEMAS.md`. Quem
entende o negocio consegue prever o caminho do arquivo sem procurar.

### Principio 2 - Separar narrativa de referencia

Existem dois tipos de conteudo, com modos de leitura opostos:

| Tipo | Pergunta que responde | Como e lido | Onde mora |
| --- | --- | --- | --- |
| **Narrativa** | "como funciona o processo X" | em sequencia, por quem esta aprendendo | `dominio/<sistema>/` |
| **Referencia** | "o que e a tabela `tabela_a`" | por consulta pontual, de varios lugares | `database/`, `integracoes/` |

Referencia permanece compartilhada, por duas razoes concretas:

1. **Multiplos fluxos apontam para a mesma tabela.** Varios fluxos linkam
   `database/tabelas/tabela_b.md`. Mover tabelas para dentro de um dominio
   forcaria duplicacao ou caminhos relativos profundos e frageis.
2. **Integracao e, por definicao, entre sistemas.** Uma mensagem de integracao
   trafega `sistema-a` <-> `sistema-b`. Nao pertence a um dominio unico.

### Principio 3 - Fluxo pertence sempre a um sistema

Nao existe fluxo orfao. Todo fluxo e fluxo *de* um sistema. Manter `fluxos/` na
raiz obriga a perguntar "de qual sistema?" a cada arquivo.

Fluxos que cruzam sistemas moram no sistema que **orquestra** o fluxo, com link
para os demais — nao numa pasta neutra.

### Estrutura alvo

```text
/
├── README.md                 porta de entrada
├── GLOSSARIO.md              termos e siglas
├── SISTEMAS.md               mapa dos sistemas (fonte da taxonomia)
├── CONTRIBUTING.md
├── .order
│
├── dominio/                  NARRATIVA
│   ├── .order
│   ├── sistema-a/
│   │   ├── .order
│   │   ├── index.md
│   │   ├── fluxos/
│   │   ├── cadastro/
│   │   ├── modulo-1/
│   │   ├── modulo-2/
│   │   ├── area-x/
│   │   └── area-y/
│   ├── sistema-b/
│   │   ├── index.md
│   │   └── fluxos/
│   └── sistema-c/
│       ├── index.md
│       ├── fluxos/
│       └── img/
│
├── database/                 REFERENCIA - dicionario de dados
│   ├── tabelas/
│   └── queries/
│       ├── sistema-a/
│       ├── sistema-a-legado/
│       └── sistema-b/
│
├── integracoes/              REFERENCIA - contratos entre sistemas
├── adr/                      decisoes de arquitetura
└── scripts/                  automacao
```

### Destino de cada pasta problematica

| Hoje | Vai para | Por que |
| --- | --- | --- |
| `sistema-b/*.sql` (19) | `database/queries/sistema-b/` | Segue a convencao que ja existe em `database/queries/` |
| `sistema-b/doc-tabela-exemplo.md` | `database/tabelas/` | E documentacao de tabela, nao narrativa |
| `sistema-b/narrativa-a.md`, `narrativa-b.md` | `dominio/sistema-b/` | Narrativa de dominio |
| `sistema-b/payload-integracao.txt` | `integracoes/mensagem-exemplo-integracao.txt` | E payload de mensagem entre `sistema-a` e `sistema-b`, nao query |
| `fluxos/sistema-b/` | `dominio/sistema-b/fluxos/` | Fluxo pertence ao sistema |
| `sistema-c/` (inteira) | `dominio/sistema-c/` | E um sistema, irmao dos outros |
| `sistema-a/` (raiz) | `dominio/sistema-a/` | Elimina a duplicacao |
| `fluxos/*.md` (5 soltos) | `dominio/sistema-a/fluxos/` | Sao todos do mesmo sistema |
| `fluxos/` (vazia ao fim) | removida | Deixa de existir como pasta de topo |

## Consequencias

### Positivas

- Duas perguntas cobrem o repositorio inteiro: "de qual sistema?" e "narrativa
  ou referencia?". Previsivel para quem nao e tecnico.
- A navegacao da wiki passa a ter significado, com `.order` por pasta.
- Elimina a ambiguidade de qual `sistema-a` e o canonico.
- Reduz de 7 pastas de topo de conteudo para 4.

### Custos e riscos

- **Links internos quebram.** Toda a movimentacao precisa de varredura de links
  relativos. Esta e a maior fonte de retrabalho.
- **Historico de renomeacao.** Usar `git mv` preserva o rastreio; copiar e
  deletar nao.
- **PRs grandes.** A migracao deve ser fatiada para permitir revisao.

## Decisoes tomadas durante a execucao

Divergencias em relacao ao plano original, registradas com o motivo.

### Siglas em maiuscula

O plano escrevia nomes de sistema em minuscula, por simetria com nomes
descritivos. Siglas ficam em maiuscula por dois motivos:

1. Sao siglas, e e assim que o `SISTEMAS.md` as nomeia.
2. Na Wiki do Azure DevOps o nome da pasta vira titulo de pagina. Uma sigla que
   coincide com uma palavra comum seria exibida como essa palavra — ambiguo
   justamente para o leitor nao tecnico que esta documentacao quer atender.

Nomes descritivos (nao siglas) permanecem minusculos.

### Sistema sem narrativa nao ganha pasta vazia

Um sistema pode estar declarado no `SISTEMAS.md` mas nao possuir **narrativa**
propria — sua unica documentacao sao queries, que ficam em
`database/queries/<sistema>/` por serem referencia.

Como o git nao versiona pasta vazia, criar a pasta produziria divergencia entre
o disco de quem a criou e qualquer clone novo. A pasta nasce quando houver o
primeiro documento narrativo, junto de um `index.md`.

### Correcao de aninhamento

Durante a execucao, dois sistemas foram colocados dentro da pasta de um terceiro.
Isso afirmava uma relacao de pertencimento que o `SISTEMAS.md` nao declara.
Corrigido para irmaos. Nenhum conteudo foi perdido (contagem conferida antes e
depois).

## Divida tecnica descoberta: links internos quebrados

A migracao motivou a criacao de um validador de links. A medicao revelou um
problema que **precede** esta reorganizacao (numeros ilustrativos):

| Momento | Links internos quebrados |
| --- | --- |
| Ultimo commit antes da migracao | 144 em 36 arquivos |
| Depois da migracao, antes de corrigir | 149 em 38 arquivos |
| Depois de corrigir as regressoes | 143 em 35 arquivos |

A migracao introduziu 5 quebras, todas corrigidas. As outras **143 ja existiam**
e apontam em maioria para uma pasta que nao existe no layout atual — residuo de
reorganizacoes anteriores.

Isso reforca a Fase 6 e muda seu escopo: nao e "verificar se a migracao quebrou
algo", e sim "corrigir uma divida acumulada e impedir que volte".

O padrao adotado nas correcoes e **caminho raiz-absoluto** (`/dominio/...`), nao
relativo. Motivo: link relativo quebra quando o arquivo de origem se move, e este
repositorio move arquivos com frequencia. Foi exatamente essa a causa das 5
regressoes.

### Catraca em vez de gate rigido

`npm run lint:links` compara o estado atual com `scripts/links-baseline.json` e
**reprova apenas quebra nova**. Um gate rigido reprovaria toda PR enquanto as
quebras herdadas existissem, e gate sempre vermelho treina o time a ignora-lo.

| Comando | Uso |
| --- | --- |
| `npm run lint:links` | Valida. Roda na pipeline. Exit 1 se houver quebra nova |
| `npm run lint:links:baseline` | Regrava o baseline. Use ao corrigir divida herdada |

O baseline so deve **encolher**. Quando uma quebra herdada e corrigida, o script
avisa e pede a regravacao. A ultima fase termina quando `total` chegar a zero — a
partir dali o gate passa a ser rigido de fato, sem mudanca de codigo.

### A validacao e sensivel a maiusculas de proposito

O validador **nao** usa `fs.existsSync`, porque essa funcao acompanha a
plataforma: no Windows `glossario.md` resolve para `GLOSSARIO.md`, no Linux nao.
Ele compara o alvo com as entradas reais de cada diretorio do caminho.

Sem isso a validacao local no Windows aprovaria links que a pipeline
(`ubuntu-latest`) reprova — e a **Wiki do Azure DevOps tambem e sensivel a
maiusculas**, entao o link quebraria para o leitor.

## Plano de migracao

Cada fase e uma PR independente e verificavel.

| Fase | Escopo | Criterio de pronto | Situacao |
| --- | --- | --- | --- |
| 1 | Consolidar `sistema-a/` da raiz em `dominio/` | Nao existe mais `sistema-a/` na raiz; lint verde | Concluida |
| 2 | Mover `sistema-c/` para `dominio/sistema-c/` | `sistema-c/` nao existe na raiz | Concluida |
| 3 | Dividir `sistema-b/` (sql, tabela, narrativa, payload) | Pasta removida; `database/queries/sistema-b/` criada | Concluida |
| 4 | Mover `fluxos/sistema-b/` e os 5 fluxos soltos | `fluxos/` deixa de existir | Concluida |
| 5 | Criar `index.md` e `.order` em cada pasta de `dominio/` | Navegacao da wiki na ordem didatica | Pendente |
| 6 | Validador de links no CI, com catraca sobre a divida | `npm run lint:links` reprova quebra nova | Concluida |
| 7 | Reduzir as quebras herdadas do baseline | `total` do baseline em 0 | Pendente |

Resultado das fases 1 a 4: as pastas de topo cairam de 9 para 5 (`adr/`,
`database/`, `dominio/`, `integracoes/`, `scripts/`), e `dominio/` passou a conter
`sistema-a/`, `sistema-b/` e `sistema-c/` como irmaos.

Regras para todas as fases:

- Usar `git mv`, nunca copiar e deletar.
- Rodar `npm run lint:md` e `npm test` antes de abrir a PR.
- Uma fase por PR.

## Pendencias em aberto

Itens que precisam de decisao antes das fases correspondentes:

1. **Nomes que misturam criterios.** Um nome de pasta que mistura, por exemplo,
   numero de instancia com nome de sistema fica enganoso quando a documentacao
   cresce. Renomear tem custo alto de links.
2. **Subpastas de `integracoes/` nomeadas por instancia**, e nao por sistema.
3. **Arquivos `.md` vazios** (0 bytes): preencher ou remover.
4. **Nomenclatura dos `.sql`.** Nomes com espacos ou sufixos como `(1)` nao
   seguem padrao. Renomear junto da fase correspondente.
5. **Arquivos `.canvas`.** O formato e JSON do Obsidian e **nao renderiza na Wiki
   do Azure DevOps**. Serve como artefato de autoria, nao de publicacao. Definir
   se entram versionados (e onde) ou ficam fora. Nao afetam o lint, que so
   processa `**/*.md`.
