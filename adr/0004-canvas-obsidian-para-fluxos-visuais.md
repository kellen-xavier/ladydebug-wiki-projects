---
title: "ADR 0003 - Canvas do Obsidian para fluxos visuais"
version: "1.0.0"
status: "proposto"
owner: "time"
updated: "2026-07-31"
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
  - canvas
  - diagrama
  - versionamento
---

## ADR 0003 - Canvas do Obsidian para fluxos visuais

> Template de ADR. Os nomes de sistema (`sistema-a`, `sistema-b`, `sistema-c`...)
> sao exemplos; substitua pelos do seu projeto.

## Status

Proposto.

## Contexto

A documentação já tem diagramas em Mermaid (ver [ADR 0002 - diagramas](/adr/0002-diagramas-arquitetura-proposta.md)),
que renderizam na wiki mas **não são navegáveis**: o leitor vê a caixa "Fluxo X"
e ainda precisa procurar o arquivo correspondente na árvore.

O time já usa o **Canvas do Obsidian** para montar mapas onde cada bloco abre o
documento com um clique. O formato é o [JSON Canvas](https://jsoncanvas.org/),
especificação aberta, extensão `.canvas`, conteúdo JSON.

Trazer esses mapas para o repositório esbarra em três regras vigentes:

1. **Frontmatter obrigatório** ([CONTRIBUTING](/CONTRIBUTING.md)): todo documento
   declara `title`, `version`, `status`, `owner`, `updated`. JSON não comporta
   frontmatter YAML.
2. **Validação de links** ([ADR 0002 - organização](/adr/0002-organizacao-orientada-a-dominio.md)):
   `scripts/validar-links.js` só varria `.md`. Um canvas cujo propósito é
   *apontar para arquivos* passaria sem nenhuma verificação — o pior caso, já
   que ele quebra silenciosamente a cada arquivo movido.
3. **Versionamento**: o Obsidian regrava o `.canvas` inteiro a cada interação,
   com chaves em ordem variável. Sem tratamento, mover um bloco na tela produz
   diff do arquivo inteiro, o review fica inviável e dois autores no mesmo
   canvas conflitam sempre.

## Decisão

### 1. Localização

Canvas é **narrativa** — logo, segue a regra do ADR 0002 e mora no domínio do
sistema. O repositório já tem convenção para documento com anexos: **uma pasta
com o nome do documento**, como em `dominio/sistema-c/fluxos/fluxo-exemplo/`
(ficha `.md` + `img/`). O canvas usa a mesma convenção:

```text
dominio/{sistema}/fluxos/{nome-do-mapa}/
    {nome-do-mapa}.md        <- ficha (sidecar), renderiza na wiki
    {nome-do-mapa}.canvas    <- o mapa
```

O `.canvas` é tratado como **anexo da ficha**, exatamente como `img/` é anexo de
um fluxo. A pasta mantém juntos os arquivos que só fazem sentido em conjunto.

Foi descartada uma pasta `fluxos/canvas/` agrupando por extensão: separar por
**tipo de arquivo** dentro do domínio reintroduz o critério que o ADR 0002
removeu — o leitor voltaria a precisar saber o formato do arquivo antes de saber
onde procurar. A arquitetura permanece: nada sai de `dominio/`, nada volta para
a raiz.

### 2. O vault é a raiz do repositório

Os nós `type: "file"` do JSON Canvas guardam caminho **relativo à raiz do
vault**. Portanto o repositório deve ser aberto como vault no Obsidian.

É isso que faz o mesmo caminho valer para o Obsidian e para o validador do CI.
Vault apontando para outra pasta gera caminhos que não resolvem aqui.

### 3. Ficha sidecar obrigatória

Todo `.canvas` tem um `.md` de mesmo basename, na mesma pasta, que:

- carrega o **frontmatter obrigatório** — é onde vive a versão do artefato;
- reproduz os links do canvas em Markdown, porque **a wiki não renderiza
  `.canvas`** e o conteúdo **não pode ficar refém do Obsidian**;
- registra o histórico de versões.

### 4. Forma canônica

`scripts/normalizar-canvas.js` reescreve o canvas com nós e arestas ordenados
por `id`, chaves em ordem fixa, indentação de 2 espaços e LF. É o que torna o
diff legível e o merge resolvível.

```bash
npm run canvas:normalizar   # aplica a forma canônica
npm run canvas:verificar    # falha se algum canvas estiver fora dela
```

### 5. Validação de links estendida

`scripts/validar-links.js` passa a ler `.canvas`, extraindo os nós
`type: "file"` e resolvendo o caminho a partir da raiz. Vale a mesma **catraca
de baseline** do ADR 0002: só quebra **nova** reprova o PR.

JSON malformado vira uma quebra registrada, não exceção — um canvas corrompido
não derruba a validação do repositório inteiro.

## Versionamento

O canvas entra nas **duas camadas de versionamento que já existem**. Nada muda
para o resto do repositório.

### Camada 1 — repositório (automática)

Inalterada, conforme [ADR 0001](/adr/0001-versionamento-automatizado-pipeline.md):
o merge em `main` gera bump SemVer em manifesto e tag `vX.Y.Z` a partir da
mensagem de commit. Canvas é arquivo versionado como qualquer outro.

| Commit | Efeito no repositório |
| --- | --- |
| `docs: ajustar posicao dos blocos no mapa do sistema-a` | PATCH |
| `feat: adicionar mapa visual de fluxos do sistema-b` | MINOR |

### Camada 2 — documento (manual, no sidecar)

O campo `version` da ficha `.md` versiona **o mapa**, não o repositório. Como o
canvas não comporta frontmatter, a ficha é a fonte da verdade.

| Mudança no canvas | Bump | Exemplo |
| --- | --- | --- |
| Refazer o mapa; remover ou renomear nó citado por outro documento; mudar o recorte do que o mapa cobre | MAJOR | Dividir o mapa do sistema-a em dois |
| Acrescentar nós ou arestas; documentar caminho novo | MINOR | Incluir um ramo novo |
| Reposicionar, redimensionar, cor, texto; corrigir caminho de arquivo | PATCH | Mover blocos para caber na tela |

Regras de acompanhamento:

- Toda alteração no `.canvas` **exige** atualizar `version` e `updated` na ficha.
- Toda alteração de versão acrescenta linha na tabela **Histórico de versões**.
- Renomear um `.canvas` exige renomear a ficha junto — o pareamento é por basename.

## Consequências

### Positivas

- Navegação visual com links reais, validados pelo CI a cada PR.
- Diff de canvas legível; merge de canvas deixa de ser conflito garantido.
- A regra de frontmatter continua valendo para 100% da documentação.
- A arquitetura orientada a domínio permanece intacta.

### Negativas / custos

- Duplicação parcial: os links aparecem no canvas e na ficha. É o preço de a
  wiki não renderizar `.canvas`; a ficha é a versão acessível.
- O time precisa rodar `npm run canvas:normalizar` antes do commit. Mitigação:
  `npm run canvas:verificar` aponta o que falta.
- Exige abrir o repositório como vault. Vaults antigos com outra estrutura
  precisam ter os canvas reautorados com os caminhos deste repositório.

## Alternativas consideradas

| Alternativa | Por que não |
| --- | --- |
| Só Mermaid | Renderiza na wiki, mas não navega: não resolve o problema original |
| Canvas fora do repositório | Some do versionamento e do gate de links; volta a divergir da documentação |
| Metadados dentro do próprio `.canvas` | O Obsidian pode descartar chaves que não conhece ao salvar; a ficha é previsível |
| Gerar a ficha automaticamente do canvas | Possível depois; exige decidir descrição e ordem — por ora, escrita à mão |
