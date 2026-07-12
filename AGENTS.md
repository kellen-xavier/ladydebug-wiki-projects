# AGENTS.md

## Objetivo do repositório

Este repositório centraliza documentação, padrões, regras e base de conhecimento do Projeto X.

## Regra principal

Toda documentação do Projeto X deve seguir o padrão localizado em:

- `docs/governanca/padroes/projeto-x.md`
- `.agents/references/projeto-x/documentacao-padrao.md`
- `.agents/references/projeto-x/regras-obrigatorias.md`
- `.agents/references/projeto-x/checklist-revisao.md`

## Fluxo obrigatório para revisão documental

Ao revisar, criar ou reorganizar documentos do Projeto X:

1. Identificar o tipo do documento.
2. Consultar o padrão oficial do Projeto X.
3. Aplicar a skill `revisar-documentacao-projeto-x` quando a tarefa envolver documentação.
4. Preservar o conteúdo original em `knowledge/raw/`.
5. Gerar versão intermediária em `knowledge/extracted/`, se houver extração.
6. Produzir versão revisada em `knowledge/curated/` ou diretamente em `docs/`, conforme maturidade.
7. Marcar lacunas como `pendente-validacao`.
8. Não publicar conteúdo sem fonte.

## Critérios mínimos de aceite

Uma documentação só pode ser considerada revisada quando possuir:

- objetivo;
- contexto;
- público-alvo;
- fonte;
- status;
- data de revisão;
- rastreabilidade com documento original;
- pendências, quando existirem.
